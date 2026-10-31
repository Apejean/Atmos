"""Per-variant pipeline: large reference image -> canonical frame -> symbol geometry/gradients + wordmark outlines."""
import sys, io, json
sys.path.insert(0,'work')
import numpy as np, cv2
from PIL import Image
import snap as SN
import sym2 as S2
from sym2 import (reduce_curve, smooth_poly, on, build, fit_all, ORDER)
from snap import snap_line, snap_curve, isect
from geom import f, Path, cr_bez, mask_of
from bezfit import fit_closed
from scipy.ndimage import gaussian_filter1d

GOLD=True
PHI=(1+5**.5)/2
OX,OY=500,200          # frame offset of the symbol's top-left corner
SYM_W=1070.0           # target symbol width in frame units (same scale as the first pass)
FW,FH=2300,1750

class Variant: pass

def load(name):
    v=Variant(); v.name=name
    img=np.array(Image.open('work/new_%s.png'%name).convert('RGB'))
    g=cv2.GaussianBlur(cv2.cvtColor(img,cv2.COLOR_RGB2GRAY),(0,0),1.5)
    H=g.shape[0]
    m=(g>40) if name=='dark' else (g<244)
    m[int(H*0.70):]=False
    n,lab,st,_=cv2.connectedComponentsWithStats(m.astype(np.uint8))
    i=1+np.argmax(st[1:,4]); x,y,w,h=st[i,:4]
    v.bbox=(x,y,w,h); v.k=SYM_W/w
    k=v.k
    small=cv2.resize(img,None,fx=k,fy=k,interpolation=cv2.INTER_AREA)
    bgc=np.median(np.r_[small[:6].reshape(-1,3),small[-6:].reshape(-1,3)],axis=0)
    fr=np.empty((FH,FW,3),np.uint8); fr[:]=bgc.astype(np.uint8)
    ox=int(round(OX-x*k)); oy=int(round(OY-y*k))
    sh,sw=small.shape[:2]
    x0=max(0,ox); y0=max(0,oy); x1=min(FW,ox+sw); y1=min(FH,oy+sh)
    fr[y0:y1,x0:x1]=small[y0-oy:y1-oy,x0-ox:x1-ox]
    v.frame=fr; v.bg=tuple(int(c) for c in bgc); v.sym_h=h*k
    v.img_to_frame=(k,ox,oy)
    return v

def setref(v):
    fr=v.frame
    S2.big=cv2.cvtColor(fr,cv2.COLOR_RGB2BGR); S2.H,S2.W=fr.shape[:2]; S2.rgb=fr[:,:,:].astype(float)
    gg=cv2.GaussianBlur(cv2.cvtColor(fr,cv2.COLOR_RGB2GRAY).astype(np.float32),(0,0),2.0)
    SN.g=gg; SN.gx=cv2.Sobel(gg,cv2.CV_32F,1,0,ksize=3); SN.gy=cv2.Sobel(gg,cv2.CV_32F,0,1,ksize=3)


def robust_poly(pts,deg,iters=6):
    x=pts[:,0]; y=pts[:,1]; w=np.ones(len(x))
    xm=x.mean(); xs=(x-xm)/(x.max()-x.min())
    for _ in range(iters):
        c=np.polynomial.polynomial.polyfit(xs,y,deg,w=w)
        res=y-np.polynomial.polynomial.polyval(xs,c); sc=1.4826*np.median(np.abs(res))+0.8
        w=1.0/np.maximum(1.0,np.abs(res)/(2.0*sc))
    return lambda X: np.polynomial.polynomial.polyval((X-xm)/(x.max()-x.min()),c)

def smooth_curve(poly,deg,x0,x1,n):
    fn=robust_poly(poly,deg)
    X=np.linspace(x0,x1,400); Y=fn(X); D=np.r_[0,np.cumsum(np.hypot(np.diff(X),np.diff(Y)))]
    t=np.linspace(0,D[-1],n); return np.c_[np.interp(t,D,X),np.interp(t,D,Y)]

