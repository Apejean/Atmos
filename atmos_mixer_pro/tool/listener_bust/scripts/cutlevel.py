# 외이도와 정수리 비율로 절단 높이(원본 좌표) 계산: 앱 모델의 (외이도-바닥)/(정수리-외이도) = 2.50/1.47
import sys, gzip, glob, json, numpy as np
import os
MPFB = os.environ.get('MPFB_DATA', '/tmp/mpfb2/src/mpfb/data')  # MakeHuman(MPFB2) 데이터 폴더
WORK = os.environ.get('BUST_WORK', 'work')  # 중간 산출물 폴더
morph, cfgp = sys.argv[1], sys.argv[2]
V = np.load(morph + '.full.npy')
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
ey = V[sorted(idx), 1].mean()
B = np.array([[float(t) for t in l.split()[1:4]] for l in open(morph) if l.startswith('v ')])
top = B[:, 1].max()
cfg = json.load(open(cfgp)); cfg['bottom_y'] = float(ey - 2.50/1.47*(top - ey)); json.dump(cfg, open(cfgp, 'w'))
print('ear y', round(ey, 3), 'top', round(top, 3), 'bottom', round(cfg['bottom_y'], 3))
