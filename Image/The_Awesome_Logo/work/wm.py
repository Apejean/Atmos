import numpy as np, cv2, sys, json
sys.path.insert(0,'work')
from bezfit import fit_closed
from geom import f

im=cv2.imread('work/dark_panel.png',0).astype(np.float32)
S=8
def trace(box,thr_frac=0.5):
    x0,y0,x1,y1=box
    c=im[y0:y1,x0:x1]
    bg=np.percentile(c,20); fg=np.percentile(c,99.7)
    big=cv2.resize(c,None,fx=S,fy=S,interpolation=cv2.INTER_CUBIC)
    thr=bg+(fg-bg)*thr_frac
    m=(big>thr).astype(np.uint8)*255
    cs,h=cv2.findContours(m,cv2.RETR_CCOMP,cv2.CHAIN_APPROX_NONE)
    items=[]
    for i,k in enumerate(cs):
        a=cv2.contourArea(k)
        if a<40: continue
        items.append((i,k[:,0,:].astype(float),h[0][i][3]))
    return items,(x0,y0),bg,fg,thr

def glyphs(box):
    items,(x0,y0),bg,fg,thr=trace(box)
    outers=[(i,k) for i,k,p in items if p==-1]
    holes=[(i,k,p) for i,k,p in items if p!=-1]
    res=[]
    for i,k in outers:
        parts=[k]+[kk for j,kk,p in holes if p==i]
        res.append(parts)
    # sort left->right by bbox
    res.sort(key=lambda ps:ps[0][:,0].min())
    return res,(x0,y0),(bg,fg,thr)

def straight(c,tol=1.2):
    p0,p1,p2,p3=c; ch=p3-p0; L=np.linalg.norm(ch)
    if L<1e-9: return False
    n=np.array([-ch[1],ch[0]])/L
    return abs((p1-p0)@n)<tol and abs((p2-p0)@n)<tol

def poly_sharp(k,eps=5.0,minlen=14):
    """crisp polygon for a straight-edged glyph contour (k in 8x px): long edges are line-fitted,
    rounded-corner bevels (short edges produced by the source blur) are dropped and corners re-intersected."""
    kk=k.astype(np.float32).reshape(-1,1,2)
    ap=cv2.approxPolyDP(kk,eps,True)[:,0,:]
    n=len(k); m_=len(ap)
    idx=[int(np.argmin(np.linalg.norm(k-v,axis=1))) for v in ap]
    edges=[]
    for i in range(m_):
        a_,b_=idx[i],idx[(i+1)%m_]
        seq=[(a_+j)%n for j in range(((b_-a_)%n)+1)]
        pts=k[seq]; L=np.linalg.norm(ap[(i+1)%m_]-ap[i])
        if L<minlen or len(pts)<7: continue
        pts=pts[int(len(pts)*0.15):max(int(len(pts)*0.85),int(len(pts)*0.15)+3)]
        c=pts.mean(0); u,s,vt=np.linalg.svd(pts-c); edges.append((c,vt[0]))
    if len(edges)<3: return None
    out=[]
    for i in range(len(edges)):
        (c1,d1),(c2,d2)=edges[i-1],edges[i]
        A=np.array([d1,-d2]).T
        if abs(np.linalg.det(A))<0.05: return None
        t=np.linalg.solve(A,c2-c1); out.append(c1+d1*t[0])
    return np.array(out)

def straight(c,tol=1.2):
    p0,p1,p2,p3=c; ch=p3-p0; L=np.linalg.norm(ch)
    if L<1e-9: return False
    n=np.array([-ch[1],ch[0]])/L
    return abs((p1-p0)@n)<tol and abs((p2-p0)@n)<tol

def to_path(parts,origin,err=2.2,sharp=False):
    """returns SVG path d in 'big' coords (4x of panel crop origin (160,85))"""
    from scipy.ndimage import gaussian_filter1d
    x0,y0=origin
    d=[]
    for k in parts:
        sp=poly_sharp(k) if sharp else None
        if sp is not None:
            P=(sp+0.5)/S+np.array([x0,y0]); P=(P-np.array([160,85]))*4
            d.append('M'+f(P[0][0])+' '+f(P[0][1])+''.join('L'+f(q[0])+' '+f(q[1]) for q in P[1:])+'Z'); continue
        P=(k+0.5)/S+np.array([x0,y0])
        P=(P-np.array([160,85]))*4
        P=np.c_[gaussian_filter1d(P[:,0],4,mode='wrap'),gaussian_filter1d(P[:,1],4,mode='wrap')]
        segs=fit_closed(P,err=err,win=16,ang=42)
        d.append('M'+f(segs[0][0][0])+' '+f(segs[0][0][1]))
        for c in segs:
            if straight(c): d.append('L'+f(c[3][0])+' '+f(c[3][1]))
            else: d.append('C'+' '.join(f(v) for v in (c[1][0],c[1][1],c[2][0],c[2][1],c[3][0],c[3][1])))
        d.append('Z')
    return ''.join(d)

STRAIGHT={'head':{0,1,2,3,4,5,8,9},'tag':{0,1,3,4,6,9,10,13,14,16,18}}
if __name__=='__main__':
    out={}
    for name,box in (('head',(75,335,550,378)),('tag',(120,384,505,408))):
        gl,org,info=glyphs(box)
        print(name,len(gl),'glyph blobs',info)
        paths=[];bbs=[]
        for parts in gl:
            paths.append(to_path(parts,org,sharp=(len(paths) in STRAIGHT[name])))
            allp=np.vstack(parts); bb=((allp.min(0)+0.5)/S+np.array(org),(allp.max(0)+0.5)/S+np.array(org))
            bbs.append([bb[0].tolist(),bb[1].tolist()])
        out[name]={'paths':paths,'bbox_panel_px':bbs}
    json.dump(out,open('work/wordmark.json','w'))
    for n in out: print(n,[ [round(v,1) for v in b[0]+b[1]] for b in out[n]['bbox_panel_px']])