def geometry(v,r=10):
    sy=v.sym_h/817.0
    def T(p): return (OX+(p[0]-42), OY+(p[1]-43)*sy)
    sl=lambda a,b,**kw: snap_line(T(a),T(b),r=kw.pop('r',r),**kw)
    G={}
    LO=sl((514,73),(42,859)); RO=sl((691,48),(1112,856))
    # the hole's left edge (bright leg vs black) is the reliable one: fit it, then extend the SAME line up to the apex
    LI1=sl((587,282),(437,532),r=16,margin=0.08,k=80); LI2=LI1
    RI=sl((587,282),(884,860),r=10,margin=0.04)
    BL=sl((42,859),(298,860),r=6,margin=0.15); BR=sl((884,860),(1112,856),r=6,margin=0.15)
    G['lines']=dict(LO=LO,RO=RO,LI1=LI1,LI2=LI2,RI=RI,BL=BL,BR=BR)
    TOPg=[(530,68),(560,61),(590,55),(620,50),(650,46)]
    tpn=snap_curve([T(p) for p in TOPg],r=6)
    cc=tpn.mean(0); uu,ss,vv=np.linalg.svd(tpn-cc); TOPL=(cc,vv[0]*(1 if vv[0][0]>0 else -1),len(tpn))
    G['lines']['TOP']=TOPL
    G['TL']=isect(TOPL,LO)
    A0=isect(TOPL,RO)                                     # apex corner = top edge x right outer edge (kept as before)
    c0,d0,_=LI1; Pb=c0+d0*((G_y_arch-c0[1])/d0[1]) if False else None
    # fold line: ONE straight line from the apex corner to the lower end of the hole's left edge (where it meets the arch)
    yb=T((0,532))[1]; Pb=on(LI1,yb)
    dd=(Pb-A0)/np.linalg.norm(Pb-A0); LIs=(A0.copy(),dd,LI1[2])
    G['dev_inner']=float(abs(((on(LI1,T((0,290))[1])-A0)@np.array([-dd[1],dd[0]]))))
    LI1=LIs; LI2=LI1; G['lines']['LI1']=LI1; G['lines']['LI2']=LI2
    G['A_apex']=A0
    # ---- golden landmarks: hole apex at 0.764 H (=1-phi^-3) above the base; the right leg's inner edge pivots about the right foot ----
    BRR0=isect(RI,BR); Hs=isect(LO,BL)[1]-min(A0[1],isect(TOPL,LO)[1])
    G['hole_before']=float((isect(LO,BL)[1]-isect(LI1,RI)[1])/Hs); G['apex_before']=float((A0[0]-isect(LO,BL)[0])/(isect(RO,BR)[0]-isect(LO,BL)[0])); G['WH_before']=float((isect(RO,BR)[0]-isect(LO,BL)[0])/Hs)
    if GOLD:
        ty=isect(LO,BL)[1]-0.7639*Hs; Ph=on(LI1,ty)
        dRI=(Ph-BRR0)/np.linalg.norm(Ph-BRR0); RI=(BRR0.copy(),dRI,RI[2]); G['lines']['RI']=RI
    G['inner_top']=isect(LI1,RI)
    G['BLc']=isect(LO,BL); G['BRL']=isect(RO,BR); G['BRR']=isect(RI,BR)
    UP=[(42,859),(110,782),(200,700),(300,627),(380,573),(437,534),(500,508),(568,491),(640,488),(700,498),(760,520),(850,565),(920,615),(980,665),(1033,705)]
    sn=snap_curve([T(p) for p in UP],r=8)
    # arch edges are smooth arcs: replace the noisy snapped polyline with a robust polynomial fit
    xe=on(RO,sn[-1][1])[0]
    # extend the fit to the right outer edge (find where the fitted curve meets RO)
    fn=robust_poly(sn,4)
    Xs=np.linspace(G['BLc'][0],xe+60,600); Ys=fn(Xs)
    c_,d_,_=RO; nrm=np.array([-d_[1],d_[0]]); sd=(np.c_[Xs,Ys]-c_)@nrm
    j=next((i for i in range(len(sd)-1) if sd[i]*sd[i+1]<=0),len(sd)-2)
    xend=Xs[j]
    up=smooth_curve(sn,4,G['BLc'][0],xend,13); up[0]=G['BLc']; up[-1]=on(RO,up[-1][1]); G['up']=up
    LOWp=[(298,860),(418,732),(487,673),(567,621),(671,577),(722,555)]
    sl_=snap_curve([T(p) for p in LOWp],r=8)
    # the arch's lower edge is an arc: least-squares circle through the snapped points -> uniform curvature, no wobble
    xx,yy=sl_[:,0],sl_[:,1]
    cc_=np.linalg.lstsq(np.c_[2*xx,2*yy,np.ones(len(xx))],xx**2+yy**2,rcond=None)[0]
    ccx,ccy=cc_[0],cc_[1]; Rr=float(np.sqrt(cc_[2]+ccx**2+ccy**2))
    yfoot=G['BLc'][1]; xfoot=ccx-np.sqrt(max(Rr**2-(yfoot-ccy)**2,1.0))
    a0=np.arctan2(yfoot-ccy,xfoot-ccx); xend=sl_[-1][0]+90
    a1=np.arctan2(-np.sqrt(max(Rr**2-(xend-ccx)**2,1.0)),xend-ccx)
    if a1<a0: a1+=2*np.pi if (a1+2*np.pi-a0)<np.pi else 0
    ang=np.linspace(a0,a1,10)
    low=np.c_[ccx+Rr*np.cos(ang),ccy+Rr*np.sin(ang)]; low[0]=(xfoot,yfoot)
    G['low']=low; G['low_circle']=(ccx,ccy,Rr)
    G['low_fn']=robust_poly(sl_,3)
    if GOLD:
        Hs2=G['BLc'][1]-min(G['A_apex'][1],G['TL'][1]); cur=(G['BLc'][1]-G['up'][:,1].min())/Hs2; dy=(0.7639*0.618034-cur)*Hs2
        def bump(P):
            x0_,x1_=P[0][0],P[-1][0]; t=(P[:,0]-x0_)/(x1_-x0_); return np.sin(np.pi*np.clip(t,0,1))
        G['up']=G['up'].copy(); G['up'][:,1]-=dy*bump(G['up'])
        G['low']=G['low'].copy(); G['low'][:,1]-=dy*bump(G['low'])
        G['crest_shift']=float(dy); G['crest_before']=float(cur); G['crest_after']=0.7639*0.618034
    G['li2_end']=on(LI2,T((0,532))[1]); G['leg_hide1']=T((300,765))
    # crossing of the arch edges with the straight inner edge RI
    def cross(curve):
        segs=cr_bez([tuple(q) for q in curve]); pts=[]
        for (a,c1,c2,b) in segs:
            for t in np.linspace(0,1,40,endpoint=False): pts.append((1-t)**3*a+3*(1-t)**2*t*c1+3*(1-t)*t*t*c2+t**3*b)
        pts.append(np.array(curve[-1],float)); pts=np.array(pts)
        c,d,_=RI; n=np.array([-d[1],d[0]]); sd=(pts-c)@n
        for i in range(len(pts)-1):
            if sd[i]==0 or sd[i]*sd[i+1]<0:
                t=sd[i]/(sd[i]-sd[i+1]); return pts[i]+(pts[i+1]-pts[i])*t
        j=int(np.argmin(np.abs(sd))); return pts[j]
    G['rleg_a']=cross(G['up'][:int(len(G['up'])*0.75)+1] if False else G['up'])
    lowx=cross(np.vstack([G['low'],G['low'][-1]+(G['low'][-1]-G['low'][-2])*1.5]))
    G['rleg_b']=lowx
    G['G_LOW']=None
    tp=np.array([G['TL'],G['A_apex']])
    G['top']=tp
    G['T']=T
    return G

