import numpy as np, cv2

def cr_bez(pts, tension=0.5):
    """Catmull-Rom through pts -> list of cubic segments (p0,c1,c2,p3)."""
    P=[np.array(p,float) for p in pts]
    segs=[]
    for i in range(len(P)-1):
        p0=P[i-1] if i>0 else P[i]
        p1=P[i]; p2=P[i+1]
        p3=P[i+2] if i+2<len(P) else P[i+1]
        c1=p1+(p2-p0)/6*tension*2
        c2=p2-(p3-p1)/6*tension*2
        segs.append((p1,c1,c2,p2))
    return segs

def f(v): return ('%.1f'%v).rstrip('0').rstrip('.')
def pt(p): return f(p[0])+' '+f(p[1])

class Path:
    def __init__(s,start): s.start=np.array(start,float); s.cmds=[]; s.cur=s.start
    def L(s,*pts):
        for p in pts: s.cmds.append(('L',[np.array(p,float)])); s.cur=np.array(p,float)
        return s
    def C(s,segs):
        for (_,c1,c2,p2) in segs: s.cmds.append(('C',[c1,c2,p2])); s.cur=p2
        return s
    def smooth(s,pts):  # through pts, starting at current point (pts[0]==cur)
        return s.C(cr_bez(pts))
    def d(s):
        o=['M'+pt(s.start)]
        for k,ps in s.cmds: o.append(k+' '.join(pt(p) for p in ps))
        return ''.join(o)+'Z'
    def poly(s,n=24):
        pts=[s.start]; cur=s.start
        for k,ps in s.cmds:
            if k=='L': pts.append(ps[0]); cur=ps[0]
            else:
                c1,c2,p2=ps
                for t in np.linspace(0,1,n+1)[1:]:
                    pts.append((1-t)**3*cur+3*(1-t)**2*t*c1+3*(1-t)*t*t*c2+t**3*p2)
                cur=p2
        return np.array(pts)

def mask_of(path,shape,scale=1.0,off=(0,0)):
    m=np.zeros(shape[:2],np.uint8)
    pts=((path.poly()-np.array(off))*scale).round().astype(np.int32)
    cv2.fillPoly(m,[pts],255)
    return m
