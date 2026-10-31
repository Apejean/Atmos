"""Extra deliverables: one-colour (mono) logos, light symbol PNG, wordmark PNGs, symbol PDFs, web favicons."""
import sys, os, io
sys.path.insert(0,'work')
import numpy as np
from PIL import Image
import resvg_py
from shapely.geometry import Polygon
from shapely.ops import unary_union
import goldbuild, logo, pdfw, tagfix
import golden_design as gd

ROOT='The_Awesome_Logo_Final'; P=lambda *a: os.path.join(ROOT,*a)
for d in ('02_SVG/Mono','03_PNG/Mono','05_Illustrator','04_App_Icons/Web'): os.makedirs(P(d),exist_ok=True)
PHI=gd.PHI
def rnd(vb): return tuple(round(float(x),1) for x in vb)
def rend(svg,width=None,height=None):
    kw={}
    if width: kw['width']=width
    if height: kw['height']=height
    return Image.open(io.BytesIO(bytes(resvg_py.svg_to_bytes(svg_string=svg,**kw)))).convert('RGBA')

V={}
for nm in ('dark','light'):
    v,G,faces,fits,wm=goldbuild.run(nm)
    allp=np.vstack([faces[k].poly(48) for k in faces]); sb=(allp.min(0),allp.max(0))
    hb=np.array(wm['head']['bbox']); tb=np.array(wm['tag']['bbox'])
    m=G['clear']
    tx0=min(hb[:,0,0].min(),tb[:,0,0].min()); tx1=max(hb[:,1,0].max(),tb[:,1,0].max())
    V[nm]=dict(G=G,faces=faces,fits=fits,wm=wm,sb=sb,m=m,
               full=(min(sb[0][0],tx0)-m,sb[0][1]-m,max(sb[1][0],tx1)-min(sb[0][0],tx0)+2*m,tb[:,1,1].max()-sb[0][1]+2*m),
               symvb=(sb[0][0]-m/PHI,sb[0][1]-m/PHI,sb[1][0]-sb[0][0]+2*m/PHI,sb[1][1]-sb[0][1]+2*m/PHI),
               wmvb=(tx0-m/PHI,hb[:,0,1].min()-m/PHI,tx1-tx0+2*m/PHI,tb[:,1,1].max()-hb[:,0,1].min()+2*m/PHI))
D=V['dark']
def wmbody(color,nm='dark'):
    w=V[nm]['wm']; logo.WM={'head':{'paths':w['head']['paths']},'tag':{'paths':w['tag']['paths']}}; return logo.wm_body(color)

# ---------- 1. one-colour symbol: ribbons separated by a thin knock-out gap ----------
f=D['faces']; u=D['G']['u']
poly=lambda k: Polygon(f[k].poly(64)).buffer(0)
right=unary_union([poly('leg_right'),poly('arch_right_low')])            # right leg (back)
left=poly('leg_left')
arch=unary_union([poly('arch_front'),poly('arch_right_band')])           # arch ribbon (front)
gap=D['G']['D']['w']/PHI**5*u                                           # knock-out gap = leg width / phi^5
vis_right=right.difference(unary_union([left,arch]).buffer(gap,join_style='mitre'))
vis_left=left.difference(arch.buffer(gap,join_style='mitre'))
silhouette=unary_union([right,left,arch])
vis_left=vis_left.intersection(silhouette); vis_right=vis_right.intersection(silhouette)
mono_d=[tagfix.geom_to_d(g,err=0.35) for g in (vis_right,vis_left,arch) if not g.is_empty]
def mono_sym_body(col):
    return '<g id="symbol_mono" fill="%s" fill-rule="evenodd">\n'%col+'\n'.join('<path id="%s" d="%s"/>'%(n,d) for n,d in zip(('leg_right','leg_left','arch'),mono_d))+'\n</g>'
