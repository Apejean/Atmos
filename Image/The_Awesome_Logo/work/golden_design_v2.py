"""Golden-ratio construction of The Awesome symbol (no spiral; ratios only).

Unit u = H/26. Frame W x H = 34u x 26u = phi^2 : 2 = 1.309 (the source symbol is 1.331).
  apex A           x = W/phi (21u) - golden section of the width
  right leg        primary stroke: foot 8u (Fibonacci), outer edge A -> (34u, base) (slope 1:2), inner edge parallel
  left leg         secondary stroke: perpendicular thickness t_L = t_R / phi; fold line from A, outer edge parallel
  top cut          rises 1u over 8u (Fibonacci 1:8) from the left top corner to A
  arch             golden ellipses (vertical semi-axis = phi x horizontal); upper edge through the left foot corner
                   and the ribbon tip E (5u above the base, on the right outer edge); crest height = hole-apex height / phi;
                   lower edge = the same ellipse shifted, through the arch foot (8u) with crest thickness t_A = t_L / phi
  stroke hierarchy t_R : t_L : t_A = phi^2 : phi : 1
"""
import numpy as np
from geom import Path, cr_bez

PHI=(1+5**.5)/2

def L(p,q): p=np.array(p,float); q=np.array(q,float); return (p,(q-p)/np.linalg.norm(q-p))
def isect(a,b):
    (c1,d1),(c2,d2)=a,b; A=np.array([d1,-d2]).T; t=np.linalg.solve(A,c2-c1); return c1+d1*t[0]
def on_y(l,y): c,d=l; return c+d*((y-c[1])/d[1])
def circle_line(cc,R,l,pick):
    c,d=l; f=c-cc; b=f@d; disc=b*b-(f@f-R*R)
    ts=[-b-np.sqrt(disc),-b+np.sqrt(disc)]; pts=[c+d*t for t in ts]; return pick(pts)
def arc_pts(cc,R,a0,a1,n):
    a=np.linspace(a0,a1,n); return np.c_[cc[0]+R*np.cos(a),cc[1]+R*np.sin(a)]
def ang(cc,p): return np.arctan2(p[1]-cc[1],p[0]-cc[0])