def build_faces(G):
    T=G['T']
    un=lambda v:v/(np.linalg.norm(v)+1e-9)
    faces={}
    tl=G['TL']; ap=G['A_apex']; inner=G['inner_top']
    faces['leg_left']=Path(tuple(tl)).L(tuple(ap),tuple(inner),tuple(G['li2_end']),G['leg_hide1'],tuple(G['BLc']))
    faces['leg_right']=Path(ap).L(tuple(G['BRL']),tuple(G['BRR']),tuple(inner))
    up=G['up']; pa=np.array(G['rleg_a'])
    keep=[tuple(q) for q in up if q[0]<pa[0]-6]
    after=[tuple(q) for q in up if q[0]>pa[0]+6]
    full=keep+[tuple(pa)]+after                      # ONE Catmull-Rom curve -> tangent-continuous at the crossing
    segs=cr_bez(full); nl=len(keep)
    upsegs_l=segs[:nl]; upsegs_r=segs[nl:]
    ue=np.array(up[-1])
    p=Path(G['BLc']).C(upsegs_l)
    low=G['low']; lb=np.array(G['rleg_b'])
    lctrl=[tuple(q) for q in low if q[0]<lb[0]-6]+[tuple(lb)]
    # dark band: its lower edge is the upper edge pushed down by the band thickness at RI, tapering to ZERO at the outer edge
    pts=[]
    for (a_,c1,c2,b_) in upsegs_r:
        for t in np.linspace(0,1,30,endpoint=False): pts.append((1-t)**3*a_+3*(1-t)**2*t*c1+3*(1-t)*t*t*c2+t**3*b_)
    pts.append(upsegs_r[-1][3]); pts=np.array(pts)
    D=np.r_[0,np.cumsum(np.linalg.norm(np.diff(pts,axis=0),axis=1))]; D=D/D[-1]
    w0=np.linalg.norm(lb-pa); side=lb-pa
    # lower edge = upper edge pushed down by a thickness that tapers to zero at the outer edge; sampled densely,
    # then reduced to a few control points by a low-order fit so the edge is perfectly smooth
    ts=np.linspace(0,1,60); off=[]
    for t in ts:
        i=int(np.searchsorted(D,t)); i=min(max(i,1),len(pts)-1)
        tg=un(pts[min(i+8,len(pts)-1)]-pts[max(i-8,0)]); n=np.array([-tg[1],tg[0]])
        if n@side<0: n=-n
        off.append(pts[i]+n*w0*(1-t)**2.2)
    off=np.array(off)
    cx_=np.polynomial.polynomial.polyfit(ts,off[:,0],5); cy_=np.polynomial.polynomial.polyfit(ts,off[:,1],5)
    tt=np.array([0,0.1,0.22,0.38,0.58,0.78,1.0])
    ctrl=[(float(np.polynomial.polynomial.polyval(t,cx_)),float(np.polynomial.polynomial.polyval(t,cy_))) for t in tt]
    ctrl[0]=tuple(lb); ctrl[-1]=tuple(ue)
    t_low=un(lb-np.array(lctrl[-2]))              # tangent of the arch's lower edge at RI: the band's lower edge continues it
    ctrl[1]=tuple(lb+t_low*np.linalg.norm(np.array(ctrl[1])-lb))
    # ONE smooth lower edge: the arch's lower edge continues across RI as the band's lower edge and tapers into ue
    lsegs=cr_bez(lctrl); rev=[(b_,c2,c1,a_) for (a_,c1,c2,b_) in reversed(lsegs)]
    # band lower edge = ONE cubic: leaves lb along the arch's lower-edge tangent (smooth continuation) and arrives at ue
    # along the upper edge's tangent (so the ribbon tapers to a point with no kink)
    tl_=un(np.array(lb)-np.array(lsegs[-1][2]))
    tu_=un(np.array(ue)-np.array(upsegs_r[-1][2]))
    Lb=np.linalg.norm(np.array(ue)-np.array(lb))
    rsegs=[(np.array(lb),np.array(lb)+tl_*0.30*Lb,np.array(ue)-tu_*0.34*Lb,np.array(ue))]
    t_low=tl_
    acc=np.r_[0,np.cumsum(np.linalg.norm(np.diff(pts,axis=0),axis=1))]
    ext_a=pts[int(np.searchsorted(acc,3))]
    ext_b=np.array(lb)+t_low*3
    p.L(tuple(ext_a),tuple(ext_b),tuple(lb)).C(rev).L(tuple(G['BLc'])); faces['arch_front']=p
    faces['arch_right_low']=Path(tuple(pa)).C(upsegs_r).L(tuple(G['BRL']),tuple(G['BRR']))
    gl=rsegs; revg=[(b_,c2,c1,a_) for (a_,c1,c2,b_) in reversed(gl)]
    q=Path(tuple(pa)).C(upsegs_r); q.C(revg).L(tuple(pa)); faces['arch_right_band']=q
    return faces

