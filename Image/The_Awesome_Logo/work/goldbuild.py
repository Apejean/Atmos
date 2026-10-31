"""Golden-ratio version: constructed symbol (golden_design) + shading and letterforms taken from the traced v1 logo,
laid out with golden-ratio relations between symbol and wordmark."""
import sys
sys.path.insert(0,'work')
import numpy as np
import pipe, tagfix
import golden_design as gd

PHI=gd.PHI

def affine_fit(src,dst):
    src=np.asarray(src,float); dst=np.asarray(dst,float)
    A=np.c_[src,np.ones(len(src))]; M,_,_,_=np.linalg.lstsq(A,dst,rcond=None)   # 3x2
    return lambda p: np.r_[np.asarray(p,float),1.0]@M

_DARK={}
def _dark():
    if not _DARK: _DARK['r']=pipe.run('dark')
    return _DARK['r']

def run(name):
    v,Go,fo,fits_o,wm=pipe.run(name)
    if name=='dark': _DARK['r']=(v,Go,fo,fits_o,wm)
    vd,Gd,_,_,wmd=_dark()
    wm=wmd                                              # one set of letterforms for both colour versions
    D=gd.design()
    H_px=Gd['BLc'][1]-Gd['A_apex'][1]                  # same symbol size for both colour versions (dark source height)
    u=H_px/D['H']
    cx_old=(Gd['BLc'][0]+Gd['BRL'][0])/2
    org=np.array([cx_old-D['W']/2*u, Gd['BLc'][1]-D['H']*u])
    faces_u=gd.faces_units(D)
    faces,xf=gd.to_frame(faces_u,u,org)
    # ---- landmarks old (traced) -> new (constructed) ----
    up_o=Go['up']; ic=int(np.argmin(up_o[:,1]))
    old=dict(TL=Go['TL'],A=Go['A_apex'],inner=Go['inner_top'],BLc=Go['BLc'],BRL=Go['BRL'],BRR=Go['BRR'],
             pa=Go['rleg_a'],lb=Go['rleg_b'],F=Go['low'][0],E=up_o[-1],crest=up_o[ic])
    new=dict(TL=xf(D['TL']),A=xf(D['A']),inner=xf(D['inner']),BLc=xf(D['BLc']),BRL=xf(D['BRL']),BRR=xf(D['BRR']),
             pa=xf(D['pa']),lb=xf(D['lb']),F=xf(D['F']),E=xf(D['E']),crest=xf((D['xc'],D['H']-D['ch'])))
    corr={'leg_left':['TL','A','inner','BLc'],'leg_right':['A','BRL','BRR','inner'],
          'arch_front':['BLc','F','pa','lb','crest'],'arch_right_low':['pa','lb','E','BRL','BRR'],
          'arch_right_band':['pa','lb','E','crest']}
    fits={}
    for k,(l1,l2) in fits_o.items():
        T=affine_fit([old[n] for n in corr[k]],[new[n] for n in corr[k]])
        fits[k]=((T(l1[0]),T(l1[1]),l1[2]),(T(l2[0]),T(l2[1]),l2[2]))
    # ---- wordmark: golden layout ----
    Wsym=D['W']*u; Hsym=D['H']*u; base=Gd['BLc'][1]; cx=org[0]+Wsym/2
    hb=np.array(wm['head']['bbox']); tb=np.array(wm['tag']['bbox'])
    hw=hb[:,1,0].max()-hb[:,0,0].min(); hcap=np.median(hb[:,1,1]-hb[:,0,1]); tcap=np.median(tb[:,1,1]-tb[:,0,1])
    s_h=PHI*Wsym/hw                                   # headline width = phi x symbol width
    cap=hcap*s_h; tcap_new=cap/PHI**2; s_t=tcap_new/tcap          # tagline cap = headline cap / phi^2
    # block (symbol top -> headline bottom) is a golden rectangle: headline width / block height = phi
    #   -> block height = symbol width  ->  gap1 = Wsym - Hsym - cap ; gap(head->tag) = gap1 / phi
    gap1=Wsym-Hsym-cap; gap2=gap1/PHI
    def place(paths,bb,s,ctr_x,top_y):
        x0,x1=bb[:,0,0].min(),bb[:,1,0].max(); y0=np.median(bb[:,0,1])     # cap line (round letters overshoot it)
        tx=ctr_x-(x0+x1)/2*s; ty=top_y-y0*s
        return [tagfix.xform(d,s,s,tx,ty) for d in paths],[[[(b[0][0])*s+tx,(b[0][1])*s+ty],[(b[1][0])*s+tx,(b[1][1])*s+ty]] for b in bb]
    head_top=base+gap1
    hp,hbb=place(wm['head']['paths'],hb,s_h,cx,head_top)
    tag_top=head_top+cap+gap2
    tp,tbb=place(wm['tag']['paths'],tb,s_t,cx,tag_top)
    wm2={'head':{'paths':hp,'bbox':hbb},'tag':{'paths':tp,'bbox':tbb},
         'info':dict(wm['info'],head_scale=s_h,tag_scale=s_t,cap=cap,tag_cap=tcap_new,gap1=gap1,gap2=gap2)}
    G=dict(TL=new['TL'],A_apex=new['A'],inner_top=new['inner'],BLc=new['BLc'],BRL=new['BRL'],BRR=new['BRR'],
           rleg_a=new['pa'],rleg_b=new['lb'],D=D,u=u,org=org,clear=D['H']*u/PHI**3,Wsym=Wsym,Hsym=Hsym)
    return v,G,faces,fits,wm2
