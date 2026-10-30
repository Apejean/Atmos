"""Schneider curve fitting (Graphics Gems) for closed contours, with corner splitting."""
import numpy as np

def _bez(c,t):
    mt=1-t
    return (mt**3)[:,None]*c[0]+(3*mt*mt*t)[:,None]*c[1]+(3*mt*t*t)[:,None]*c[2]+(t**3)[:,None]*c[3]

def _chord(pts):
    d=np.r_[0,np.cumsum(np.linalg.norm(np.diff(pts,axis=0),axis=1))]
    return d/d[-1] if d[-1]>0 else d

def _gen(pts,u,t1,t2):
    p0,p3=pts[0],pts[-1]
    b0=(1-u)**3;b1=3*u*(1-u)**2;b2=3*u*u*(1-u);b3=u**3
    A1=b1[:,None]*t1;A2=b2[:,None]*t2
    C=np.array([[ (A1*A1).sum(),(A1*A2).sum()],[(A1*A2).sum(),(A2*A2).sum()]])
    tmp=pts-(b0+b1)[:,None]*p0-(b2+b3)[:,None]*p3
    X=np.array([(A1*tmp).sum(),(A2*tmp).sum()])
    det=C[0,0]*C[1,1]-C[0,1]*C[1,0]
    seg=np.linalg.norm(p3-p0)
    if abs(det)>1e-12:
        a1=(X[0]*C[1,1]-X[1]*C[0,1])/det; a2=(C[0,0]*X[1]-C[1,0]*X[0])/det
    else: a1=a2=0
    eps=1e-6*seg
    if a1<eps or a2<eps: a1=a2=seg/3
    return np.array([p0,p0+t1*a1,p3+t2*a2,p3])

def _reparam(c,pts,u):
    out=u.copy()
    for i,(p,t) in enumerate(zip(pts,u)):
        mt=1-t
        q=mt**3*c[0]+3*mt*mt*t*c[1]+3*mt*t*t*c[2]+t**3*c[3]
        d1=3*(mt*mt*(c[1]-c[0])+2*mt*t*(c[2]-c[1])+t*t*(c[3]-c[2]))
        d2=6*(mt*(c[2]-2*c[1]+c[0])+t*(c[3]-2*c[2]+c[1]))
        num=((q-p)*d1).sum(); den=(d1*d1).sum()+((q-p)*d2).sum()
        if abs(den)>1e-12: out[i]=min(1,max(0,t-num/den))
    return out

def _fit(pts,t1,t2,err,out,depth=0):
    n=len(pts)
    if n==2:
        d=np.linalg.norm(pts[1]-pts[0])/3
        out.append(np.array([pts[0],pts[0]+t1*d,pts[1]+t2*d,pts[1]])); return
    u=_chord(pts)
    for _ in range(6):
        c=_gen(pts,u,t1,t2)
        dist=np.linalg.norm(_bez(c,u)-pts,axis=1)
        m=dist.argmax()
        if dist[m]<err: out.append(c); return
        u=_reparam(c,pts,u)
    c=_gen(pts,u,t1,t2); dist=np.linalg.norm(_bez(c,u)-pts,axis=1); m=int(dist.argmax())
    if dist[m]<err*1.0: out.append(c); return
    m=min(max(m,1),n-2)
    tc=pts[m-1]-pts[m+1]; tc=tc/(np.linalg.norm(tc)+1e-12)
    _fit(pts[:m+1],t1,tc,err,out,depth+1); _fit(pts[m:],-tc,t2,err,out,depth+1)

def _unit(v):
    n=np.linalg.norm(v); return v/n if n>1e-12 else v

def fit_open(pts,err):
    pts=np.asarray(pts,float)
    k=min(4,len(pts)-1)
    t1=_unit(pts[min(k,len(pts)-1)]-pts[0]); t2=_unit(pts[max(-k-1,-len(pts))]-pts[-1])
    out=[]; _fit(pts,t1,t2,err,out); return out

def find_corners(pts,win=6,ang=50):
    n=len(pts); idx=[]
    for i in range(n):
        a=pts[i]-pts[(i-win)%n]; b=pts[(i+win)%n]-pts[i]
        na,nb=np.linalg.norm(a),np.linalg.norm(b)
        if na<1e-9 or nb<1e-9: continue
        cos=np.clip((a@b)/(na*nb),-1,1); th=np.degrees(np.arccos(cos))
        if th>ang: idx.append((th,i))
    # non-max suppression
    idx.sort(reverse=True); keep=[]
    for th,i in idx:
        if all(min((i-j)%n,(j-i)%n)>win for j in keep): keep.append(i)
    return sorted(keep)

def fit_closed(pts,err=0.6,win=6,ang=50):
    """pts: Nx2 closed contour (no repeated end point). returns list of cubic segments."""
    pts=np.asarray(pts,float); n=len(pts)
    cs=find_corners(pts,win,ang)
    if not cs:
        # split at two far points to fit smooth closed curve
        cs=[0,n//2]
    segs=[]
    for a,b in zip(cs,cs[1:]+[cs[0]+n]):
        idx=[i%n for i in range(a,b+1)]
        seg=pts[idx]
        if len(seg)<2: continue
        if len(cs)>2 or True:
            segs+=fit_open(seg,err)
    return segs
