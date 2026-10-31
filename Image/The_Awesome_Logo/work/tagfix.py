"""Regularise the wordmark: repeated letters share one outline, small tagline letters reuse the headline
outlines (same typeface, scaled) or are rebuilt from straight strokes measured on the traced glyph."""
import re, numpy as np
from geom import f

def parse(d):
    toks=re.findall(r'[MLCZ]|-?[\d.]+(?:e-?\d+)?',d); out=[];i=0
    while i<len(toks):
        t=toks[i]
        if t=='Z': out.append(('Z',[])); i+=1; continue
        n={'M':1,'L':1,'C':3}[t]
        pts=[(float(toks[i+1+2*j]),float(toks[i+2+2*j])) for j in range(n)]
        out.append((t,pts)); i+=1+2*n
    return out

def dump(cmds):
    return ''.join(t+' '.join(f(v) for p in pts for v in p) if t!='Z' else 'Z' for t,pts in cmds)

def xform(d,sx,sy,tx,ty):
    return dump([(t,[(x*sx+tx,y*sy+ty) for x,y in pts]) for t,pts in parse(d)])

def bbox(d):
    P=np.array([p for t,pts in parse(d) for p in pts]); return P.min(0),P.max(0)

def poly_stats(d):
    # area/perimeter of the (first) polygon-like contour using its vertices
    cmds=parse(d); area=0;per=0
    for sub in re.split(r'(?<=Z)',d):
        if not sub: continue
        P=np.array([p for t,pts in parse(sub) for p in pts[-1:]])
        if len(P)<3: continue
        x,y=P[:,0],P[:,1]; a=0.5*abs(np.dot(x,np.roll(y,-1))-np.dot(y,np.roll(x,-1)))
        area+= a if sub is None else a
        per+=np.linalg.norm(P-np.roll(P,-1,axis=0),axis=1).sum()
    return area,per

def poly(pts): return 'M'+'L'.join(f(x)+' '+f(y) for x,y in pts)+'Z'

def isect(p,q,r,s):
    p,q,r,s=map(np.array,(p,q,r,s)); A=np.array([q-p,-(s-r)]).T; t=np.linalg.solve(A,r-p); return p+(q-p)*t[0]

def build_N(x0,y0,x1,y1,w):
    W=x1-x0;H=y1-y0; dw=w/np.cos(np.arctan2(W-w,H))
    pA=isect((x0+dw,y0),(x1,y1),(x1-w,y0),(x1-w,y1)); pB=isect((x0,y0),(x1-dw,y1),(x0+w,y0),(x0+w,y1))
    return poly([(x0,y0),(x0+dw,y0),tuple(pA),(x1-w,y0),(x1,y0),(x1,y1),(x1-dw,y1),tuple(pB),(x0+w,y1),(x0,y1)])

def build_L(x0,y0,x1,y1,w): return poly([(x0,y0),(x0+w,y0),(x0+w,y1-w),(x1,y1-w),(x1,y1),(x0,y1)])
def build_I(x0,y0,x1,y1,w):
    c=(x0+x1)/2; return poly([(c-w/2,y0),(c+w/2,y0),(c+w/2,y1),(c-w/2,y1)])
def build_Y(x0,y0,x1,y1,w):
    W=x1-x0;H=y1-y0;cx=(x0+x1)/2; yj=y0+0.50*H
    dw=w/np.cos(np.arctan2(W/2-w/2,yj-y0))
    sl=(yj-y0)/(cx-w/2-x0)
    yv=y0+(cx-(x0+dw))*sl
    return poly([(x0,y0),(x0+dw,y0),(cx,yv),(x1-dw,y0),(x1,y0),(cx+w/2,yj),(cx+w/2,y1),(cx-w/2,y1),(cx-w/2,yj)])
def build_X(x0,y0,x1,y1,w):
    W=x1-x0;H=y1-y0; dw=w/np.cos(np.arctan2(W-w,H)); cx=(x0+x1)/2; cy=(y0+y1)/2
    top=isect((x0+dw,y0),(x1,y1),(cx,y0),(cx,y1)); bot=isect((x0,y1),(x1-dw,y0),(cx,y0),(cx,y1))
    # simple 12-gon: use the symmetric construction
    b=isect((x0+dw,y0),(x1,y1),(x1-dw,y0),(x0,y1))   # not used
    lft=isect((x0,y0),(x1-dw,y1),(x0,y1),(x1-dw,y0)); 
    top=isect((x0+dw,y0),(x1,y1),(x1-dw,y0),(x0,y1))
    bot=isect((x0,y0),(x1-dw,y1),(x0+dw,y1),(x1,y0))
    lft=isect((x0,y0),(x1-dw,y1),(x0,y1),(x1-dw,y0)); rgt=isect((x0+dw,y0),(x1,y1),(x0+dw,y1),(x1,y0))
    return poly([(x0,y0),(x0+dw,y0),tuple(top),(x1-dw,y0),(x1,y0),tuple(rgt),(x1,y1),(x1-dw,y1),tuple(bot),(x0+dw,y1),(x0,y1),tuple(lft)])


