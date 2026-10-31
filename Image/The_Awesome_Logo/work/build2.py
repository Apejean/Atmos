import sys, os, io, json, shutil
sys.path.insert(0,'work')
import numpy as np, cv2
from PIL import Image, ImageDraw
import resvg_py, fitz
import pipe, logo, pdfw, goldbuild
import eps as epsmod
from pipe import FW,FH,ORDER
from scipy.spatial import cKDTree

ROOT='The_Awesome_Logo_Final'
PHI=(1+5**.5)/2
if os.path.exists(ROOT): shutil.rmtree(ROOT)
for d in ('01_Original_Reference','02_SVG','03_PNG','04_App_Icons','05_Illustrator','06_Comparison','07_Documentation'):
    os.makedirs(os.path.join(ROOT,d))
def P(*a): return os.path.join(ROOT,*a)

V={}
for nm,col in (('dark','#ffffff'),('light','#000000')):
    v,G,faces,fits,wm=goldbuild.run(nm)
    allp=np.vstack([faces[k].poly() for k in ORDER])
    sb=(allp.min(0),allp.max(0))
    hb=np.array(wm['head']['bbox']); tb=np.array(wm['tag']['bbox'])
    tx0=min(hb[:,0,0].min(),tb[:,0,0].min()); tx1=max(hb[:,1,0].max(),tb[:,1,0].max()); ty1=tb[:,1,1].max(); ty0=hb[:,0,1].min()
    x0=min(sb[0][0],tx0); x1=max(sb[1][0],tx1); y0=sb[0][1]; y1=ty1
    M=G['clear']                       # clear space = symbol height / phi^3
    V[nm]=dict(v=v,G=G,faces=faces,fits=fits,wm=wm,col=col,symbb=sb,full=(x0-M,y0-M,x1-x0+2*M,y1-y0+2*M),
               wmvb=(tx0-60,ty0-60,tx1-tx0+120,ty1-ty0+120),symvb=(sb[0][0]-M/PHI,sb[0][1]-M/PHI,sb[1][0]-sb[0][0]+2*M/PHI,sb[1][1]-sb[0][1]+2*M/PHI))
    V[nm]['defs'],V[nm]['sbody']=logo.sym_defs_and_body(faces,fits)

def wmbody(nm,color):
    logo.WM={'head':{'paths':V[nm]['wm']['head']['paths']},'tag':{'paths':V[nm]['wm']['tag']['paths']}}
    return logo.wm_body(color)
def r(vb): return tuple(round(float(x),1) for x in vb)
DARK_BG='#000000'; LIGHT_BG='#ffffff'
def full_svg(nm,color,title,bg=None):
    d=V[nm]; return logo.wrap(r(d['full']),d['defs'],'<g id="symbol">\n'+d['sbody']+'\n</g>\n'+wmbody(nm,color),title=title,bg=bg)
def sym_svg(nm,bg=None): d=V[nm]; return logo.wrap(r(d['symvb']),d['defs'],'<g id="symbol">\n'+d['sbody']+'\n</g>',title='The Awesome - Symbol',bg=bg)
def wm_svg(nm,color,bg=None): d=V[nm]; return logo.wrap(r(d['wmvb']),'',wmbody(nm,color),title='The Awesome - Wordmark',bg=bg)

# Dark version = dark-tuned metal + white lettering ON A BLACK BACKGROUND; Light version = light-tuned metal + black lettering
# ON A WHITE BACKGROUND. *_Transparent files are the same artwork without the background, for placing on your own colour.
S={'Full_Logo_Dark':full_svg('dark','#ffffff','The Awesome - Full Logo (dark version)',DARK_BG),
   'Full_Logo_Dark_Transparent':full_svg('dark','#ffffff','The Awesome - Full Logo (dark version, no background)'),
   'Full_Logo_Light':full_svg('light','#000000','The Awesome - Full Logo (light version)',LIGHT_BG),
   'Full_Logo_Light_Transparent':full_svg('light','#000000','The Awesome - Full Logo (light version, no background)'),
   'Symbol_Dark':sym_svg('dark',DARK_BG),'Symbol_Dark_Transparent':sym_svg('dark'),
   'Symbol_Light':sym_svg('light',LIGHT_BG),'Symbol_Light_Transparent':sym_svg('light'),
   'Wordmark_Dark':wm_svg('dark','#ffffff',DARK_BG),'Wordmark_Dark_Transparent':wm_svg('dark','#ffffff'),
   'Wordmark_Light':wm_svg('light','#000000',LIGHT_BG),'Wordmark_Light_Transparent':wm_svg('light','#000000')}