def design(H=26.0):
    """frame W x H = 34u x 26u = phi^2 : 2 (1.309, the source symbol's proportion 1.331 within 1.7%)"""
    W=34.0
    D={'W':W,'H':H}
    A=np.array([W/PHI,0.0]); BRL=np.array([W,H]); BLc=np.array([0.0,H])     # apex on the golden section of the width (21u)
    # right leg (front ribbon, primary stroke): foot 8u (Fibonacci); outer slope 13/26 = 1/2
    BRR=np.array([W-8.0,H]); RO=L(A,BRL); RI=(BRR,RO[1])
    t_R=abs(RO[1][0]*(BRR-BRL)[1]-RO[1][1]*(BRR-BRL)[0])
    # left leg (back ribbon, secondary stroke): perpendicular thickness t_L = t_R / phi
    t_L=t_R/PHI
    from scipy.optimize import brentq
    def tl(fl):
        d=np.array([A[0]-fl,-H]); d/=np.linalg.norm(d); return abs(d[1]*fl)-t_L
    fL=brentq(tl,1,12); Fl=np.array([fL,H])
    LI=L(A,Fl); LO=(BLc,LI[1])
    TOP=L(A,A+np.array([-8.0,1.0]))
    TL=isect(TOP,LO); inner=isect(LI,RI)
    h_hole=H-inner[1]
    # ---- arch: golden ELLIPSES (horizontal semi-axis = phi x vertical semi-axis). Computed as circles in a space
    # squeezed horizontally by phi (x' = x/phi), where they are circles, then mapped back (exact affine). ----
    K=1/PHI
    sq=lambda p: np.array([p[0]/K,p[1]]); sqL=lambda l: (sq(l[0]),(lambda d: d/np.linalg.norm(d))(np.array([l[1][0]/K,l[1][1]])))
    # upper edge: through the left foot corner BLc and the ribbon tip E (on the right outer edge, 5u above the base);
    # crest height = h_hole / phi
    hE=5.0; E=on_y(RO,H-hE); ch=h_hole/PHI; yt=H-ch
    from scipy.optimize import brentq
    Es=sq(E)
    def f(cx):
        R=(cx**2+(H-yt)**2)/(2*(H-yt)); cy=yt+R; return (Es[0]-cx)**2+(Es[1]-cy)**2-R**2
    xcs=brentq(f,1,W/K-0.1); R1=(xcs**2+(H-yt)**2)/(2*(H-yt)); C1=np.array([xcs,yt+R1]); xc=xcs*K
    t_leg=t_L
    # lower edge: the SAME ellipse (same axes), shifted: through the arch foot F (8u, Fibonacci) and through P on the
    # right leg's inner edge; P's height is solved so the arch thickness at the crest = t_L / phi
    foot=8.0; F=np.array([foot,H]); R2=R1; Fs=sq(F); RIs=sqL(RI)
    def lower(hP):
        P=on_y(RI,H-hP); Ps=sq(P); m=(Fs+Ps)/2; dv=Ps-Fs; dl=np.linalg.norm(dv); n_=np.array([-dv[1],dv[0]])/dl
        hg=np.sqrt(R2**2-(dl/2)**2); C=m+n_*hg
        if C[1]<m[1]: C=m-n_*hg
        Qy=C[1]-np.sqrt(R2**2-(xcs-C[0])**2); return C,P,Qy-yt
    t_arch=t_leg/PHI
    hP=brentq(lambda h: lower(h)[2]-t_arch, hE+0.3, H*0.6)
    C2,P,t_crest=lower(hP); Q=np.array([xc,yt+t_crest])
    usq=lambda p: np.array([p[0]*K,p[1]])
    pa=usq(circle_line(C1,R1,RIs,lambda P_:min(P_,key=lambda q:q[1])))
    lb=usq(circle_line(C2,R2,RIs,lambda P_:min(P_,key=lambda q:q[1])))
    D.update(A=A,BRL=BRL,BRR=BRR,BLc=BLc,F=F,Fl=Fl,TL=TL,inner=inner,C1=C1,R1=R1,C2=C2,R2=R2,K=K,E=E,pa=pa,lb=lb,
             xc=xc,ch=ch,h_hole=h_hole,t_leg=t_leg,foot=foot,hE=hE,hP=hP,Q=Q,P=P,t_crest=t_crest,t_arch=t_arch,
             lines=dict(RO=RO,RI=RI,LI=LI,LO=LO,TOP=TOP),t_right=t_R,t_L=t_L,fL=fL,
             ell1=(usq(C1),R1*K,R1),ell2=(usq(C2),R2*K,R2))
    return D

def arc_path_pts(cc,R,p0,p1,n=40):
    a0,a1=ang(cc,p0),ang(cc,p1)
    # go the short way over the top (angles are in (-pi,0] for the upper half of a circle below its centre)
    if abs(a1-a0)>np.pi: a1+=2*np.pi*(1 if a1<a0 else -1)
    return arc_pts(cc,R,a0,a1,n)

def bezier_arc(cc,R,p0,p1):
    """exact-ish circular arc as cubic segments (<=45 deg each)"""
    a0,a1=ang(cc,p0),ang(cc,p1)
    if abs(a1-a0)>np.pi: a1+=2*np.pi*(1 if a1<a0 else -1)
    n=max(1,int(np.ceil(abs(a1-a0)/(np.pi/4)))); segs=[]
    for i in range(n):
        b0=a0+(a1-a0)*i/n; b1=a0+(a1-a0)*(i+1)/n; k=4/3*np.tan((b1-b0)/4)
        P0=cc+R*np.array([np.cos(b0),np.sin(b0)]); P3=cc+R*np.array([np.cos(b1),np.sin(b1)])
        T0=np.array([-np.sin(b0),np.cos(b0)]); T1=np.array([-np.sin(b1),np.cos(b1)])
        segs.append((P0,P0+k*R*T0,P3-k*R*T1,P3))
    return segs

