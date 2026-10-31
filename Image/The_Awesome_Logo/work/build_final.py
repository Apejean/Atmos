import sys, os, json, shutil, io, re
sys.path.insert(0,'work')
import numpy as np, cv2
from PIL import Image, ImageDraw
import logo
from logo import *
import eps as epsmod
import fitz

ROOT='The_Awesome_Logo_Final'
for d in ('01_Original_Reference','02_SVG','03_PNG','04_App_Icons','05_Illustrator','06_Comparison','07_Documentation'):
    os.makedirs(os.path.join(ROOT,d),exist_ok=True)
def P(*a): return os.path.join(ROOT,*a)

G=geometry(); faces=build(G); fits=fit_all(faces)
defs,sym_body=sym_defs_and_body(faces,fits)
WHITE='#ffffff'; BLACK='#000000'

SYM_TIGHT=(42,43,1070,817); pad=30
SYM_VB=(SYM_TIGHT[0]-pad,SYM_TIGHT[1]-pad,SYM_TIGHT[2]+2*pad,SYM_TIGHT[3]+2*pad)
WM_VB=(-303-40,1030-40,1819+80,(1268-1030)+80)
FULL_VB=logo.FULL_VB

def svgs():
    out={}
    out['Symbol_Only']=wrap(SYM_VB,defs,'<g id="symbol">\n'+sym_body+'\n</g>',title='The Awesome - Symbol')
    out['Wordmark_Tagline']=wrap(WM_VB,'',wm_body(BLACK),title='The Awesome - Wordmark')
    out['Wordmark_Tagline_White']=wrap(WM_VB,'',wm_body(WHITE),title='The Awesome - Wordmark (white)')
    out['Full_Logo_Light']=wrap(FULL_VB,defs,'<g id="symbol">\n'+sym_body+'\n</g>\n'+wm_body(BLACK),title='The Awesome - Full Logo (for light backgrounds)')
    out['Full_Logo_Dark']=wrap(FULL_VB,defs,'<g id="symbol">\n'+sym_body+'\n</g>\n'+wm_body(WHITE),title='The Awesome - Full Logo (for dark backgrounds)')
    return out
S=svgs()
for k,v in S.items(): open(P('02_SVG',k+'.svg'),'w',encoding='utf-8').write(v)

def png(svg,fn,width=None,height=None,bg=None):
    kw={}
    if width: kw['width']=width
    if height: kw['height']=height
    if bg: kw['background']=bg
    b=resvg_py.svg_to_bytes(svg_string=svg,**kw)
    open(fn,'wb').write(bytes(b))

# ---- PNG ----
sym=render(S['Symbol_Only'],width=1024)           # 1024 wide, transparent
canvas=Image.new('RGBA',(1024,1024),(0,0,0,0)); canvas.paste(sym,(0,(1024-sym.size[1])//2),sym)
canvas.save(P('03_PNG','Symbol_Transparent_1024.png'))
png(S['Full_Logo_Light'],P('03_PNG','Full_Logo_Light_Transparent.png'),width=3000)
png(S['Full_Logo_Dark'],P('03_PNG','Full_Logo_Dark_Transparent.png'),width=3000)

# ---- app icons ----
def app_icon(dark,size=1024):
    bgc=(0,0,0,255) if dark else (255,255,255,255)
    img=Image.new('RGBA',(size,size),(0,0,0,0)); dr=ImageDraw.Draw(img)
    r=int(size*0.225)
    dr.rounded_rectangle((0,0,size-1,size-1),r,fill=bgc)
    if not dark:  # thin neutral edge so the white tile is visible on white pages
        dr.rounded_rectangle((0,0,size-1,size-1),r,outline=(225,225,225,255),width=max(2,size//256))
    w=int(size*0.62); s=render(S['Symbol_Only'],width=w)
    img.alpha_composite(s,((size-w)//2,(size-s.size[1])//2+int(size*0.01)))
    return img
ai_d=app_icon(True); ai_l=app_icon(False)
ai_d.save(P('04_App_Icons','AppIcon_Dark_1024.png')); ai_l.save(P('04_App_Icons','AppIcon_Light_1024.png'))
sizes=[16,24,32,48,64,128,256]
ico_imgs=[app_icon(True,s) for s in sizes]
ico_imgs[-1].save(P('04_App_Icons','Windows.ico'),sizes=[(s,s) for s in sizes],append_images=ico_imgs[:-1])
ai_d.save(P('04_App_Icons','macOS.icns'),sizes=[(16,16),(32,32),(64,64),(128,128),(256,256),(512,512),(1024,1024)])

# ---- Illustrator: PDF / EPS / compatible SVG ----
import pdfw
def mk_pdf(fn,col):
    pdfw.write_pdf(fn,FULL_VB,faces,ORDER,fits,[(WM['head']['paths'],col),(WM['tag']['paths'],col)])
mk_pdf(P('05_Illustrator','Full_Logo_Light.pdf'),BLACK)
mk_pdf(P('05_Illustrator','Full_Logo_Dark.pdf'),WHITE)
epsmod.write_eps(P('05_Illustrator','Full_Logo.eps'),FULL_VB,faces,ORDER,fits,WM['head']['paths'],BLACK,WM['tag']['paths'],BLACK)
ill=S['Full_Logo_Light'].replace('<svg xmlns="http://www.w3.org/2000/svg"','<svg version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"',1)
ill='<?xml version="1.0" encoding="UTF-8"?>\n'+ill
open(P('05_Illustrator','Illustrator_Compatible.svg'),'w',encoding='utf-8').write(ill)

# ---- verification of outputs ----
rep={}
for fn in ('Full_Logo_Light.pdf','Full_Logo_Dark.pdf'):
    d=fitz.open(P('05_Illustrator',fn)); pg=d[0]
    rep[fn]={'pages':len(d),'embedded_images':len(pg.get_images()),'size_pt':[round(pg.rect.width,1),round(pg.rect.height,1)],'drawings':len(pg.get_drawings())}
    pix=pg.get_pixmap(dpi=96,alpha=False); pix.save(os.path.join('work','pdf_'+fn.replace('.pdf','.png')))
json.dump(rep,open('work/verify_outputs.json','w'),indent=1)
print(rep)
for fn in ('Windows.ico','macOS.icns'):
    im=Image.open(P('04_App_Icons',fn)); print(fn,im.format,getattr(im,'ico',None) and sorted(im.ico.sizes()) if fn.endswith('ico') else im.info.get('sizes'))
