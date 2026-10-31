import sys, json, io
sys.path.insert(0,'work')
import numpy as np, cv2
from PIL import Image
import sym2
from sym2 import *

WM=json.load(open('work/wordmark.json'))
SYM_VB=(42,43,1070,817)
# wordmark extents in "big" coords (panel px -> (p-(160,85))*4)
HEAD_BB=(-303.0,1030.0,1516.0,1130.0)
FULL_VB=(-453,-107,2119,1525)

def sym_defs_and_body(faces,fits,overlay=True):
    d=[];b=[]
    for k,(l1,l2) in fits.items():
        p0,p1,st=l1
        d.append('<linearGradient id="g_%s" gradientUnits="userSpaceOnUse" x1="%s" y1="%s" x2="%s" y2="%s">'%(k,f(p0[0]),f(p0[1]),f(p1[0]),f(p1[1])))
        for s,c in st: d.append('<stop offset="%.3f" stop-color="#%02x%02x%02x"/>'%(s,*[int(round(v)) for v in c]))
        d.append('</linearGradient>')
        p0,p1,st=l2
        d.append('<linearGradient id="h_%s" gradientUnits="userSpaceOnUse" x1="%s" y1="%s" x2="%s" y2="%s">'%(k,f(p0[0]),f(p0[1]),f(p1[0]),f(p1[1])))
        for s,c,a in st: d.append('<stop offset="%.3f" stop-color="#%02x%02x%02x" stop-opacity="%.3f"/>'%(s,*c,a))
        d.append('</linearGradient>')
    for k in ORDER:
        dd=faces[k].d()
        b.append('<g id="%s"><path d="%s" fill="url(#g_%s)"/><path d="%s" fill="url(#h_%s)"/></g>'%(k,dd,k,dd,k))
    return '\n'.join(d),'\n'.join(b)

def wm_body(color,tag_color=None,with_tag=True):
    tag_color=tag_color or color
    o=['<g id="wordmark" fill="%s" fill-rule="evenodd">'%color]
    for i,p in enumerate(WM['head']['paths']): o.append('<path id="THE_AWESOME_%d" d="%s"/>'%(i+1,p))
    o.append('</g>')
    if with_tag:
        o.append('<g id="tagline" fill="%s" fill-rule="evenodd">'%tag_color)
        for i,p in enumerate(WM['tag']['paths']): o.append('<path id="TECHNOLOGY_ARTISTRY_%d" d="%s"/>'%(i+1,p))
        o.append('</g>')
    return '\n'.join(o)

def wrap(vb,defs,body,bg=None,size=None,title='The Awesome'):
    w,h=size if size else (vb[2],vb[3])
    s='<svg xmlns="http://www.w3.org/2000/svg" viewBox="%s %s %s %s" width="%s" height="%s">\n<title>%s</title>\n'%(vb+(w,h,title))
    if defs: s+='<defs>\n%s\n</defs>\n'%defs
    if bg: s+='<rect x="%s" y="%s" width="%s" height="%s" fill="%s"/>\n'%(vb+(bg,))
    return s+body+'\n</svg>\n'

def render(svg,width=None):
    kw={'width':width} if width else {}
    return Image.open(io.BytesIO(bytes(resvg_py.svg_to_bytes(svg_string=svg,**kw)))).convert('RGBA')

def symbol_fits(panel_png,bbox_map=None):
    pass

if __name__=='__main__':
    G=geometry(); faces=build(G); fits=fit_all(faces)
    defs,sb=sym_defs_and_body(faces,fits)
    svg=wrap(FULL_VB,defs,sb+'\n'+wm_body('#ffffff'),bg='#000000')
    open('work/full_dark_test.svg','w').write(svg)
    # compare at panel scale: panel (0,0,600,460) -> big x -640..1760 , y -340..1500
    vb=(-640,-340,2400,1840)
    svg2=wrap(vb,defs,sb+'\n'+wm_body('#ffffff'),bg='#000000')
    a=np.array(render(svg2,width=600).convert('RGB')).astype(float)
    ref=cv2.imread('work/dark_panel.png')[:,:,::-1].astype(float)
    print(a.shape,ref.shape)
    d=np.abs(a-ref).mean(2); d[:70,:200]=0
    print('panel-scale mean abs diff %.2f'%d.mean())
    Image.fromarray(a.astype(np.uint8)).save('work/full_render.png')
    Image.fromarray(np.clip(d*4,0,255).astype(np.uint8)).save('work/full_diff.png')