# ---------------- wordmark ----------------
def straight(c,tol=1.0):
    p0,p1,p2,p3=c; ch=p3-p0; L=np.linalg.norm(ch)
    if L<1e-9: return False
    n=np.array([-ch[1],ch[0]])/L
    return abs((p1-p0)@n)<tol and abs((p2-p0)@n)<tol

def poly_sharp(k,eps,minlen):
    kk=k.astype(np.float32).reshape(-1,1,2)
    ap=cv2.approxPolyDP(kk,eps,True)[:,0,:]
    n=len(k); m_=len(ap)
    idx=[int(np.argmin(np.linalg.norm(k-vv,axis=1))) for vv in ap]
    ev=[ap[(i+1)%m_]-ap[i] for i in range(m_)]
    def turn(i):  # turning angle at vertex i between edge i-1 and edge i (degrees)
        a=ev[i-1];b=ev[i]; c=(a@b)/(np.linalg.norm(a)*np.linalg.norm(b)+1e-9); return np.degrees(np.arccos(np.clip(c,-1,1)))
    hgt=(k[:,1].max()-k[:,1].min())
    cand=[ (np.linalg.norm(ev[i])<0.16*hgt and turn(i)<68 and turn((i+1)%m_)<68) for i in range(m_)]
    if all(cand): return None
    unit=lambda v:v/(np.linalg.norm(v)+1e-9)
    forced={}      # edge index -> forced direction (round end caps)
    keep=[not c for c in cand]
    i0=next(i for i in range(m_) if not cand[i])
    i=i0
    for _ in range(m_):
        j=(i+1)%m_
        if cand[j]:
            run=[]; t=j
            while cand[t%m_]: run.append(t%m_); t+=1
            d_in=unit(ev[i]); d_out=unit(ev[t%m_])
            ang=np.degrees(np.arccos(np.clip(d_in@d_out,-1,1)))
            if ang>135 and len(run)>=2:
                perp=np.array([-d_in[1],d_in[0]])
                r=min(run,key=lambda q:abs(unit(ev[q])@d_in)); keep[r]=True; forced[r]=perp
            i=t%m_
        else: i=j
        if i==i0: break
    edges=[]
    for i in range(m_):
        if not keep[i]: continue
        a_,b_=idx[i],idx[(i+1)%m_]
        seq=[(a_+j)%n for j in range(((b_-a_)%n)+1)]
        pts=k[seq]
        if len(pts)<3: continue
        if i not in forced: pts=pts[int(len(pts)*0.15):max(int(len(pts)*0.85),int(len(pts)*0.15)+2)]
        c=pts.mean(0)
        if i in forced: dv=forced[i]*(1 if forced[i]@ev[i]>0 else -1)
        else:
            u,s_,vt=np.linalg.svd(pts-c); dv=vt[0].copy()
        for ax in (np.array([1.0,0.0]),np.array([0.0,1.0])):
            if abs(dv@ax)>np.cos(np.radians(2.5)): dv=ax*np.sign(dv@ax)
        edges.append((c,dv))
    if len(edges)<3: return None
    out=[]
    for i in range(len(edges)):
        (c1,d1),(c2,d2)=edges[i-1],edges[i]
        A=np.array([d1,-d2]).T
        if abs(np.linalg.det(A))<0.05: return None
        t=np.linalg.solve(A,c2-c1); out.append(c1+d1*t[0])
    out=np.array(out)
    # reject if a corner lies far from the traced contour (unstable intersection)
    # accept only if the polygon reproduces the traced glyph shape (area overlap)
    x0,y0=np.floor(k.min(0)).astype(int)-6; x1,y1=np.ceil(k.max(0)).astype(int)+6
    if out.min()<-1e5 or out.max()>1e6: return None
    from scipy.spatial import cKDTree
    if cKDTree(k).query(out)[0].max()>10*SS: return None
    ma=np.zeros((y1-y0,x1-x0),np.uint8); mb=ma.copy()
    cv2.fillPoly(ma,[(k-[x0,y0]).round().astype(np.int32)],1); cv2.fillPoly(mb,[(out-[x0,y0]).round().astype(np.int32)],1)
    iou=(ma&mb).sum()/max((ma|mb).sum(),1)
    return out if iou>=0.90 else None

