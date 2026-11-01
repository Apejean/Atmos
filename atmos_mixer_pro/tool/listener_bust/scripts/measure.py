# 지금 흉상의 "치수"만 측정: 높이별 단면을 각도별 최대 반경으로 재고 저차 푸리에로 요약(세부 제거)
import sys, json, numpy as np
import os
MPFB = os.environ.get('MPFB_DATA', '/tmp/mpfb2/src/mpfb/data')  # MakeHuman(MPFB2) 데이터 폴더
WORK = os.environ.get('BUST_WORK', 'work')  # 중간 산출물 폴더
V = np.load(f'{WORK}/cur.v.npy'); T = np.load(f'{WORK}/cur.f.npy')
NA = 384; th = np.linspace(0, 2*np.pi, NA, endpoint=False)
top = V[:, 1].max()
levels = np.concatenate([np.linspace(0.005, 3.6, 360), 3.6 + (top-3.6-0.004)*np.sin(np.linspace(0, np.pi/2, 31)[1:])])
tri = V[T]
# 귀 삼각형 제외: 귀 높이에서 머리 옆면(반폭 ≈ 0.80→0.865)보다 바깥
cen = tri.mean(1)
ear = (cen[:, 1] > 2.08) & (cen[:, 1] < 2.92) & (np.abs(cen[:, 0]) > 0.79 + (cen[:, 1]-2.08)*0.08) & (cen[:, 2] < 0.35)
tri = tri[~ear] if False else tri  # (제외 대신 build 단계에서 메움)
print('ear tris removed', int(ear.sum()))
out = []
for y in levels:
    s = (tri[:, :, 1] - y)
    m = (s.min(1) < 0) & (s.max(1) > 0)
    tt = tri[m]; ss = s[m]
    pts = []
    for a, b in ((0, 1), (1, 2), (2, 0)):
        c = (ss[:, a] * ss[:, b]) < 0
        t = ss[c, a] / (ss[c, a] - ss[c, b])
        p = tt[c, a] + t[:, None]*(tt[c, b] - tt[c, a]); pts.append(p[:, [0, 2]])
    P = np.concatenate(pts)
    zmn, zmx = P[:, 1].min(), P[:, 1].max()
    fr = 0.5 + 0.14*np.clip((y - 1.25)/0.25, 0, 1)  # 머리 구간은 중심을 얼굴 쪽으로
    cx, cz = 0.0, zmn + fr*(zmx - zmn)
    ang = np.arctan2(P[:, 1]-cz, P[:, 0]-cx) % (2*np.pi); r = np.hypot(P[:, 0]-cx, P[:, 1]-cz)
    bins = (ang/(2*np.pi)*NA).astype(int) % NA
    R = np.zeros(NA); np.maximum.at(R, bins, r)
    if 2.08 < y < 2.95:
        def adist(p, q): d = np.abs(p - q) % (2*np.pi); return np.minimum(d, 2*np.pi - d)
        thb = (np.arange(NA)+0.5)/NA*2*np.pi
        sec = (np.minimum(adist(thb, -0.17), adist(thb, np.pi+0.17)) < np.radians(42))
        ok0 = (R > 0) & ~sec; idx = np.arange(NA)
        Rfit = np.interp(idx, idx[ok0], R[ok0], period=NA)
        psec = sec[bins]; keep = ~(psec & (r > Rfit[bins] + 0.012))
        R = np.zeros(NA); np.maximum.at(R, bins[keep], r[keep])
    # 빈 칸 보간
    ok = R > 0; idx = np.arange(NA)
    R = np.interp(idx, idx[ok], R[ok], period=NA)
    out.append([y, cz] + R.tolist())
out = np.array(out)
np.save(f'{WORK}/raw_profile.npy', out)
print(out.shape)
