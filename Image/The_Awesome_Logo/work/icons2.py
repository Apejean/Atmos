import io, os
import resvg_py
from PIL import Image, ImageDraw, ImageFilter

ROOT='The_Awesome_Logo_Final'
SVG=open(os.path.join(ROOT,'02_SVG','Symbol_Dark_Transparent.svg'),encoding='utf-8').read()   # dark-variant symbol (silver on black tile)
def sym(width):
    return Image.open(io.BytesIO(bytes(resvg_py.svg_to_bytes(svg_string=SVG,width=width)))).convert('RGBA')

def windows_master(size=1024,sym_frac=0.66):
    """full-bleed square tile, no rounded corners"""
    img=Image.new('RGBA',(size,size),(0,0,0,255))
    w=int(size*sym_frac); s=sym(w)
    img.alpha_composite(s,((size-w)//2,(size-s.size[1])//2+int(size*0.012)))
    return img

def mac_master(size=1024,tile=824,radius=185,sym_frac=0.62):
    """macOS Big Sur-style icon: 824px rounded tile centred on a 1024 canvas (100px margin) + soft drop shadow"""
    S=size/1024.0; t=int(round(tile*S)); r=int(round(radius*S)); m=(size-t)//2
    canvas=Image.new('RGBA',(size,size),(0,0,0,0))
    sh=Image.new('L',(size,size),0); ImageDraw.Draw(sh).rounded_rectangle((m,m+int(12*S),m+t-1,m+t-1+int(12*S)),r,fill=int(255*0.38))
    sh=sh.filter(ImageFilter.GaussianBlur(14*S))
    shadow=Image.new('RGBA',(size,size),(0,0,0,255)); shadow.putalpha(sh); canvas.alpha_composite(shadow)
    tile_img=Image.new('RGBA',(size,size),(0,0,0,0)); ImageDraw.Draw(tile_img).rounded_rectangle((m,m,m+t-1,m+t-1),r,fill=(0,0,0,255))
    canvas.alpha_composite(tile_img)
    w=int(t*sym_frac); s=sym(w)
    canvas.alpha_composite(s,((size-w)//2,(size-s.size[1])//2+int(t*0.012)))
    return canvas

out=os.path.join(ROOT,'04_App_Icons')
win=windows_master(); win.save(os.path.join(out,'AppIcon_Windows_Square_1024.png'))
sizes=[16,24,32,48,64,128,256]
imgs=[win.resize((s,s),Image.LANCZOS) for s in sizes]
imgs[-1].save(os.path.join(out,'Windows.ico'),sizes=[(s,s) for s in sizes],append_images=imgs[:-1])
mac=mac_master(); mac.save(os.path.join(out,'AppIcon_macOS_1024.png'))
mac.save(os.path.join(out,'macOS.icns'),sizes=[(16,16),(32,32),(64,64),(128,128),(256,256),(512,512),(1024,1024)])
for n in ('Windows.ico','macOS.icns'):
    im=Image.open(os.path.join(out,n)); print(n,im.format,sorted(im.ico.sizes()) if n.endswith('ico') else im.info.get('sizes'),os.path.getsize(os.path.join(out,n)))
# preview sheet: Windows (square) next to macOS on a light and a dark background
sheet=Image.new('RGB',(1500,560),(236,236,238))
sheet.paste(win.resize((420,420)),(40,70)); 
bg2=Image.new('RGBA',(520,520),(236,236,238,255)); bg2.alpha_composite(mac.resize((520,520))); sheet.paste(bg2.convert('RGB'),(520,20))
for i,s in enumerate((64,32,16)): sheet.paste(win.resize((s,s),Image.LANCZOS),(1120+i*110,120)); 
for i,s in enumerate((64,32,16)): 
    a=Image.new('RGBA',(s,s),(236,236,238,255)); a.alpha_composite(mac.resize((s,s),Image.LANCZOS)); sheet.paste(a.convert('RGB'),(1120+i*110,260))
sheet.save('work/icon_preview.png')