ELLIPSE={'head':{7},'tag':{5,7}}
SGS={'cur':None}
STRAIGHT={'head':{0,1,2,3,4,5,8,9},'tag':{0,1,3,4,6,9,10,11,13,14,16,18}}
SS=3   # contour upsampling

def trace_text(v,y_from):
    g=cv2.cvtColor(v.frame,cv2.COLOR_RGB2GRAY).astype(np.float32)
    dark=(v.name=='dark')
    reg=g[y_from:]
    big=cv2.resize(reg,None,fx=SS,fy=SS,interpolation=cv2.INTER_CUBIC)
    if dark: bg=np.percentile(big,20); fg=np.percentile(big,99.8); m=(big>(bg+(fg-bg)*0.5))
    else: fg=np.percentile(big,0.2); bg=np.percentile(big,80); m=(big<(bg+(fg-bg)*0.5))
    m=(m*255).astype(np.uint8)
    cs,h=cv2.findContours(m,cv2.RETR_CCOMP,cv2.CHAIN_APPROX_NONE)
    items=[(i,k[:,0,:].astype(float),h[0][i][3]) for i,k in enumerate(cs) if cv2.contourArea(k)>60*SS*SS/4]
    outers=[(i,k) for i,k,p in items if p==-1]
    glyphs=[]
    for i,k in outers:
        parts=[k]+[kk for j,kk,p in items if p==i]
        bb=(k.min(0),k.max(0)); glyphs.append((parts,bb))
    hs=[b[1][1]-b[0][1] for _,b in glyphs]; thr=(max(hs)+min(hs))/2
    heads=[x for x in glyphs if x[1][1][1]-x[1][0][1]>thr]; tags=[x for x in glyphs if x[1][1][1]-x[1][0][1]<=thr]
    # the multiplication sign is shorter than letters but belongs to the tagline (already in tags)
    heads.sort(key=lambda x:x[1][0][0]); tags.sort(key=lambda x:x[1][0][0])
    return heads,tags,y_from

