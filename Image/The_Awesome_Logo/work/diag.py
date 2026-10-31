import sys, io, pickle
sys.path.insert(0,'work')
import numpy as np, cv2
from PIL import Image
import pipe, logo, resvg_py
from pipe import *

def svg_frame(v,faces,fits,wm,txtcol,overlay=True):
    defs,sb=logo.sym_defs_and_body(faces,fits)
    logo.WM={'head':{'paths':wm['head']['paths']},'tag':{'paths':wm['tag']['paths']}}
    body='<g id="symbol">\n'+sb+'\n</g>\n'+logo.wm_body(txtcol)
    bg='#%02x%02x%02x'%v.bg
    return logo.wrap((0,0,FW,FH),defs,body,bg=bg)

if __name__=='__main__':
    for nm,col in (('dark','#ffffff'),('light','#000000')):
        v,G,faces,fits,wm=pipe.run(nm)
        svg=svg_frame(v,faces,fits,wm,col)
        open('work/diag_%s.svg'%nm,'w').write(svg)
        a=np.array(Image.open(io.BytesIO(bytes(resvg_py.svg_to_bytes(svg_string=svg)))).convert('RGB')).astype(float)
        ref=v.frame.astype(float); d=np.abs(a-ref).mean(2)
        y0=int(G['BLc'][1])
        print(nm,'sym diff %.2f'%d[200:y0+10,450:1620].mean(),'text diff %.2f'%d[y0+40:].mean(),'all %.2f'%d.mean())
        Image.fromarray(a.astype(np.uint8)).save('work/diag_%s.png'%nm)
        Image.fromarray(np.clip(d*4,0,255).astype(np.uint8)).save('work/diagdiff_%s.png'%nm)
