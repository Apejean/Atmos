# 기준 흉상 OBJ(이전 listener_bust_white.obj)를 읽어 치수 측정용 배열로 저장
import sys, os, numpy as np
src = sys.argv[1]; WORK = os.environ.get('BUST_WORK', 'work')
V, F = [], []
for l in open(src):
    if l.startswith('v '): V.append([float(t) for t in l.split()[1:4]])
    elif l.startswith('f '):
        f = [int(t.split('/')[0]) - 1 for t in l.split()[1:]]
        for k in range(1, len(f) - 1): F.append([f[0], f[k], f[k + 1]])
np.save(f'{WORK}/cur.v.npy', np.array(V)); np.save(f'{WORK}/cur.f.npy', np.array(F))
print('reference verts', len(V), 'tris', len(F))