for k,vv in S.items(): open(P('02_SVG',k+'.svg'),'w',encoding='utf-8').write(vv)

def rend(svg,width=None,bg=None):
    kw={}
    if width: kw['width']=width
    if bg: kw['background']=bg
    return Image.open(io.BytesIO(bytes(resvg_py.svg_to_bytes(svg_string=svg,**kw)))).convert('RGBA')

# PNG
def square(svg,bg,fn,size=1024):
    s=rend(svg,width=int(size*0.8)); cv=Image.new('RGBA',(size,size),bg); cv.alpha_composite(s,((size-s.size[0])//2,(size-s.size[1])//2)); cv.save(P('03_PNG',fn))
square(S['Symbol_Dark_Transparent'],(0,0,0,255),'Symbol_Dark_1024.png')
square(S['Symbol_Dark_Transparent'],(0,0,0,0),'Symbol_Dark_Transparent_1024.png')
square(S['Symbol_Light_Transparent'],(255,255,255,255),'Symbol_Light_1024.png')
square(S['Symbol_Light_Transparent'],(0,0,0,0),'Symbol_Light_Transparent_1024.png')
for k in ('Full_Logo_Dark','Full_Logo_Dark_Transparent','Full_Logo_Light','Full_Logo_Light_Transparent',
          'Wordmark_Dark','Wordmark_Dark_Transparent','Wordmark_Light','Wordmark_Light_Transparent'):
    rend(S[k],width=3000).save(P('03_PNG',k+'.png'))
S['Symbol_Only']=S['Symbol_Dark_Transparent']; S['Symbol_Only_Light']=S['Symbol_Light_Transparent']   # used by the icons below

# app icons
def app_icon(dark,size=1024):
    img=Image.new('RGBA',(size,size),(0,0,0,0)); dr=ImageDraw.Draw(img); rr=int(size*0.225)
    dr.rounded_rectangle((0,0,size-1,size-1),rr,fill=(0,0,0,255) if dark else (255,255,255,255))
    if not dark: dr.rounded_rectangle((0,0,size-1,size-1),rr,outline=(225,225,225,255),width=max(2,size//256))
    w=int(size*0.62); s=rend(S['Symbol_Only'] if dark else S['Symbol_Only_Light'],width=w)
    img.alpha_composite(s,((size-w)//2,(size-s.size[1])//2+int(size*0.01))); return img
ad=app_icon(True); al=app_icon(False)
ad.save(P('04_App_Icons','AppIcon_Dark_1024.png')); al.save(P('04_App_Icons','AppIcon_Light_1024.png'))
sizes=[16,24,32,48,64,128,256]; ic=[app_icon(True,s) for s in sizes]
ic[-1].save(P('04_App_Icons','Windows.ico'),sizes=[(s,s) for s in sizes],append_images=ic[:-1])
ad.save(P('04_App_Icons','macOS.icns'),sizes=[(16,16),(32,32),(64,64),(128,128),(256,256),(512,512),(1024,1024)])

# Illustrator
for nm,fn,col,bg in (('light','Full_Logo_Light.pdf','#000000',LIGHT_BG),('dark','Full_Logo_Dark.pdf','#ffffff',DARK_BG),
                     ('light','Full_Logo_Light_Transparent.pdf','#000000',None),('dark','Full_Logo_Dark_Transparent.pdf','#ffffff',None)):
    d=V[nm]; pdfw.write_pdf(P('05_Illustrator',fn),r(d['full']),d['faces'],ORDER,d['fits'],[(d['wm']['head']['paths'],col),(d['wm']['tag']['paths'],col)],scale=0.4,bg=bg)
for nm,fn,bg in (('dark','Symbol_Dark.pdf',DARK_BG),('light','Symbol_Light.pdf',LIGHT_BG)):
    d=V[nm]; pdfw.write_pdf(P('05_Illustrator',fn),r(d['symvb']),d['faces'],ORDER,d['fits'],[],scale=0.4,bg=bg)
d=V['light']; epsmod.write_eps(P('05_Illustrator','Full_Logo.eps'),r(d['full']),d['faces'],ORDER,d['fits'],d['wm']['head']['paths'],'#000000',d['wm']['tag']['paths'],'#000000')
ill=S['Full_Logo_Light_Transparent'].replace('<svg xmlns="http://www.w3.org/2000/svg"','<svg version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"',1)
open(P('05_Illustrator','Illustrator_Compatible.svg'),'w',encoding='utf-8').write('<?xml version="1.0" encoding="UTF-8"?>\n'+ill)
VO={}
for fn in ('Full_Logo_Light.pdf','Full_Logo_Dark.pdf'):
    dd=fitz.open(P('05_Illustrator',fn)); pg=dd[0]
    VO[fn]={'pages':len(dd),'embedded_images':len(pg.get_images()),'size_pt':[round(pg.rect.width,1),round(pg.rect.height,1)]}
    pg.get_pixmap(dpi=96,alpha=False).save('work/pdf2_'+fn.replace('.pdf','.png'))
json.dump(VO,open('work/verify_outputs.json','w'),indent=1)

# -------- comparison --------
def contours_mask(gray,dark,thr,scale=2):
    g=cv2.resize(gray,None,fx=scale,fy=scale,interpolation=cv2.INTER_CUBIC)
    return ((g>thr) if dark else (g<thr)).astype(np.uint8)*255
def chamfer(a,b,scale=2,largest=False):
    ka,_=cv2.findContours(a,cv2.RETR_EXTERNAL,cv2.CHAIN_APPROX_NONE); kb,_=cv2.findContours(b,cv2.RETR_EXTERNAL,cv2.CHAIN_APPROX_NONE)
    if largest:
        ka=[max(ka,key=cv2.contourArea)]; kb=[max(kb,key=cv2.contourArea)]
    A=np.vstack([k[:,0,:] for k in ka]).astype(float); B=np.vstack([k[:,0,:] for k in kb]).astype(float)
    d1=cKDTree(B).query(A)[0]/scale; d2=cKDTree(A).query(B)[0]/scale
    return {'mean_px':round(float((d1.mean()+d2.mean())/2),3),'p95_px':round(float(np.percentile(np.r_[d1,d2],95)),3),'max_px':round(float(max(d1.max(),d2.max())),3)}
MET={}
for nm,suf in (('dark',''),('light','_Light')):
    d=V[nm]; v=d['v']
    svg=logo.wrap((0,0,FW,FH),d['defs'],'<g id="symbol">\n'+d['sbody']+'\n</g>\n'+wmbody(nm,d['col']),bg='#%02x%02x%02x'%v.bg)
    vec=np.array(rend(svg).convert('RGB')); ref=getattr(v,'frame_orig',v.frame)
    fx0,fy0,fw,fh=d['full']; X0,Y0=int(max(fx0,0)),int(max(fy0,0)); X1,Y1=int(min(fx0+fw,FW)),int(min(fy0+fh,FH))
    rc=ref[Y0:Y1,X0:X1]; vc=vec[Y0:Y1,X0:X1]
    sv=lambda n,a: Image.fromarray(np.clip(a,0,255).astype(np.uint8)).save(P('06_Comparison',n+suf+'.png'))
    sv('Original',rc); sv('Vector_Render',vc)
    # side by side: traced original (left) | golden construction (right)
    Image.fromarray(np.concatenate([rc,vc],1)).save(P('06_Comparison','Original_vs_Golden'+suf+'.png'))
    MET[nm]={'note':'golden construction: intentionally different from the traced original'}
json.dump(MET,open('work/metrics2.json','w'),indent=1,ensure_ascii=False)
json.dump({nm:{'faces':{k:[len(f.cmds)+1 for f in [V[nm]['faces'][k]]][0] for k in ORDER},'G':{k:np.round(V[nm]['G'][k],1).tolist() for k in ('TL','A_apex','inner_top','BLc','BRL','BRR')},'symbb':np.round(V[nm]['symbb'],1).tolist(),'full_vb':r(V[nm]['full']),
          'fits':{k:[len(V[nm]['fits'][k][0][2]),len(V[nm]['fits'][k][1][2])] for k in ORDER}} for nm in V},open('work/geo2.json','w'),indent=1)
print(json.dumps(MET,indent=1,ensure_ascii=False))
