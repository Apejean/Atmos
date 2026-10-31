import numpy as np, cv2, io, json, sys
sys.path.insert(0,'work')
from geom import *
import resvg_py
from PIL import Image

big=cv2.imread('work/sym_big.png')           # BGR, coords = "big"
H,W=big.shape[:2]
rgb=big[:,:,::-1].astype(float)

# ---------- geometry (big coords) ----------
TOP=[(437,532),(500,508),(568,491),(640,488),(700,498),(760,520),(850,565),(920,615),(980,665),(1033,705)]
topsegs=cr_bez(TOP)
i700=4   # segs[0..3] up to (700,498)
LOW=[(298,860),(418,732),(487,673),(567,621),(671,577),(722,555)]
lowsegs=cr_bez(LOW)
UPL=[(42,859),(110,782),(200,700),(300,625),(400,565),(437,532)]
upl=cr_bez(UPL)

def build():
    faces={}
    # left leg (back layer)
    p=Path((514,73)).smooth([(514,73),(553,59),(675,43)]).L((691,48),(587,282),(437,532),(300,765),(42,859))
    faces['leg_left']=p
    # right leg
    p=Path((691,48)).L((1112,856),(884,860),(726,550),(700,498),(587,282)); faces['leg_right']=p
    # arch front face (left of right leg)
    p=Path((42,859)).C(upl).C(topsegs[:i700])
    p.L((722,555))
    # lower edge back to (298,860): reverse of lowsegs
    rev=[(b,c2,c1,a) for (a,c1,c2,b) in reversed(lowsegs)]
    p.C(rev).L((42,859)); faces['arch_front']=p
    # arch right band
    G_LOW=[(722,555),(790,592),(860,635),(930,685),(990,728),(1046,742)]
    p=Path((700,498)).C(topsegs[i700:]).L((1046,742))
    gl=cr_bez(G_LOW); revg=[(b,c2,c1,a) for (a,c1,c2,b) in reversed(gl)]
    p.C(revg).L((700,498)); faces['arch_right']=p
    return faces
ORDER=['leg_right','leg_left','arch_front','arch_right']

def fit_gradient(mask,nstops=12):
    ys,xs=np.nonzero(mask)
    col=rgb[ys,xs]; lum=col.mean(1)
    A=np.c_[xs,ys,np.ones(len(xs))]
    sol=np.linalg.lstsq(A,lum,rcond=None)[0]
    d=np.array(sol[:2]); n=np.linalg.norm(d)
    d=d/n if n>1e-6 else np.array([0,1.0])
    t=xs*d[0]+ys*d[1]
    t0,t1=np.percentile(t,[0.5,99.5])
    edges=np.linspace(t0,t1,nstops+1)
    stops=[]
    for i in range(nstops):
        sel=(t>=edges[i])&(t<=edges[i+1])
        if sel.sum()<3: continue
        c=np.median(col[sel],0)
        stops.append(((edges[i]+edges[i+1])/2-t0,c))
    c0=xs[np.argmin(t)],ys[np.argmin(t)]
    # endpoints along d
    cen=np.array([xs.mean(),ys.mean()]); tc=cen@d
    p0=cen+d*(t0-tc); p1=cen+d*(t1-tc)
    return p0,p1,[(s/(t1-t0),c) for s,c in stops]

def svg_of(faces,grads,vb=(42,43,1070,817),bg=None):
    o=['<svg xmlns="http://www.w3.org/2000/svg" viewBox="%d %d %d %d" width="%d" height="%d">'%(vb+(vb[2],vb[3]))]
    o.append('<defs>')
    for k,(p0,p1,st) in grads.items():
        o.append('<linearGradient id="g_%s" gradientUnits="userSpaceOnUse" x1="%s" y1="%s" x2="%s" y2="%s">'%(k,f(p0[0]),f(p0[1]),f(p1[0]),f(p1[1])))
        for s,c in st: o.append('<stop offset="%.3f" stop-color="#%02x%02x%02x"/>'%(s,*[int(round(v)) for v in c]))
        o.append('</linearGradient>')
    o.append('</defs>')
    if bg: o.append('<rect x="%d" y="%d" width="%d" height="%d" fill="%s"/>'%(vb+(bg,)))
    for k in ORDER:
        o.append('<path id="%s" d="%s" fill="url(#g_%s)"/>'%(k,faces[k].d(),k))
    o.append('</svg>')
    return '\n'.join(o)

def render(svg,scale=1):
    b=resvg_py.svg_to_bytes(svg_string=svg)
    return Image.open(io.BytesIO(bytes(b))).convert('RGBA')

if __name__=='__main__':
    faces=build()
    grads={}
    for k in ORDER:
        m=mask_of(faces[k],(H,W))
        # visible mask: subtract faces above in order
        for k2 in ORDER[ORDER.index(k)+1:]:
            m=cv2.bitwise_and(m,cv2.bitwise_not(mask_of(faces[k2],(H,W))))
        m=cv2.erode(m,np.ones((7,7),np.uint8))
        grads[k]=fit_gradient(m>0)
    svg=svg_of(faces,grads,bg='#000')
    open('work/sym_test.svg','w').write(svg)
    im=render(svg); a=np.array(im.convert('RGB')).astype(float)
    ref=rgb[43:860,42:1112]
    d=np.abs(a-ref).mean(2)
    print('size',im.size,'mean abs diff',d.mean())
    sil=(ref.mean(2)>30)
    print('mean abs diff in silhouette',d[sil].mean())
    Image.fromarray(a.astype(np.uint8)).save('work/sym_render.png')
    dm=np.clip(d*3,0,255).astype(np.uint8)
    Image.fromarray(dm).save('work/sym_diff.png')
    ov=(0.5*a+0.5*ref).astype(np.uint8); Image.fromarray(ov).save('work/sym_ov.png')
