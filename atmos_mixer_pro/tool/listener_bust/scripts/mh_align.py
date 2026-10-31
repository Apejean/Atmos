# std8과 같은 변환(외이도 기준 축척·이동)을 고운 머리 메시에 적용
import sys, gzip, numpy as np
import os
MPFB = os.environ.get('MPFB_DATA', '/tmp/mpfb2/src/mpfb/data')  # MakeHuman(MPFB2) 데이터 폴더
WORK = os.environ.get('BUST_WORK', 'work')  # 중간 산출물 폴더
morph, raw_std8, head = sys.argv[1:4]
V = np.load(morph + '.full.npy')
out = []
for side in 'lr':
    d = {}
    for l in gzip.open(f'{MPFB}/targets/ears/{side}-ear-trans-backward.target.gz', 'rt'):
        p = l.split()
        if len(p) == 4: d[int(p[0])] = np.linalg.norm([float(x) for x in p[1:]])
    m = max(d.values()); out += [k for k, v in d.items() if v > 0.7*m]
E = V[sorted(set(out))]; canal = np.array([0.0, E[:, 1].mean(), E[:, 2].mean()])
A8 = np.load(raw_std8); top = A8[:, 1].max(); s = (3.97 - 2.50)/(top - canal[1])
xoff = 0.5*(A8[:, 0].min() + A8[:, 0].max())
H = np.load(head + '.raw.v.npy'); B = (H - canal)*s + np.array([0, 2.50, -0.076]); B[:, 0] -= xoff*s
np.save(head + '.v.npy', B); print('head bbox', B.min(0).round(3), B.max(0).round(3), 'scale', round(s, 4))
