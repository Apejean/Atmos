import numpy as np, cv2
big=cv2.imread('work/sym_big.png')
g=cv2.GaussianBlur(cv2.cvtColor(big,cv2.COLOR_BGR2GRAY).astype(np.float32),(0,0),2.0)
gx=cv2.Sobel(g,cv2.CV_32F,1,0,ksize=3); gy=cv2.Sobel(g,cv2.CV_32F,0,1,ksize=3)
def samp(a,x,y):
    return cv2.getRectSubPix(a,(1,1),(float(x),float(y)))[0,0]
def best_on_normal(p,n,r=10,sign=None):
    best=None;bv=-1
    for s in np.arange(-r,r+0.01,0.5):
        q=p+n*s
        v=samp(gx,*q)*n[0]+samp(gy,*q)*n[1]
        v=abs(v) if sign is None else v*sign
        if v>bv: bv=v;best=q
    return best,bv
def snap_line(p0,p1,r=10,margin=0.06,sign=None,k=40):
    p0=np.array(p0,float);p1=np.array(p1,float)
    d=p1-p0;L=np.linalg.norm(d);d/=L;n=np.array([-d[1],d[0]])
    pts=[]
    for t in np.linspace(margin,1-margin,k):
        q,v=best_on_normal(p0+(p1-p0)*t,n,r,sign)
        if v>8: pts.append(q)
    pts=np.array(pts)
    c=pts.mean(0);u,s,vt=np.linalg.svd(pts-c);dirv=vt[0]
    if dirv@d<0: dirv=-dirv
    return c,dirv,len(pts)
def isect(a,b):
    (c1,d1,_),(c2,d2,_)=a,b
    A=np.array([d1,-d2]).T; t=np.linalg.solve(A,c2-c1)
    return c1+d1*t[0]
def snap_curve(pts,step=20,r=10,sign=None,keep=None):
    from geom import cr_bez
    segs=cr_bez(pts)
    poly=[]
    for (a,c1,c2,b) in segs:
        for t in np.linspace(0,1,12,endpoint=False):
            poly.append((1-t)**3*a+3*(1-t)**2*t*c1+3*(1-t)*t*t*c2+t**3*b)
    poly.append(np.array(pts[-1],float)); poly=np.array(poly)
    out=[]
    for i,p in enumerate(poly):
        a=poly[max(i-1,0)];b=poly[min(i+1,len(poly)-1)];d=b-a;d/=np.linalg.norm(d)+1e-9
        n=np.array([-d[1],d[0]])
        q,v=best_on_normal(p,n,r,sign)
        out.append(q if v>8 else p)
    return np.array(out)