from shapely.geometry import LineString, box, Polygon
from shapely.ops import unary_union
from bezfit import fit_closed

def geom_to_d(g,err=0.22):
    polys=[g] if g.geom_type=='Polygon' else list(g.geoms)
    d=''
    for p in polys:
        for ring in [p.exterior]+list(p.interiors):
            r=ring.segmentize(0.45); P=np.array(r.coords)[:-1]
            segs=fit_closed(P,err=err,win=7,ang=50)
            d+='M'+f(segs[0][0][0])+' '+f(segs[0][0][1])
            for c in segs:
                p0,p1,p2,p3=c; ch=p3-p0; L=np.linalg.norm(ch)
                n=np.array([-ch[1],ch[0]])/(L+1e-9)
                if L>1e-9 and abs((p1-p0)@n)<0.12 and abs((p2-p0)@n)<0.12: d+='L'+f(p3[0])+' '+f(p3[1])
                else: d+='C'+' '.join(f(v) for v in (p1[0],p1[1],p2[0],p2[1],p3[0],p3[1]))
            d+='Z'
    return d

def stroke(pts,w): return LineString(pts).buffer(w/2,cap_style='flat',join_style='mitre')

def build_C(x0,y0,x1,y1,w,span=42):
    cx,cy=(x0+x1)/2,(y0+y1)/2; a=(x1-x0-w)/2; b=(y1-y0-w)/2
    ph=np.radians(np.linspace(span,360-span,120))
    return geom_to_d(stroke(list(zip(cx+a*np.cos(ph),cy+b*np.sin(ph))),w))
def build_G(x0,y0,x1,y1,w):
    cx,cy=(x0+x1)/2,(y0+y1)/2; a=(x1-x0-w)/2; b=(y1-y0-w)/2
    ph=np.radians(np.linspace(-42,-360,130))
    pts=list(zip(cx+a*np.cos(ph),cy+b*np.sin(ph)))+[(cx+0.02*a,cy)]
    return geom_to_d(stroke(pts,w))
def build_R(x0,y0,x1,y1,w):
    W=x1-x0;H=y1-y0; yb=y0+0.55*H-w/2; yt=y0+w/2; r=(yb-yt)/2; xs=x0+w/2; xr=x1-w/2-r-0.02*W
    th=np.linspace(-np.pi/2,np.pi/2,40)
    bowl=[(xs,yt),(xr,yt)]+[(xr+r*np.cos(t),yt+r+r*np.sin(t)) for t in th]+[(xs,yb)]
    stem=stroke([(xs,y0),(xs,y1)],w)
    leg=stroke([(x0+0.36*W,yb),(x1-0.3*w,y1+w*1.5)],w)
    g=unary_union([stem,stroke(bowl,w),leg]).intersection(box(x0-5,y0,x1+5,y1))
    return geom_to_d(g)

def mstroke(pts,w): return LineString(pts).buffer(w/2,cap_style='flat',join_style='mitre',mitre_limit=8)

def build_A(x0,y0,x1,y1,w,bar_top):
    W=x1-x0;H=y1-y0;cx=(x0+x1)/2
    th=np.arctan2(W/2,H)            # leg angle from vertical (outer)
    dw=w/np.cos(th)
    xb=W/2-dw/2                      # centerline half-spread at feet
    ya=y0+ (w/2)/np.sin(th)          # centerline apex so the outer tip sits near y0
    lg=mstroke([(cx-xb*(1+0.3*w/H),y1+0.3*w),(cx,ya),(cx+xb*(1+0.3*w/H),y1+0.3*w)],w)
    hull=Polygon([(x0,y1),(cx,y0-w),(x1,y1)])
    bar=box(x0,bar_top,x1,bar_top+w*0.92).intersection(lg.convex_hull)
    g=unary_union([lg,bar]).intersection(box(x0-1,y0,x1+1,y1))
    return geom_to_d(g)

