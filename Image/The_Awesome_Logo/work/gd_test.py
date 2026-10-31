import sys,io; sys.path.insert(0,'work')
import numpy as np, golden_design as gd
from PIL import Image
import resvg_py
D=gd.design()
for k in ('TL','inner','C1','R1','C2','R2','E','pa','lb','ch','h_hole','t_leg','t_right'):
    v=D[k]; print(k,np.round(v,3).tolist() if hasattr(v,'__len__') else round(float(v),3))
faces=gd.faces_units(D)
order=('leg_right','leg_left','arch_front','arch_right_low','arch_right_band')
body=''.join('<path d="%s" fill="%s" stroke="#000" stroke-width="0.04"/>'%(faces[k].d(),c) for k,c in zip(order,['#bbb','#ddd','#999','#ccc','#555']))
svg='<svg xmlns="http://www.w3.org/2000/svg" viewBox="-2 -2 38 30" width="1140" height="900"><rect x="-2" y="-2" width="38" height="30" fill="#fff"/>'+body+'</svg>'
Image.open(io.BytesIO(bytes(resvg_py.svg_to_bytes(svg_string=svg)))).save('work/gd_test.png')
