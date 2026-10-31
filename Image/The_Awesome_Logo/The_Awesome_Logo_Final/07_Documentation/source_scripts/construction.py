"""Golden-ratio construction drawings (symbol grid + full-logo proportions) and the dimension table."""
import sys, os, io, json
sys.path.insert(0,'work')
import numpy as np, resvg_py
from PIL import Image
import golden_design as gd, goldbuild, logo
from geom import f

ROOT='The_Awesome_Logo_Final'; OUT=os.path.join(ROOT,'08_Golden_Design'); os.makedirs(OUT,exist_ok=True)
PHI=gd.PHI
D=gd.design(); faces=gd.faces_units(D)

def line_full(l,x0=-1,x1=35):
    c,d=l; pts=[]
    for t in np.linspace(-60,60,2):
        pts.append(c+d*t)
    return pts
def svg_line(p,q,cls): return '<line x1="%s" y1="%s" x2="%s" y2="%s" class="%s"/>'%(f(p[0]),f(p[1]),f(q[0]),f(q[1]),cls)
def label(p,txt,dx=0.3,dy=-0.3,cls='lab'): return '<text x="%s" y="%s" class="%s">%s</text>'%(f(p[0]+dx),f(p[1]+dy),cls,txt)

# ---------- symbol construction (units) ----------
W,H=D['W'],D['H']
o=['<svg xmlns="http://www.w3.org/2000/svg" viewBox="-3 -3.5 41 35" width="2050" height="1750">',
   '<style>.g{fill:none;stroke:#c8961e;stroke-width:0.05}.gl{fill:none;stroke:#2f6fdc;stroke-width:0.045;stroke-dasharray:0.3 0.2}'
   '.c{fill:none;stroke:#d33;stroke-width:0.045;stroke-dasharray:0.25 0.15}.f{fill:#e9ebee;stroke:#222;stroke-width:0.06}'
   '.pt{fill:#d33}.lab{font:0.55px Arial,sans-serif;fill:#222}.dim{font:0.6px Arial,sans-serif;fill:#c8961e;font-weight:bold}.ttl{font:0.8px Arial,sans-serif;fill:#111;font-weight:bold}</style>',
   '<rect x="-3" y="-3.5" width="41" height="35" fill="#ffffff"/>']
for k in ('leg_right','leg_left','arch_front','arch_right_low','arch_right_band'):
    o.append('<path class="f" d="%s"/>'%faces[k].d())
o.append('<rect class="g" x="0" y="0" width="%s" height="%s" style="stroke-width:0.09"/>'%(f(W),f(H)))
# golden sections of the frame: x = W/phi^2, W/phi ; heights H/phi^2, H/phi above the base
for x in (W/PHI**2,W/PHI): o.append(svg_line((x,0),(x,H),'g'))
for hgt in (H/PHI**2,H/PHI): o.append(svg_line((0,H-hgt),(W,H-hgt),'g'))
o.append(svg_line((0,H-5),(W,H-5),'g'))
# defining lines
clip='<clipPath id="cp"><rect x="-1.5" y="-1.5" width="37" height="29"/></clipPath>'
o.append('<defs>%s</defs><g clip-path="url(#cp)">'%clip)
for n in ('LO','LI','RO','RI','TOP'):
    p,q=line_full(D['lines'][n]); o.append(svg_line(p,q,'gl'))
for C,a_,b_ in [e for e in (D['ell1'],D['ell2']) if e is not None]:
    o.append('<ellipse class="c" cx="%s" cy="%s" rx="%s" ry="%s"/>'%(f(C[0]),f(C[1]),f(a_),f(b_)))
o.append('</g>')
pts=dict(A=D['A'],TL=D['TL'],P_hole=D['inner'],BLc=D['BLc'],F=D['F'],BRR=D['BRR'],BRL=D['BRL'],E=D['E'],pa=D['pa'],lb=D['lb'],crest=(D['xc'],H-D['ch']))
for n,p in pts.items():
    o.append('<circle class="pt" cx="%s" cy="%s" r="0.16"/>'%(f(p[0]),f(p[1])))
    o.append(label(p,'%s (%.2f, %.2f)'%(n,p[0],H-p[1])))
o.append('<text class="dim" x="17" y="-1.6" text-anchor="middle">W = 34u</text><text class="dim" x="-1.7" y="13" text-anchor="middle" transform="rotate(-90 -1.7 13)">H = 26u</text>')
o.append('<text class="dim" x="10.5" y="27.3" text-anchor="middle">W/phi = 21u</text><text class="dim" x="27.5" y="27.3" text-anchor="middle">W/phi^2 = 13u</text>')
o.append('<text class="dim" x="34.3" y="%s">H/phi</text><text class="dim" x="34.3" y="%s">H/phi^2</text><text class="dim" x="34.3" y="21.2">5u</text>'%(f(H-H/PHI+0.2),f(H-H/PHI**2+0.2)))
o.append('<text class="ttl" x="-2.5" y="30.6">The Awesome symbol - golden construction (u = H/26; coordinates = (x, height above base))</text>')
o.append('</svg>')
svg='\n'.join(o)
open(os.path.join(OUT,'Construction_Symbol.svg'),'w',encoding='utf-8').write(svg)
Image.open(io.BytesIO(bytes(resvg_py.svg_to_bytes(svg_string=svg)))).convert('RGB').save(os.path.join(OUT,'Construction_Symbol.png'))

