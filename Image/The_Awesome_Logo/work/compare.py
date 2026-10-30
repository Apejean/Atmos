import sys, os, json, io
sys.path.insert(0,'work')
import numpy as np, cv2
from PIL import Image
import resvg_py, fitz

ROOT='The_Awesome_Logo_Final'
ref=cv2.imread('work/dark_panel.png')[:,:,::-1].copy()   # RGB 600x460 reference (dark panel of the board)
BG=(15,15,15)
svg=open(os.path.join(ROOT,'02_SVG','Full_Logo_Dark.svg'),encoding='utf-8').read()
# panel px -> viewBox: panel (0,0,600,460) == big (-640,-340,2400,1840)
svg=svg.replace('viewBox="-453 -107 2119 1525" width="2119" height="1525"','viewBox="-640 -340 2400 1840" width="2400" height="1840"')
svg=svg.replace('</title>','</title>\n<rect x="-640" y="-340" width="2400" height="1840" fill="#0f0f0f"/>',1)
def rend(width):
    return np.array(Image.open(io.BytesIO(bytes(resvg_py.svg_to_bytes(svg_string=svg,width=width)))).convert('RGB'))
vec=rend(600)
mask_label=np.ones(ref.shape[:2],bool); mask_label[:70,:200]=False     # board caption text, not part of the logo

def save(name,arr): Image.fromarray(arr.astype(np.uint8)).save(os.path.join(ROOT,'06_Comparison',name))
save('Original.png',ref); save('Vector_Render.png',vec)
save('Overlay_Comparison.png',0.5*ref.astype(float)+0.5*vec.astype(float))
diff=np.abs(ref.astype(float)-vec.astype(float)).mean(2)
save('Difference_Map.png',np.clip(diff*4,0,255))
# colored overlay: ref=red, vector=cyan (where they agree it is grey/white)
rl=ref.mean(2); vl=vec.mean(2)
col=np.stack([rl,vl,vl],2); save('Overlay_RedCyan.png',col)

# outline comparison at 3x : contours of luminance threshold
def contours(img,thr):
    g=cv2.cvtColor(img,cv2.COLOR_RGB2GRAY)
    g=cv2.resize(g,None,fx=3,fy=3,interpolation=cv2.INTER_CUBIC)
    m=(g>thr).astype(np.uint8)*255
    return m
out=np.zeros((460*3,600*3,3),np.uint8)
mr=contours(ref,30); mv=contours(vec,30)
cr,_=cv2.findContours(mr,cv2.RETR_LIST,cv2.CHAIN_APPROX_NONE); cv_,_=cv2.findContours(mv,cv2.RETR_LIST,cv2.CHAIN_APPROX_NONE)
cv2.drawContours(out,cr,-1,(255,60,60),1); cv2.drawContours(out,cv_,-1,(60,255,255),1)
save('Outline_Comparison.png',out)
# 4x zoom of the symbol, 3 panels side by side (reference | vector | diff)
x0,y0,x1,y1=160,85,450,310
def z(a): return cv2.resize(a[y0:y1,x0:x1],None,fx=3,fy=3,interpolation=cv2.INTER_CUBIC)
refz=cv2.resize(ref[y0:y1,x0:x1],None,fx=3,fy=3,interpolation=cv2.INTER_CUBIC)
vecz=rend(600*3)[y0*3:y1*3,x0*3:x1*3]
dz=np.clip(np.abs(refz.astype(float)-vecz.astype(float)).mean(2)*4,0,255)
save('Symbol_Zoom_3up.png',np.concatenate([refz,vecz,np.stack([dz]*3,2)],1))

# ---- metrics ----
M={}
sy=(slice(90,305),slice(165,445)); wh=(slice(335,375),slice(75,550)); tg=(slice(385,408),slice(120,505))
def iou(a,b): return float((a&b).sum()/max((a|b).sum(),1))
def region(name,sl,thr):
    r=(ref.mean(2)>thr)[sl]; v=(vec.mean(2)>thr)[sl]
    d=diff[sl]; un=(r|v)
    return {'IoU_threshold_%d'%thr:round(iou(r,v),4),'mean_abs_diff_all':round(float(d.mean()),2),'mean_abs_diff_inside_union':round(float(d[un].mean()),2)}
M['symbol']=region('sym',sy,60); M['wordmark']=region('wm',wh,128); M['tagline']=region('tg',tg,128)
# silhouette contour distance (symbol): symmetric chamfer in panel px at 3x
def chamfer(a,b):
    ka,_=cv2.findContours(a,cv2.RETR_EXTERNAL,cv2.CHAIN_APPROX_NONE); kb,_=cv2.findContours(b,cv2.RETR_EXTERNAL,cv2.CHAIN_APPROX_NONE)
    A=np.vstack([k[:,0,:] for k in ka]).astype(float); B=np.vstack([k[:,0,:] for k in kb]).astype(float)
    from scipy.spatial import cKDTree
    d1=cKDTree(B).query(A)[0]/3; d2=cKDTree(A).query(B)[0]/3
    return {'mean_px':round(float((d1.mean()+d2.mean())/2),3),'p95_px':round(float(np.percentile(np.r_[d1,d2],95)),3),'max_px':round(float(max(d1.max(),d2.max())),3)}
sl3=(slice(90*3,305*3),slice(165*3,445*3))
M['symbol_outline_distance']=chamfer(mr[sl3],mv[sl3])
sl3=(slice(335*3,375*3),slice(75*3,550*3)); M['wordmark_outline_distance']=chamfer(mr[sl3],mv[sl3])
# per-face luminance error (symbol) via masks from the vector render order is not available here -> quadrant summary
H,W=diff.shape
M['symbol_quadrant_mean_abs_diff']={}
cx,cy=(165+445)//2,(90+305)//2
for n,(ys,xs) in {'top_left':(slice(90,cy),slice(165,cx)),'top_right':(slice(90,cy),slice(cx,445)),'bottom_left':(slice(cy,305),slice(165,cx)),'bottom_right':(slice(cy,305),slice(cx,445))}.items():
    M['symbol_quadrant_mean_abs_diff'][n]=round(float(diff[ys,xs].mean()),2)
M['note']='Reference background (~RGB 15) was reproduced with a flat #0f0f0f rect only for this comparison; the delivered logos are transparent.'
json.dump(M,open('work/metrics.json','w'),indent=1,ensure_ascii=False)
print(json.dumps(M,indent=1))
