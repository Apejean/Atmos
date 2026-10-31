# 최종 메시 → Atmos 규격 OBJ(정점 법선 포함, f a//a 형식, 바닥 y=0 고정)
import sys, numpy as np
src, out = sys.argv[1], sys.argv[2]
P = np.load(src + '.v.npy'); F = np.load(src + '.f.npy')
P[P[:, 1] < 1e-3, 1] = 0.0
e = np.sort(np.concatenate([F[:, [0, 1]], F[:, [1, 2]], F[:, [2, 0]]]), 1); _, cn = np.unique(e, axis=0, return_counts=True)
if (cn != 2).any(): sys.exit(f'메시가 닫혀 있지 않음: 열린 {int((cn == 1).sum())}, 비다양체 {int((cn > 2).sum())}')
fn = np.cross(P[F[:, 1]] - P[F[:, 0]], P[F[:, 2]] - P[F[:, 0]])
N = np.zeros_like(P)
for k in range(3): np.add.at(N, F[:, k], fn)
N /= np.linalg.norm(N, axis=1, keepdims=True) + 1e-12
with open(out, 'w') as f:
    f.write('# Listener bust - smooth lofted head and torso, MakeHuman (CC0) ears, flat capped bottom, pure white material\n')
    f.write('mtllib listener_bust_white.mtl\no listener_bust_white\nusemtl PureWhite\n')
    f.write(''.join(f'v {a:.7f} {b:.7f} {c:.7f}\n' for a, b, c in P))
    f.write(''.join(f'vn {a:.7f} {b:.7f} {c:.7f}\n' for a, b, c in N))
    f.write(''.join(f'f {a+1}//{a+1} {b+1}//{b+1} {c+1}//{c+1}\n' for a, b, c in F))
print('verts', len(P), 'tris', len(F), 'bbox', P.min(0).round(3), P.max(0).round(3))