def build_W(x0,y0,x1,y1,w):
    W=x1-x0;H=y1-y0
    th=np.arctan2(W/4,H); dw=w/np.cos(th)
    xs=[x0+dw/2+0.0, x0+W*0.25, x0+W*0.5, x0+W*0.75, x1-dw/2]
    ys=[y0-0.4*w, y1+0.6*w, y0-0.4*w, y1+0.6*w, y0-0.4*w]
    # shift the outer ends so that the stroke outer edge reaches x0/x1 at the top
    g=mstroke(list(zip(xs,ys)),w).intersection(box(x0-1,y0,x1+1,y1))
    return geom_to_d(g)

def build_M(x0,y0,x1,y1,w,vy=0.70):
    W=x1-x0;H=y1-y0;cx=(x0+x1)/2
    g=mstroke([(x0+w/2,y1+w),(x0+w/2,y0-0.4*w),(cx,y0+vy*H),(x1-w/2,y0-0.4*w),(x1-w/2,y1+w)],w).intersection(box(x0-1,y0,x1+1,y1))
    return geom_to_d(g)

def regularise(wm):
    H=wm['head']; T=wm['tag']
    hp=H['paths']; hb=[np.array(b) for b in H['bbox']]
    tp=T['paths']; tb=[np.array(b) for b in T['bbox']]
    # -- headline: repeated E shares the cleanest outline (first E)
    e_src=2
    for j in (5,9):
        if j<len(hp):
            sx=hb[j][0][0]-hb[e_src][0][0]; sy=hb[j][0][1]-hb[e_src][0][1]
            if abs(hb[j][1][0]-hb[j][0][0]-(hb[e_src][1][0]-hb[e_src][0][0]))<0.1*(hb[e_src][1][0]-hb[e_src][0][0]):
                hp[j]=xform(hp[e_src],1,1,sx,sy)
    # headline letters A, W, M rebuilt from straight strokes (stroke width measured on T,H,E below)
    # headline stroke width from straight letters T,H,E
    ws=[]
    for j in (0,1,2):
        a,p=poly_stats(hp[j]); ws.append(2*a/p)
    w_head=float(np.median(ws))
    # rebuild A, W, M (headline) from the measured stroke width
    subs=[q for q in re.split(r'(?<=Z)',hp[3]) if q]
    hole=bbox(subs[1])[1][1] if len(subs)>1 else hb[3][0][1]+0.6*(hb[3][1][1]-hb[3][0][1])
    hp[3]=build_A(hb[3][0][0],hb[3][0][1],hb[3][1][0],hb[3][1][1],w_head,hole)
    hp[4]=build_W(hb[4][0][0],hb[4][0][1],hb[4][1][0],hb[4][1][1],w_head)
    hp[8]=build_M(hb[8][0][0],hb[8][0][1],hb[8][1][0],hb[8][1][1],w_head)
    capH=hb[0][1][1]-hb[0][0][1]; capT=tb[0][1][1]-tb[0][0][1]; s=capT/capH; w=w_head*s
    info={'w_head':w_head,'scale':s,'w_tag':w}
    # -- tagline letters from headline outlines: T E H A S O  (tag idx -> head idx)
    mp={0:0,13:0,16:0,1:2,3:1,5:7,7:7,11:3,15:6}
    for ti,hi in mp.items():
        tbb=tb[ti]; hbb=hb[hi]
        wid=(hbb[1][0]-hbb[0][0])*s; twid=tbb[1][0]-tbb[0][0]
        if abs(wid-twid)>0.14*twid: continue
        cx_t=(tbb[0][0]+tbb[1][0])/2; cx_h=(hbb[0][0]+hbb[1][0])/2
        tx=cx_t-cx_h*s; ty=tbb[1][1]-hbb[1][1]*s
        tp[ti]=xform(hp[hi],s,s,tx,ty)
    # -- straight letters rebuilt from strokes: N L I Y X
    mk={4:build_N,6:build_L,14:build_I,9:build_Y,18:build_Y,10:build_X,2:build_C,8:build_G,12:build_R,17:build_R}
    for ti,fn in mk.items():
        x0,y0=tb[ti][0]; x1,y1=tb[ti][1]
        ww=w if ti!=10 else w*0.9
        tp[ti]=fn(x0,y0,x1,y1,ww)
    return info
