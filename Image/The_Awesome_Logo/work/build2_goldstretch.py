import sys, os, io, json, shutil
sys.path.insert(0,'work')
import numpy as np, cv2
from PIL import Image, ImageDraw
import resvg_py, fitz
import pipe, logo, pdfw
import eps as epsmod
from pipe import FW,FH,ORDER
from scipy.spatial import cKDTree

ROOT='The_Awesome_Logo_Final'
if os.path.exists(ROOT): shutil.rmtree(ROOT)
for d in ('01_Original_Reference','02_SVG','03_PNG','04_App_Icons','05_Illustrator','06_Comparison','07_Documentation'):
    os.makedirs(os.path.join(ROOT,d))
def P(*a): return os.path.join(ROOT,*a)

V={}
for nm,col in (('dark','#ffffff'),('light','#000000')):
    v,G,faces,fits,wm=pipe.run(nm)
    allp=np.vstack([faces[k].poly() for k in ORDER])
    sb=(allp.min(0),allp.max(0))
    hb=np.array(wm['head']['bbox']); tb=np.array(wm['tag']['bbox'])
    tx0=min(hb[:,0,0].min(),tb[:,0,0].min()); tx1=max(hb[:,1,0].max(),tb[:,1,0].max()); ty1=tb[:,1,1].max(); ty0=hb[:,0,1].min()
    x0=min(sb[0][0],tx0); x1=max(sb[1][0],tx1); y0=sb[0][1]; y1=ty1
    M=130
    V[nm]=dict(v=v,G=G,faces=faces,fits=fits,wm=wm,col=col,symbb=sb,full=(x0-M,y0-M,x1-x0+2*M,y1-y0+2*M),
               wmvb=(tx0-60,ty0-60,tx1-tx0+120,ty1-ty0+120),symvb=(sb[0][0]-30,sb[0][1]-30,sb[1][0]-sb[0][0]+60,sb[1][1]-sb[0][1]+60))
    V[nm]['defs'],V[nm]['sbody']=logo.sym_defs_and_body(faces,fits)

def wmbody(nm,color):
    logo.WM={'head':{'paths':V[nm]['wm']['head']['paths']},'tag':{'paths':V[nm]['wm']['tag']['paths']}}
    return logo.wm_body(color)
def r(vb): return tuple(round(float(x),1) for x in vb)
def full_svg(nm,color,title):
    d=V[nm]; return logo.wrap(r(d['full']),d['defs'],'<g id="symbol">\n'+d['sbody']+'\n</g>\n'+wmbody(nm,color),title=title)
def sym_svg(nm): d=V[nm]; return logo.wrap(r(d['symvb']),d['defs'],'<g id="symbol">\n'+d['sbody']+'\n</g>',title='The Awesome - Symbol')
def wm_svg(nm,color): d=V[nm]; return logo.wrap(r(d['wmvb']),'',wmbody(nm,color),title='The Awesome - Wordmark')

S={'Symbol_Only':sym_svg('dark'),'Symbol_Only_Light':sym_svg('light'),
   'Wordmark_Tagline':wm_svg('light','#000000'),'Wordmark_Tagline_White':wm_svg('dark','#ffffff'),
   'Full_Logo_Light':full_svg('light','#000000','The Awesome - Full Logo (for light backgrounds)'),
   'Full_Logo_Dark':full_svg('dark','#ffffff','The Awesome - Full Logo (for dark backgrounds)')}
for k,vv in S.items(): open(P('02_SVG',k+'.svg'),'w',encoding='utf-8').write(vv)

def rend(svg,width=None,bg=None):
    kw={}
    if width: kw['width']=width
    if bg: kw['background']=bg
    return Image.open(io.BytesIO(bytes(resvg_py.svg_to_bytes(svg_string=svg,**kw)))).convert('RGBA')