def ebezier(D,C,p0,p1):
    K=D['K']; sq=lambda p: np.array([p[0]/K,p[1]])
    R=np.linalg.norm(sq(p0)-C)
    segs=bezier_arc(C,R,sq(p0),sq(p1)); u=lambda p: np.array([p[0]*K,p[1]])
    return [tuple(u(x) for x in s_) for s_ in segs]

def faces_units(D):
    un=lambda v:v/np.linalg.norm(v)
    A,BRL,BRR,BLc,F,TL,inner=D['A'],D['BRL'],D['BRR'],D['BLc'],D['F'],D['TL'],D['inner']
    C1,R1,C2,R2,E,pa,lb=D['C1'],D['R1'],D['C2'],D['R2'],D['E'],D['pa'],D['lb']
    LI=D['lines']['LI']; faces={}
    up_l=ebezier(D,C1,BLc,pa); up_r=ebezier(D,C1,pa,E)
    lo_l=ebezier(D,C2,F,lb)
    # left leg: visible part above the arch; below the arch upper edge it is hidden, its polygon follows the arch middle
    mid=[]
    for x in np.linspace(1.0,inner[0]-1,6):
        K=D['K']; y1=C1[1]-np.sqrt(R1**2-(x/K-C1[0])**2); y2=C2[1]-np.sqrt(max(R2**2-(x/K-C2[0])**2,0)) if x>=F[0] else D['H']
        mid.append((x,(y1+min(y2,D['H']))/2))
    xin=[q for q in mid if isect(LI,(np.array(q),np.array([1.0,0])))[0]>q[0]]
    p=Path(tuple(TL)).L(tuple(A),tuple(inner))
    # go down the fold line until it is inside the arch, then back along the arch middle to the foot
    y_in=C1[1]-np.sqrt(R1**2-(on_y(LI,D['H']-D['ch'])[0]/D['K']-C1[0])**2)
    p.L(tuple(on_y(LI,min(y_in+1.2,D['H']-0.5))))
    for q in reversed(mid): p.L(q)
    p.L(tuple(BLc)); faces['leg_left']=p
    faces['leg_right']=Path(tuple(A)).L(tuple(BRL),tuple(BRR),tuple(inner))
    # arch front (left of the right leg's inner edge), 0.08u overlap under the right-hand faces
    t_up=un(up_r[0][1]-pa); t_lo=un(lb-lo_l[-1][2])
    q=Path(tuple(BLc)).C(up_l).L(tuple(pa+t_up*0.08),tuple(lb+t_lo*0.08),tuple(lb))
    rev=[(b,c2,c1,a) for (a,c1,c2,b) in reversed(lo_l)]
    q.C(rev).L(tuple(BLc)); faces['arch_front']=q
    faces['arch_right_low']=Path(tuple(pa)).C(up_r).L(tuple(BRL),tuple(BRR))
    # band: lower edge continues the arch's lower edge tangent at lb and meets the upper edge tangentially at E
    tu=un(E-up_r[-1][2]); Lb=np.linalg.norm(E-lb)
    band_low=[(lb,lb+t_lo*0.30*Lb,E-tu*0.34*Lb,E)]
    revb=[(b,c2,c1,a) for (a,c1,c2,b) in reversed(band_low)]
    r=Path(tuple(pa)).C(up_r); r.C(revb).L(tuple(pa)); faces['arch_right_band']=r
    return faces

def to_frame(faces,u,org):
    def xf(p): return np.asarray(p,float)*u+org
    out={}
    for k,p in faces.items():
        q=Path(tuple(xf(p.start)))
        for c,ps in p.cmds: q.cmds.append((c,[xf(x) for x in ps]))
        q.cur=xf(p.cur); out[k]=q
    return out,xf
