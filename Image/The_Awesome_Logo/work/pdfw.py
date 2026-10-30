"""Hand-written vector PDF (1.4) writer: axial shadings + luminosity soft masks for the alpha overlay."""
import re
from geom import f

def stitch(stops,comps):
    st=[(0.0,stops[0][1])]+[(max(0,min(1,s)),c) for s,c in stops]+[(1.0,stops[-1][1])]
    clean=[]
    for s,c in st:
        if clean and s<=clean[-1][0]: s=clean[-1][0]+1e-4
        clean.append((s,c))
    fns=[];bounds=[];enc=[]
    for (s0,c0),(s1,c1) in zip(clean,clean[1:]):
        fns.append('<< /FunctionType 2 /Domain [0 1] /C0 [%s] /C1 [%s] /N 1 >>'%(' '.join('%.4f'%v for v in c0),' '.join('%.4f'%v for v in c1)))
        bounds.append(s1); enc+=[0,1]
    return '<< /FunctionType 3 /Domain [0 1] /Functions [%s] /Bounds [%s] /Encode [%s] >>'%(' '.join(fns),' '.join('%.4f'%b for b in bounds[:-1]),' '.join(map(str,enc)))

def path_ops(path):
    o=['%s %s m'%(f(path.start[0]),f(path.start[1]))]
    for k,ps in path.cmds:
        if k=='L': o.append('%s %s l'%(f(ps[0][0]),f(ps[0][1])))
        else: o.append('%s %s %s %s %s %s c'%tuple(f(v) for p in ps for v in p))
    o.append('h'); return '\n'.join(o)

def d_ops(d):
    toks=re.findall(r'[MLCZ]|-?[\d.]+',d); out=[];i=0
    while i<len(toks):
        t=toks[i]
        if t=='M': out.append('%s %s m'%(toks[i+1],toks[i+2])); i+=3
        elif t=='L': out.append('%s %s l'%(toks[i+1],toks[i+2])); i+=3
        elif t=='C': out.append('%s c'%' '.join(toks[i+1:i+7])); i+=7
        elif t=='Z': out.append('h'); i+=1
        else: i+=1
    return '\n'.join(out)

def write_pdf(fn,vb,faces,order,fits,groups,scale=0.4,title='The Awesome Logo',bg=None):
    """groups: list of (paths_d_list, '#rrggbb') drawn after the symbol"""
    x,y,w,h=vb; W=w*scale; H=h*scale
    objs=[]  # list of bytes bodies; object n = index+1
    def add(body):
        objs.append(body if isinstance(body,bytes) else body.encode('latin-1')); return len(objs)
    # reserve 1 catalog,2 pages,3 page,4 content
    add(b''); add(b''); add(b''); add(b'')
    shad={};gs={};content=['q','%.4f 0 0 %.4f %.4f %.4f cm'%(scale,-scale,-x*scale,(y+h)*scale)]
    if bg:   # solid page background (dark / light version)
        r_,g_,b_=[int(bg[i:i+2],16)/255 for i in (1,3,5)]
        content+=['%.4f %.4f %.4f rg'%(r_,g_,b_),'%s %s %s %s re f'%(x,y,w,h)]
    for k in order:
        l1,l2=fits[k]
        p0,p1,st=l1
        fn1=stitch([(s,tuple(v/255 for v in c)) for s,c in st],3)
        sh1='<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [%s %s %s %s] /Function %s /Extend [true true] >>'%(f(p0[0]),f(p0[1]),f(p1[0]),f(p1[1]),fn1)
        n1=add(sh1); shad['S%s'%k]=n1
        content+=['q',path_ops(faces[k]),'W n','/S%s sh'%k,'Q']
        q0,q1,st2=l2
        colf=stitch([(s,tuple(v/255 for v in c)) for s,c,a in st2],3)
        alf=stitch([(s,(a,)) for s,c,a in st2],1)
        shc='<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [%s %s %s %s] /Function %s /Extend [true true] >>'%(f(q0[0]),f(q0[1]),f(q1[0]),f(q1[1]),colf)
        sha='<< /ShadingType 2 /ColorSpace /DeviceGray /Coords [%s %s %s %s] /Function %s /Extend [true true] >>'%(f(q0[0]),f(q0[1]),f(q1[0]),f(q1[1]),alf)
        nc=add(shc); na=add(sha); shad['C%s'%k]=nc
        # soft-mask group: gray gradient inside a transparency group form (same coordinate system as the page CTM)
        bbox=[x-10,y-10,x+w+10,y+h+10]
        grp=('/S%s sh'%('A'+k)).encode()
        form=('<< /Type /XObject /Subtype /Form /BBox [%s] /Group << /S /Transparency /CS /DeviceGray >> /Resources << /Shading << /SA%s %d 0 R >> >> /Length %d >>\nstream\n'%(' '.join(f(v) for v in bbox),k,na,len(('/SA%s sh'%k).encode()))).encode()+('/SA%s sh'%k).encode()+b'\nendstream'
        nf=add(form)
        ngs=add('<< /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G %d 0 R >> >>'%nf); gs['M%s'%k]=ngs
        content+=['q',path_ops(faces[k]),'W n','/M%s gs'%k,'/C%s sh'%k,'Q']
    for paths,col in groups:
        r,g,b=[int(col[i:i+2],16)/255 for i in (1,3,5)]
        content.append('%.4f %.4f %.4f rg'%(r,g,b))
        for d in paths: content+= [d_ops(d),'f*']
    content.append('Q')
    cs='\n'.join(content).encode('latin-1')
    objs[3]=('<< /Length %d >>\nstream\n'%len(cs)).encode()+cs+b'\nendstream'
    res='<< /Shading << %s >> /ExtGState << %s >> >>'%(' '.join('/%s %d 0 R'%(k,v) for k,v in shad.items() if not k.startswith('SA')),' '.join('/%s %d 0 R'%(k,v) for k,v in gs.items()))
    objs[0]=b'<< /Type /Catalog /Pages 2 0 R >>'
    objs[1]=b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>'
    objs[2]=('<< /Type /Page /Parent 2 0 R /MediaBox [0 0 %s %s] /Contents 4 0 R /Resources %s >>'%(f(W),f(H),res)).encode()
    out=bytearray(b'%PDF-1.4\n%\xe2\xe3\xcf\xd3\n'); offs=[]
    for i,b in enumerate(objs):
        offs.append(len(out)); out+=('%d 0 obj\n'%(i+1)).encode()+b+b'\nendobj\n'
    xr=len(out); out+=('xref\n0 %d\n0000000000 65535 f \n'%(len(objs)+1)).encode()
    for o in offs: out+=('%010d 00000 n \n'%o).encode()
    out+=('trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n'%(len(objs)+1,xr)).encode()
    open(fn,'wb').write(bytes(out))