for col,tag,bg in (('#000000','Black','#ffffff'),('#ffffff','White','#000000')):
    # with background (black logo on white / white logo on black) and without (transparent)
    for suf,bgc in (('',bg),('_Transparent',None)):
        s=logo.wrap(rnd(D['symvb']),'',mono_sym_body(col),title='The Awesome - Symbol (one colour)',bg=bgc)
        open(P('02_SVG','Mono','Symbol_Mono_%s%s.svg'%(tag,suf)),'w',encoding='utf-8').write(s)
        fsv=logo.wrap(rnd(D['full']),'',mono_sym_body(col)+'\n'+wmbody(col),title='The Awesome - Full Logo (one colour)',bg=bgc)
        open(P('02_SVG','Mono','Full_Logo_Mono_%s%s.svg'%(tag,suf)),'w',encoding='utf-8').write(fsv)
        rend(fsv,width=3000).save(P('03_PNG','Mono','Full_Logo_Mono_%s%s.png'%(tag,suf)))
        sym=rend(logo.wrap(rnd(D['symvb']),'',mono_sym_body(col)),width=820)
        cv=Image.new('RGBA',(1024,1024),(0,0,0,0) if bgc is None else tuple(int(bgc[i:i+2],16) for i in (1,3,5))+(255,))
        cv.alpha_composite(sym,((1024-sym.size[0])//2,(1024-sym.size[1])//2)); cv.save(P('03_PNG','Mono','Symbol_Mono_%s%s_1024.png'%(tag,suf)))
        pdfw.write_pdf(P('05_Illustrator','Full_Logo_Mono_%s%s.pdf'%(tag,suf)),rnd(D['full']),{},[],{},
                       [(mono_d,col),(D['wm']['head']['paths'],col),(D['wm']['tag']['paths'],col)],bg=bgc)

# ---------- 3. web favicons (from the full-bleed dark square icon) ----------
sq=Image.open(P('04_App_Icons','AppIcon_Windows_Square_1024.png')).convert('RGBA')
web=P('04_App_Icons','Web')
fav=[sq.resize((s,s),Image.LANCZOS) for s in (16,32,48)]
fav[-1].save(os.path.join(web,'favicon.ico'),sizes=[(16,16),(32,32),(48,48)],append_images=fav[:-1])
for s,n in ((16,'favicon-16x16.png'),(32,'favicon-32x32.png'),(180,'apple-touch-icon.png'),(192,'android-chrome-192x192.png'),(512,'android-chrome-512x512.png')):
    sq.resize((s,s),Image.LANCZOS).convert('RGB').save(os.path.join(web,n))
# scalable favicon: the dark symbol on a full-bleed black square
dd=V['dark']; defs,sb=logo.sym_defs_and_body(dd['faces'],dd['fits'])
x,y,w_,h_=dd['symvb']; side=max(w_,h_)/0.66; cx,cy=x+w_/2,y+h_/2
open(os.path.join(web,'favicon.svg'),'w',encoding='utf-8').write(logo.wrap(rnd((cx-side/2,cy-side/2,side,side)),defs,'<g>'+sb+'</g>',bg='#000000',title='The Awesome'))
open(os.path.join(web,'site.webmanifest'),'w',encoding='utf-8').write('{"name":"The Awesome","short_name":"The Awesome","icons":[{"src":"android-chrome-192x192.png","sizes":"192x192","type":"image/png"},{"src":"android-chrome-512x512.png","sizes":"512x512","type":"image/png"}],"theme_color":"#000000","background_color":"#000000","display":"standalone"}\n')
open(os.path.join(web,'head_snippet.html'),'w',encoding='utf-8').write(
'<link rel="icon" href="/favicon.ico" sizes="48x48">\n<link rel="icon" href="/favicon.svg" type="image/svg+xml">\n'
'<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32x32.png">\n<link rel="icon" type="image/png" sizes="16x16" href="/favicon-16x16.png">\n'
'<link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png">\n<link rel="manifest" href="/site.webmanifest">\n')
print('gap',gap,'mono paths',[len(x) for x in mono_d])