# PNG
sym=rend(S['Symbol_Only'],width=1024); cv=Image.new('RGBA',(1024,1024),(0,0,0,0)); cv.paste(sym,(0,(1024-sym.size[1])//2),sym); cv.save(P('03_PNG','Symbol_Transparent_1024.png'))
rend(S['Full_Logo_Light'],width=3000).save(P('03_PNG','Full_Logo_Light_Transparent.png'))
rend(S['Full_Logo_Dark'],width=3000).save(P('03_PNG','Full_Logo_Dark_Transparent.png'))

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
for nm,fn,col in (('light','Full_Logo_Light.pdf','#000000'),('dark','Full_Logo_Dark.pdf','#ffffff')):
    d=V[nm]; pdfw.write_pdf(P('05_Illustrator',fn),r(d['full']),d['faces'],ORDER,d['fits'],[(d['wm']['head']['paths'],col),(d['wm']['tag']['paths'],col)],scale=0.4)
d=V['light']; epsmod.write_eps(P('05_Illustrator','Full_Logo.eps'),r(d['full']),d['faces'],ORDER,d['fits'],d['wm']['head']['paths'],'#000000',d['wm']['tag']['paths'],'#000000')
ill=S['Full_Logo_Light'].replace('<svg xmlns="http://www.w3.org/2000/svg"','<svg version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"',1)
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
    sv('Original',rc); sv('Vector_Render',vc); sv('Overlay_Comparison',0.5*rc.astype(float)+0.5*vc.astype(float))
    diff=np.abs(rc.astype(float)-vc.astype(float)).mean(2); sv('Difference_Map',diff*4)
    gr=cv2.cvtColor(rc,cv2.COLOR_RGB2GRAY); gv=cv2.cvtColor(vc,cv2.COLOR_RGB2GRAY)
    dark=(nm=='dark'); thr_o=30 if dark else 236
    mr=contours_mask(gr,dark,thr_o); mv=contours_mask(gv,dark,thr_o)
    out=np.zeros(mr.shape+(3,),np.uint8)
    c1,_=cv2.findContours(mr,cv2.RETR_LIST,cv2.CHAIN_APPROX_NONE); c2,_=cv2.findContours(mv,cv2.RETR_LIST,cv2.CHAIN_APPROX_NONE)
    cv2.drawContours(out,c1,-1,(255,60,60),1); cv2.drawContours(out,c2,-1,(60,255,255),1); sv('Outline_Comparison',out)
    # metrics
    sb=d['symbb']; sx0,sy0=(sb[0]-[X0,Y0]).astype(int)-4; sx1,sy1=(sb[1]-[X0,Y0]).astype(int)+4
    thr_i=60 if dark else 215
    sl=(slice(max(sy0,0),sy1),slice(max(sx0,0),sx1))
    def iou(a,b): return float((a&b).sum()/max((a|b).sum(),1))
    ra=(gr>thr_i) if dark else (gr<thr_i); va=(gv>thr_i) if dark else (gv<thr_i)
    # symbol silhouette: vector alpha mask vs reference mask (threshold vs local background, largest component)
    svs=logo.wrap((0,0,FW,FH),d['defs'],'<g>'+d['sbody']+'</g>')
    al=np.array(rend(svs))[:,:,3][Y0:Y1,X0:X1]>127
    gref=cv2.cvtColor(ref,cv2.COLOR_RGB2GRAY)[Y0:Y1,X0:X1]
    bgl=float(np.median(gref[:20,:20]))
    rm=((gref>bgl+22) if dark else (gref<bgl-14)).astype(np.uint8)
    rm=cv2.morphologyEx(rm,cv2.MORPH_OPEN,np.ones((3,3),np.uint8))
    n_,lab,st,_=cv2.connectedComponentsWithStats(rm); big=1+int(np.argmax(st[1:,4])); rm=(lab==big)
    rm=cv2.morphologyEx(rm.astype(np.uint8),cv2.MORPH_CLOSE,np.ones((5,5),np.uint8))>0
    box_=np.zeros_like(rm); box_[max(sy0,0):sy1,max(sx0,0):sx1]=True
    ra,va=rm&box_,al&box_
    m={'symbol':{'mean_abs_diff':round(float(diff[max(sy0,0):sy1,max(sx0,0):sx1].mean()),2)}}
    # outline offset: distance from each vector-outline point to the strongest reference edge along the normal (+-8px)
    import snap as SN
    pipe.setref(v)
    alf=np.array(rend(svs))[:,:,3]>127
    cs_,_=cv2.findContours(alf.astype(np.uint8),cv2.RETR_EXTERNAL,cv2.CHAIN_APPROX_NONE); C=max(cs_,key=cv2.contourArea)[:,0,:].astype(float)
    from scipy.ndimage import gaussian_filter1d
    Cs=np.c_[gaussian_filter1d(C[:,0],3,mode='wrap'),gaussian_filter1d(C[:,1],3,mode='wrap')]
    offs=[];used=0
    for i in range(0,len(Cs),4):
        tg=Cs[(i+4)%len(Cs)]-Cs[i-4]; tg=tg/(np.linalg.norm(tg)+1e-9); nrm=np.array([-tg[1],tg[0]])
        q,val=SN.best_on_normal(Cs[i]+0.5,nrm,8)
        if val>10: offs.append(float(np.linalg.norm(q-(Cs[i]+0.5)))); used+=1
    offs=np.array(offs)
    m['symbol_outline_distance']={'mean_px':round(float(offs.mean()),2),'p95_px':round(float(np.percentile(offs,95)),2),'max_px':round(float(offs.max()),2),'points_used_pct':round(100*used/max(len(range(0,len(Cs),4)),1),1)}
    sl=(slice(max(sy0,0),sy1),slice(max(sx0,0),sx1))
    ty0=int(sb[1][1]-Y0)+30; tsl=(slice(ty0,None),slice(None))
    ra2=(gr>128) if dark else (gr<128); va2=(gv>128) if dark else (gv<128)
    m['wordmark_and_tagline']={'IoU':round(iou(ra2[tsl],va2[tsl]),4),'mean_abs_diff':round(float(diff[tsl].mean()),2)}
    hb=np.array(d['wm']['head']['bbox']); hy0,hy1=int(hb[:,0,1].min()-Y0)-6,int(hb[:,1,1].max()-Y0)+6
    hsl=(slice(hy0,hy1),slice(None))
    m['headline']={'IoU':round(iou(ra2[hsl],va2[hsl]),4),'outline_distance':chamfer(mr[hy0*2:hy1*2],mv[hy0*2:hy1*2])}
    tb=np.array(d['wm']['tag']['bbox']); ty0_,ty1_=int(tb[:,0,1].min()-Y0)-6,int(tb[:,1,1].max()-Y0)+6
    m['tagline']={'IoU':round(iou(ra2[ty0_:ty1_],va2[ty0_:ty1_]),4),'outline_distance':chamfer(mr[ty0_*2:ty1_*2],mv[ty0_*2:ty1_*2])}
    m['source_size_px']=list(Image.open('work/new_%s.png'%nm).size); m['symbol_source_bbox']=[int(x) for x in v.bbox]; m['frame_scale']=round(v.k,4)
    m['glyphs']=len(d['wm']['head']['paths']),len(d['wm']['tag']['paths']); m['wordmark_info']={k:round(float(x),3) for k,x in d['wm']['info'].items()}
    MET[nm]=m
    if nm=='dark':
        # 3-up zoom of the symbol
        a=rc[sl]; b=vc[sl]; dd=np.stack([np.clip(np.abs(a.astype(float)-b.astype(float)).mean(2)*4,0,255)]*3,2)
        Image.fromarray(np.concatenate([a,b,dd.astype(np.uint8)],1)).save(P('06_Comparison','Symbol_Zoom_3up.png'))
    else:
        a=rc[sl]; b=vc[sl]; dd=np.stack([np.clip(np.abs(a.astype(float)-b.astype(float)).mean(2)*4,0,255)]*3,2)
        Image.fromarray(np.concatenate([a,b,dd.astype(np.uint8)],1)).save(P('06_Comparison','Symbol_Zoom_3up_Light.png'))
json.dump(MET,open('work/metrics2.json','w'),indent=1,ensure_ascii=False)
json.dump({nm:{'faces':{k:[len(f.cmds)+1 for f in [V[nm]['faces'][k]]][0] for k in ORDER},'G':{k:np.round(V[nm]['G'][k],1).tolist() for k in ('TL','A_apex','inner_top','BLc','BRL','BRR')},'symbb':np.round(V[nm]['symbb'],1).tolist(),'full_vb':r(V[nm]['full']),
          'fits':{k:[len(V[nm]['fits'][k][0][2]),len(V[nm]['fits'][k][1][2])] for k in ORDER}} for nm in V},open('work/geo2.json','w'),indent=1)
print(json.dumps(MET,indent=1,ensure_ascii=False))