# ---------- full logo proportions (frame px, dark variant) ----------
v,G,fc,fits,wm=goldbuild.run('dark')
defs,sb=logo.sym_defs_and_body(fc,fits)
logo.WM={'head':{'paths':wm['head']['paths']},'tag':{'paths':wm['tag']['paths']}}
hb=np.array(wm['head']['bbox']); tb=np.array(wm['tag']['bbox'])
org=G['org']; u=G['u']; Ws,Hs=G['Wsym'],G['Hsym']
sx0,sy0=org; sy1=sy0+Hs
hx0,hx1=hb[:,0,0].min(),hb[:,1,0].max(); hy0,hy1=np.median(hb[:,0,1]),np.median(hb[:,1,1])   # cap line / baseline (ignores O/S overshoot)
tx0,tx1=tb[:,0,0].min(),tb[:,1,0].max(); ty0,ty1=np.median(tb[:,0,1]),np.median(tb[:,1,1])
m=G['clear']
vb=(hx0-m,sy0-m,(hx1-hx0)+2*m+m*3.6,(ty1-sy0)+2*m)
sw=vb[2]/600
L=['<g fill="none" stroke-width="%s">'%f(sw*1.6)]
def rect(x,y,w,h,col,dash=None): return '<rect x="%s" y="%s" width="%s" height="%s" stroke="%s"%s/>'%(f(x),f(y),f(w),f(h),col,' stroke-dasharray="%s %s"'%(f(sw*8),f(sw*5)) if dash else '')
L.append(rect(hx0,sy0,hx1-hx0,hy1-sy0,'#c8961e'))                 # block symbol top -> headline bottom: golden rectangle
L.append(rect(sx0,sy0,Ws,Hs,'#c8961e'))                            # symbol: golden rectangle
L.append(rect(hx0-m,sy0-m,(hx1-hx0)+2*m,(ty1-sy0)+2*m,'#2f6fdc',True))   # clear space
for y in (sy1,hy0,hy1,ty0,ty1): L.append('<line x1="%s" x2="%s" y1="%s" y2="%s" stroke="#d33" stroke-dasharray="%s %s"/>'%(f(hx0-m),f(hx1+m),f(y),f(y),f(sw*6),f(sw*4)))
L.append('</g>')
fs=sw*11
def T(x,y,t,anchor='start',col='#c8961e'): return '<text x="%s" y="%s" font-family="Arial" font-size="%s" fill="%s" text-anchor="%s">%s</text>'%(f(x),f(y),f(fs),col,anchor,t)
L.append(T(hx1+sw*6,(sy0+hy1)/2-fs*1.5,'block %.0f x %.0f'%(hx1-hx0,hy1-sy0))); L.append(T(hx1+sw*6,(sy0+hy1)/2,'= 1 : %.3f'%((hx1-hx0)/(hy1-sy0))))
L.append(T(sx0+Ws/2,sy0-sw*8,'symbol %.0f x %.0f (34:26)   headline = phi x symbol width'%(Ws,Hs),'middle'))
L.append(T(hx1+sw*6,(sy1+hy0)/2,'gap A %.0f'%(hy0-sy1)))
L.append(T(hx1+sw*6,(hy1+ty0)/2,'gap B %.0f  (A/B = %.3f)'%(ty0-hy1,(hy0-sy1)/(ty0-hy1))))
L.append(T(hx1+sw*6,(hy0+hy1)/2,'cap %.0f'%(hy1-hy0)))
L.append(T(hx1+sw*6,(ty0+ty1)/2,'tag cap %.0f (cap/tag = %.3f)'%(ty1-ty0,(hy1-hy0)/(ty1-ty0))))
svg2=logo.wrap(tuple(round(float(q),1) for q in vb),defs,'<g>'+sb+'</g>'+logo.wm_body('#ffffff')+'\n'.join(L),bg='#000000')
svg2=svg2.replace('fill="#ffffff" fill-rule="evenodd"','fill="#ffffff" fill-rule="evenodd"')
open(os.path.join(OUT,'Construction_Full_Logo.svg'),'w',encoding='utf-8').write(svg2)
Image.open(io.BytesIO(bytes(resvg_py.svg_to_bytes(svg_string=svg2,width=2400)))).convert('RGB').save(os.path.join(OUT,'Construction_Full_Logo.png'))

R=dict(u=u,W_sym=Ws,H_sym=Hs,logo_W=hx1-hx0,logo_H=ty1-sy0,block_H=hy1-sy0,gapA=hy0-sy1,gapB=ty0-hy1,cap=hy1-hy0,tagcap=ty1-ty0,
       head_over_sym=(hx1-hx0)/Ws,clear=m,tag_W=tx1-tx0)
def js(x):
    if hasattr(x,'tolist'): return x.tolist()
    if isinstance(x,(tuple,list)): return [js(y) for y in x]
    return float(x) if isinstance(x,(np.floating,)) else x
D2={k:js(v) for k,v in D.items() if k!='lines'}
json.dump({'layout_px':R,'design_units':D2},open(os.path.join('work','golden_dims.json'),'w'),indent=1,default=float)
print(json.dumps(R,indent=1,default=float))
