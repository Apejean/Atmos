"""Minimal vector EPS (PostScript level 3, axial shadings) writer for the logo."""
import numpy as np
from geom import f

def seg_ps(path):
    o=[f(path.start[0])+' '+f(path.start[1])+' moveto']
    for k,ps in path.cmds:
        if k=='L': o.append('%s %s lineto'%(f(ps[0][0]),f(ps[0][1])))
        else: o.append('%s %s %s %s %s %s curveto'%tuple(f(v) for p in ps for v in p))
    o.append('closepath'); return '\n'.join(o)

def fn_stitch(stops):
    # stops: [(offset,(r,g,b))]  offsets ascending 0..1
    st=[(0.0,stops[0][1])]+[(max(0,min(1,s)),c) for s,c in stops]+[(1.0,stops[-1][1])]
    # dedupe/ascending
    clean=[]
    for s,c in st:
        if clean and s<=clean[-1][0]: s=clean[-1][0]+1e-4
        clean.append((s,c))
    fns=[];bounds=[];enc=[]
    for (s0,c0),(s1,c1) in zip(clean,clean[1:]):
        fns.append('<< /FunctionType 2 /Domain [0 1] /C0 [%s] /C1 [%s] /N 1 >>'%(' '.join('%.4f'%(v/255) for v in c0),' '.join('%.4f'%(v/255) for v in c1)))
        bounds.append(s1); enc+= [0,1]
    bounds=bounds[:-1]
    return '<< /FunctionType 3 /Domain [0 1] /Functions [%s] /Bounds [%s] /Encode [%s] >>'%(' '.join(fns),' '.join('%.4f'%b for b in bounds),' '.join(str(e) for e in enc))

def write_eps(fn,vb,faces,order,fits,wm_paths,color,tag_paths=None,tag_color=None):
    x,y,w,h=vb
    o=['%!PS-Adobe-3.0 EPSF-3.0','%%BoundingBox: 0 0 '+str(int(round(w/2)))+' '+str(int(round(h/2))),
       '%%HiResBoundingBox: 0 0 %s %s'%(f(w/2),f(h/2)),'%%Title: The Awesome Logo','%%Creator: logo reconstruction script','%%EndComments','gsave',
       '0.5 0.5 scale','0 %s translate 1 -1 scale'%f(h),'%s %s translate'%(f(-x),f(-y))]
    for k in order:
        p0,p1,st=fits[k][0]
        o.append('gsave'); o.append(seg_ps(faces[k])); o.append('clip newpath')
        o.append('<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [%s %s %s %s] /Function %s /Extend [true true] >> shfill'%(f(p0[0]),f(p0[1]),f(p1[0]),f(p1[1]),fn_stitch(st)))
        o.append('grestore')
    def txt(paths,col):
        r,g,b=[int(col[i:i+2],16)/255 for i in (1,3,5)]
        o.append('%.4f %.4f %.4f setrgbcolor'%(r,g,b))
        for d in paths: o.append(d)
    def ps_of_d(d):
        import re
        toks=re.findall(r'[MLCZ]|-?[\d.]+',d); out=[];i=0
        while i<len(toks):
            t=toks[i]
            if t=='M': out.append('%s %s moveto'%(toks[i+1],toks[i+2])); i+=3
            elif t=='L': out.append('%s %s lineto'%(toks[i+1],toks[i+2])); i+=3
            elif t=='C': out.append('%s curveto'%' '.join(toks[i+1:i+7])); i+=7
            elif t=='Z': out.append('closepath'); i+=1
            else: i+=1
        return '\n'.join(out)
    for paths,col in ((wm_paths,color),(tag_paths or [],tag_color or color)):
        if not paths: continue
        r,g,b=[int(col[i:i+2],16)/255 for i in (1,3,5)]
        o.append('%.4f %.4f %.4f setrgbcolor'%(r,g,b))
        for d in paths:
            o.append('newpath'); o.append(ps_of_d(d)); o.append('eofill')
    o+=['grestore','showpage','%%EOF']
    open(fn,'w',newline='\n').write('\n'.join(o)+'\n')
