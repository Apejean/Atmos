# 외이도·정수리·바닥을 현재 앱 규격에 맞춤: 외이도 (y 2.50, z -0.076), 정수리 3.97, 바닥 0
import sys, gzip, glob, json, numpy as np
import os
MPFB = os.environ.get('MPFB_DATA', '/tmp/mpfb2/src/mpfb/data')  # MakeHuman(MPFB2) 데이터 폴더
WORK = os.environ.get('BUST_WORK', 'work')  # 중간 산출물 폴더
morph, out = sys.argv[1], sys.argv[2]
D = f'{MPFB}/targets/ears'
def ear_idx():
    # 귀 이동 타깃에서 거의 통째로 움직이는 정점 = 귓바퀴
    out = []
    for side in 'lr':
        d = {}
        for l in gzip.open(f'{MPFB}/targets/ears/{side}-ear-trans-backward.target.gz', 'rt'):
            p = l.split()
            if len(p) == 4: d[int(p[0])] = np.linalg.norm([float(x) for x in p[1:]])
        m = max(d.values()); out += [k for k, v in d.items() if v > 0.7*m]
    return set(out)
idx = ear_idx()
V = np.load(morph + '.full.npy')
E = V[sorted(idx)]
canal = np.array([0.0, E[:, 1].mean(), E[:, 2].mean()])
A = np.load(out + '.raw.v.npy'); Fc = np.load(out + '.f.npy')
top = A[:, 1].max()
s = (3.97 - 2.50) / (top - canal[1])
B = (A - canal) * s + np.array([0, 2.50, -0.076])
B[:, 0] -= 0.5*(B[:, 0].min() + B[:, 0].max())
print('bottom ring y range', B[B[:, 1] < 0.01, 1].min(), B[B[:, 1] < 0.01, 1].max())
B[B[:, 1] < 1e-3, 1] = 0.0
print('ear y-span', ((E[:, 1].min()-canal[1])*s+2.5).round(3), ((E[:, 1].max()-canal[1])*s+2.5).round(3), 'bottom', B[:, 1].min().round(4), 'top', B[:, 1].max().round(4))
print('bbox', B.min(0).round(3), B.max(0).round(3), 'verts', len(B), 'tris', len(Fc))
np.save(out + '.v.npy', B)
with open(out, 'w') as f:
    f.write('# Listener bust - MakeHuman CC0 base mesh, bust cut and voxel remesh in Blender, pure white material\n')
    f.write('mtllib listener_bust_white.mtl\no listener_bust_white\nusemtl PureWhite\n')
    f.write(''.join(f'v {a:.7f} {b:.7f} {c:.7f}\n' for a, b, c in B))
    f.write(''.join(f'f {a+1} {b+1} {c+1}\n' for a, b, c in Fc))
