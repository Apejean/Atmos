import numpy as np, cv2, io, json, sys
sys.path.insert(0,'work')
from geom import *
from snap import *
import resvg_py
from PIL import Image

big=cv2.imread('work/sym_big.png'); H,W=big.shape[:2]
rgb=big[:,:,::-1].astype(float)
VB=(42,43,1070,817)

def reduce_curve(poly,n):
    d=np.r_[0,np.cumsum(np.linalg.norm(np.diff(poly,axis=0),axis=1))]
    ts=np.linspace(0,d[-1],n)
    return np.c_[np.interp(ts,d,poly[:,0]),np.interp(ts,d,poly[:,1])]

def smooth_poly(p,w=5):
    k=np.ones(w)/w; q=p.copy()
    for c in (0,1):
        pad=np.r_[[p[0,c]]*(w//2),p[:,c],[p[-1,c]]*(w//2)]
        q[:,c]=np.convolve(pad,k,'valid')
    q[0]=p[0];q[-1]=p[-1]; return q

def on(line,y):
    c,d,_=line; return c+d*((y-c[1])/d[1])

def geometry():
    G={}
    LO=snap_line((514,73),(42,859)); RO=snap_line((691,48),(1112,856))
    LI1=snap_line((691,48),(587,282),r=8); LI2=snap_line((587,282),(437,532),r=8)
    RI=snap_line((726,550),(884,860),r=8)
    BL=snap_line((42,859),(298,860),r=6,margin=0.15); BR=snap_line((884,860),(1112,856),r=6,margin=0.15)
    G['lines']=dict(LO=LO,RO=RO,LI1=LI1,LI2=LI2,RI=RI,BL=BL,BR=BR)
    G['TL']=on(LO,73)
    G['A_apex']=isect(LI1,RO)
    G['inner_top']=isect(LI1,LI2)
    G['BLc']=isect(LO,BL); G['BRL']=isect(RO,BR); G['BRR']=isect(RI,BR)
    UP=[(42,859),(110,782),(200,700),(300,627),(380,573),(437,534),(500,508),(568,491),(640,488),(700,498),(760,520),(850,565),(920,615),(980,665),(1033,705)]
    sn=smooth_poly(snap_curve(UP,r=8),7)
    up=reduce_curve(sn,16); up[0]=G['BLc']; up[-1]=on(RO,up[-1][1])
    G['up']=up
    LOWp=[(298,860),(418,732),(487,673),(567,621),(671,577),(722,555)]
    sl=smooth_poly(snap_curve(LOWp,r=8),7); low=reduce_curve(sl,9)
    G['low']=low
    G['li2_end']=on(LI2,532)
    G['leg_hide1']=(300,765)
    G['RI_top']=on(RI,550)
    G['rleg_a']=(700,498)
    G['rleg_b']=(722,555)
    G['G_LOW']=[(728,556),(800,574),(870,607),(930,647),(985,688)]
    return G

def build(G):
    faces={}
    tl=G['TL']; ap=G['A_apex']; inner=G['inner_top']
    p=Path(tl).smooth([tuple(tl),(553,59),(675,43)]).L(tuple(ap),tuple(inner),tuple(G['li2_end']),G['leg_hide1'],tuple(G['BLc']))
    faces['leg_left']=p
    p=Path(ap).L(tuple(G['BRL']),tuple(G['BRR']),tuple(G['RI_top']),G['rleg_a'],tuple(inner)); faces['leg_right']=p
    up=G['up']; split=int(np.argmin(np.abs(up[:,0]-700)))
    upsegs=cr_bez([tuple(q) for q in up])
    p=Path(G['BLc']).C(upsegs[:split]); p.L((706,500),(728,557))
    low=G['low']; lsegs=cr_bez([tuple(q) for q in low]); rev=[(b,c2,c1,a) for (a,c1,c2,b) in reversed(lsegs)]
    p.L(tuple(low[-1])).C(rev).L(tuple(G['BLc'])); faces['arch_front']=p
    ue=tuple(up[-1])
    # lower right block: everything below the arch upper edge inside the right leg
    q=Path(upsegs[split][0]).C(upsegs[split:]).L(tuple(G['BRL']),tuple(G['BRR']),tuple(G['RI_top']))
    faces['arch_right_low']=q
    # thin dark band along the upper edge
    q=Path(upsegs[split][0]).C(upsegs[split:])
    gl=cr_bez([tuple(G['RI_top'])]+[tuple(x) for x in G['G_LOW']]+[ue]); revg=[(b,c2,c1,a) for (a,c1,c2,b) in reversed(gl)]
    q.L(ue).C(revg); faces['arch_right_band']=q
    return faces
ORDER=['leg_right','leg_left','arch_front','arch_right_low','arch_right_band']

def fit_dir(xs,ys,val):
    A=np.c_[xs,ys,np.ones(len(xs))]; sol=np.linalg.lstsq(A,val,rcond=None)[0]
    d=np.array(sol[:2]); n=np.linalg.norm(d)
    return d/n if n>1e-6 else np.array([0,1.0])

def bins(t,n):
    t0,t1=np.percentile(t,[0.5,99.5]); e=np.linspace(t0,t1,n+1); return t0,t1,e

def fit_face(mask,n1=12,n2=12):
    ys,xs=np.nonzero(mask); col=rgb[ys,xs]; lum=col.mean(1)
    d1=fit_dir(xs,ys,lum)
    t=xs*d1[0]+ys*d1[1]; t0,t1,e=bins(t,n1)
    model=np.zeros_like(col); stops1=[]
    for i in range(n1):
        sel=(t>=e[i])&(t<=e[i+1]) if i<n1-1 else (t>=e[i])
        if sel.sum()<2: continue
        c=np.median(col[sel],0); model[sel]=c; stops1.append(((e[i]+e[i+1])/2,c))
    cen=np.array([xs.mean(),ys.mean()]); tc=cen@d1
    lay1=(cen+d1*(t0-tc),cen+d1*(t1-tc),[((s-t0)/(t1-t0),c) for s,c in stops1])
    d2=np.array([-d1[1],d1[0]]); t2=xs*d2[0]+ys*d2[1]; u0,u1,e2=bins(t2,n2)
    stops2=[]
    for i in range(n2):
        sel=(t2>=e2[i])&(t2<=e2[i+1]) if i<n2-1 else (t2>=e2[i])
        if sel.sum()<2: continue
        m=model[sel].mean(); r=(col[sel]-model[sel]).mean()
        if r>=0: stops2.append(((e2[i]+e2[i+1])/2,(255,255,255),min(1,r/max(255-m,1))))
        else: stops2.append(((e2[i]+e2[i+1])/2,(0,0,0),min(1,-r/max(m,1))))
    tc2=cen@d2
    lay2=(cen+d2*(u0-tc2),cen+d2*(u1-tc2),[((s-u0)/(u1-u0),c,a) for s,c,a in stops2])
    return lay1,lay2

def svg_of(faces,fits,bg=None,vb=VB,overlay=True):
    o=['<svg xmlns="http://www.w3.org/2000/svg" viewBox="%d %d %d %d" width="%d" height="%d">'%(vb+(vb[2],vb[3])),'<defs>']
    for k,(l1,l2) in fits.items():
        p0,p1,st=l1
        o.append('<linearGradient id="g_%s" gradientUnits="userSpaceOnUse" x1="%s" y1="%s" x2="%s" y2="%s">'%(k,f(p0[0]),f(p0[1]),f(p1[0]),f(p1[1])))
        for s,c in st: o.append('<stop offset="%.3f" stop-color="#%02x%02x%02x"/>'%(s,*[int(round(v)) for v in c]))
        o.append('</linearGradient>')
        p0,p1,st=l2
        o.append('<linearGradient id="h_%s" gradientUnits="userSpaceOnUse" x1="%s" y1="%s" x2="%s" y2="%s">'%(k,f(p0[0]),f(p0[1]),f(p1[0]),f(p1[1])))
        for s,c,a in st: o.append('<stop offset="%.3f" stop-color="#%02x%02x%02x" stop-opacity="%.3f"/>'%(s,*c,a))
        o.append('</linearGradient>')
    o.append('</defs>')
    if bg: o.append('<rect x="%d" y="%d" width="%d" height="%d" fill="%s"/>'%(vb+(bg,)))
    for k in ORDER:
        o.append('<g id="%s"><path d="%s" fill="url(#g_%s)"/>'%(k,faces[k].d(),k))
        if overlay: o.append('<path d="%s" fill="url(#h_%s)"/>'%(faces[k].d(),k))
        o.append('</g>')
    o.append('</svg>'); return '\n'.join(o)

def render(svg):
    return Image.open(io.BytesIO(bytes(resvg_py.svg_to_bytes(svg_string=svg)))).convert('RGBA')

def fit_all(faces):
    fits={}
    for k in ORDER:
        m=mask_of(faces[k],(H,W))
        for k2 in ORDER[ORDER.index(k)+1:]:
            m=cv2.bitwise_and(m,cv2.bitwise_not(mask_of(faces[k2],(H,W))))
        m=cv2.erode(m,np.ones((7,7),np.uint8)); fits[k]=fit_face(m>0)
    return fits

if __name__=='__main__':
    G=geometry(); faces=build(G); fits=fit_all(faces)
    ref=rgb[43:860,42:1112]; sil=(ref.mean(2)>30)
    for ov in (False,True):
        svg=svg_of(faces,fits,bg='#000',overlay=ov)
        a=np.array(render(svg).convert('RGB')).astype(float); d=np.abs(a-ref).mean(2)
        print('overlay',ov,'diff all %.2f sil %.2f'%(d.mean(),d[sil].mean()))
    open('work/sym2_test.svg','w').write(svg)
    Image.fromarray(a.astype(np.uint8)).save('work/sym_render.png')
    Image.fromarray(np.clip(d*3,0,255).astype(np.uint8)).save('work/sym_diff.png')
    print({k:np.round(G[k],1).tolist() for k in('TL','A_apex','inner_top','BLc','BRL','BRR')})
    print({k:(np.round(v[0],1).tolist(),np.round(v[1],3).tolist(),v[2]) for k,v in G['lines'].items()})