def ellipse_d(k,y0):
    P=(k+0.5)/SS+np.array([0,y0])
    (cx,cy),(MA,ma),ang=cv2.fitEllipse(P.astype(np.float32).reshape(-1,1,2))
    a,b=MA/2,ma/2; th=np.radians(ang); R=np.array([[np.cos(th),-np.sin(th)],[np.sin(th),np.cos(th)]])
    kk=0.5522847498
    pts=[(a,0),(a,b*kk),(a*kk,b),(0,b),(-a*kk,b),(-a,b*kk),(-a,0),(-a,-b*kk),(-a*kk,-b),(0,-b),(a*kk,-b),(a,-b*kk),(a,0)]
    q=[R@np.array(p)+np.array([cx,cy]) for p in pts]
    o='M'+f(q[0][0])+' '+f(q[0][1])
    for i in range(1,13,3): o+='C'+' '.join(f(v) for pp in q[i:i+3] for v in pp)
    return o+'Z'

def to_path(parts,y0,sharp,eps,minlen,err,ellipse=False):
    d=[]
    for k in parts:
        if ellipse: d.append(ellipse_d(k,y0)); continue
        sp=poly_sharp(k,eps,minlen) if sharp else None
        if sp is not None:
            P=sp/SS+np.array([0,y0]); P=P+0.5/SS
            d.append('M'+f(P[0][0])+' '+f(P[0][1])+''.join('L'+f(q[0])+' '+f(q[1]) for q in P[1:])+'Z'); continue
        P=(k+0.5)/SS+np.array([0,y0])
        sg=1.9*SS if sharp else (SGS.get('cur') or 3.2*SS)
        P=np.c_[gaussian_filter1d(P[:,0],sg,mode='wrap'),gaussian_filter1d(P[:,1],sg,mode='wrap')]
        segs=fit_closed(P,err=err,win=int(7*SS/2),ang=38 if sharp else 42)
        d.append('M'+f(segs[0][0][0])+' '+f(segs[0][0][1]))
        for c in segs:
            if straight(c): d.append('L'+f(c[3][0])+' '+f(c[3][1]))
            else: d.append('C'+' '.join(f(x) for x in (c[1][0],c[1][1],c[2][0],c[2][1],c[3][0],c[3][1])))
        d.append('Z')
    return ''.join(d)

def wordmark(v,sym_bottom):
    heads,tags,y0=trace_text(v,int(sym_bottom+40))
    out={}
    for name,gl in (('head',heads),('tag',tags)):
        paths=[];bbs=[]
        for i,(parts,bb) in enumerate(gl):
            SGS['cur']=(6.0*SS if (name=='head' and i==6) else None)
            paths.append(to_path(parts,y0,i in STRAIGHT[name],eps=2.4*SS/2,minlen=7*SS/2,err=1.4,ellipse=(i in ELLIPSE[name])))
            lo=bb[0]/SS+np.array([0,y0]); hi=bb[1]/SS+np.array([0,y0]); bbs.append([lo.tolist(),hi.tolist()])
        out[name]={'paths':paths,'bbox':bbs}
    import tagfix
    out['info']=tagfix.regularise(out)
    return out

def golden_affine(G,corr=1.0):
    """symbol bounding box -> golden rectangle (W = phi*H) about the base centre, apex moved to 0.618 W (shear: base stays fixed)"""
    yb=G['BLc'][1]; ytop=min(G['A_apex'][1],G['TL'][1]); H=yb-ytop
    x0=G['BLc'][0]; x1=G['BRL'][0]; W=x1-x0; cx=(x0+x1)/2
    sx=PHI*H/W*corr
    apex_frac=(G['A_apex'][0]-cx)/W+0.5                  # apex position across the base before scaling (kept by the scale)
    dx_apex=(0.618034-apex_frac)*PHI*H                   # shear that puts the apex at 0.618 of the new width
    def xf(p):
        p=np.asarray(p,float); x,y=p[0],p[1]
        return np.array([cx+(x-cx)*sx+dx_apex*(yb-y)/(yb-G['A_apex'][1]),y])
    def xf_inv_cols(xq,yq):
        x1_=xq-dx_apex*(yb-yq)/(yb-G['A_apex'][1]); return cx+(x1_-cx)/sx
    return xf,xf_inv_cols,dict(sx=sx,shear=dx_apex,H=H,W=W,W_new=W*sx,cx=cx,apex_frac_before=apex_frac,yb=yb)

def transform_path(p,xf):
    q=Path(tuple(xf(p.start)))
    for k,ps in p.cmds:
        q.cmds.append((k,[xf(x) for x in ps]))
    q.cur=xf(p.cur); return q

def run(name):
    v=load(name); setref(v)
    G=geometry(v); faces=build_faces(G)
    v.frame_orig=v.frame.copy()
    if GOLD:
        faces0=faces; corr=1.0
        for _ in range(3):
            xf,xinv,info=golden_affine(G,corr)
            faces={k:transform_path(p,xf) for k,p in faces0.items()}
            allp=np.vstack([faces[k].poly() for k in faces]); Wb,Hb=allp.max(0)-allp.min(0)
            err=Wb/Hb/PHI
            if abs(err-1)<0.0015: break
            corr/=err
        info['bbox_W']=float(Wb); info['bbox_H']=float(Hb)
        for kk in ('TL','A_apex','inner_top','BLc','BRL','BRR','rleg_a','rleg_b'):
            G[kk]=xf(G[kk])
        G['golden']=info
        # warp the symbol rows of the reference with the inverse map so the gradients are fitted on the adjusted shape
        yb=int(info['yb'])+20; fr=v.frame.copy()
        yy,xx=np.mgrid[0:yb,0:fr.shape[1]].astype(np.float32)
        mx=xinv(xx,yy).astype(np.float32)
        fr[:yb]=cv2.remap(v.frame[:yb],mx,yy,cv2.INTER_CUBIC,borderMode=cv2.BORDER_REPLICATE)
        v.frame=fr; setref(v)
    fits=fit_all(faces)
    sym_bottom=max(G['BLc'][1],G['BRL'][1])
    wm=wordmark(v,sym_bottom)
    return v,G,faces,fits,wm
if __name__=='__main__':
    for nm in ('dark','light'):
        v,G,faces,fits,wm=run(nm)
        print(nm,'k=%.4f'%v.k,'bbox',v.bbox,'frame sym h %.1f'%v.sym_h,'glyphs head',len(wm['head']['paths']),'tag',len(wm['tag']['paths']))
        print(' corners',{k:np.round(G[k],1).tolist() for k in ('TL','A_apex','inner_top','BLc','BRL','BRR')})
        print(' line pts',{k:c[2] for k,c in G['lines'].items()})
        Image.fromarray(v.frame).save('work/frame_%s.png'%nm)
