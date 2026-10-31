# MakeHuman 흉상(std8)의 귀 주변 머리 표면을 로프트 머리 표면에 맞춰 반경 방향으로 보정(귀는 함께 이동)
import sys, json, numpy as np
from scipy import ndimage as ndi
prof, mhv, mhf, out, cfgp = sys.argv[1:6]; cfg = json.load(open(cfgp))
L = np.load(prof); Y = L[:, 0]; CZ = L[:, 1]; RL = L[:, 2:]; NA = RL.shape[1]
th = np.linspace(0, 2*np.pi, NA, endpoint=False)
V = np.load(mhv); T = np.load(mhf); tri = V[T]
sel = np.where((Y > 1.9) & (Y < 3.45))[0]
RM = np.zeros((len(sel), NA))
def adist(p, q): d = np.abs(p - q) % (2*np.pi); return np.minimum(d, 2*np.pi - d)
for k, i in enumerate(sel):
    y = Y[i]; s = tri[:, :, 1] - y; m = (s.min(1) < 0) & (s.max(1) > 0); tt = tri[m]; ss = s[m]; pts = []
    for a, b in ((0, 1), (1, 2), (2, 0)):
        c = (ss[:, a]*ss[:, b]) < 0; t = ss[c, a]/(ss[c, a]-ss[c, b]); p = tt[c, a] + t[:, None]*(tt[c, b]-tt[c, a]); pts.append(p[:, [0, 2]])
    P = np.concatenate(pts); ang = np.arctan2(P[:, 1]-CZ[i], P[:, 0]) % (2*np.pi); r = np.hypot(P[:, 0], P[:, 1]-CZ[i])
    bins = (ang/(2*np.pi)*NA).astype(int) % NA
    R = np.zeros(NA); np.maximum.at(R, bins, r)
    # 귀 섹터에서 섹터 밖 보간보다 튀어나온 교점(귀)은 버림
    sec = np.minimum(adist(th, -0.25), adist(th, np.pi+0.25)) < np.radians(48)
    ok0 = (R > 0) & ~sec; idx = np.arange(NA); Rfit = np.interp(idx, idx[ok0], R[ok0], period=NA)
    keep = ~(sec[bins] & (r > Rfit[bins] + 0.012)) if 2.1 < y < 3.2 else np.ones(len(r), bool)
    R = np.zeros(NA); np.maximum.at(R, bins[keep], r[keep]); ok = R > 0; R = np.interp(idx, idx[ok], R[ok], period=NA)
    RM[k] = R
RM = ndi.gaussian_filter1d(ndi.gaussian_filter1d(RM, cfg['sth'], axis=1, mode='wrap'), cfg['sy'], axis=0, mode='nearest')
D = RL[sel] - RM
D = np.clip(D, -cfg.get('clip', 0.05), cfg.get('clip', 0.05))
D = ndi.gaussian_filter1d(ndi.gaussian_filter1d(D, cfg.get('dsth', 8), axis=1, mode='wrap'), cfg.get('dsy', 6), axis=0, mode='nearest')
Ys = Y[sel]
# 정점 보정
y = V[:, 1]; m = (y > Ys[0]) & (y < Ys[-1])
czv = np.interp(y[m], Y, CZ); ang = np.arctan2(V[m, 2]-czv, V[m, 0]) % (2*np.pi); r = np.hypot(V[m, 0], V[m, 2]-czv)
fi = np.interp(y[m], Ys, np.arange(len(Ys))); fj = ang/(2*np.pi)*NA
d = ndi.map_coordinates(np.pad(D, ((0, 0), (0, 1)), mode='wrap'), [fi, fj], order=1, mode='nearest')
r2 = r + d
W = V.copy(); W[m, 0] = r2*np.cos(ang); W[m, 2] = czv + r2*np.sin(ang)
print('delta range in ear zone', np.percentile(D[:, :], [5, 50, 95]).round(3))
with open(out, 'w') as f:
    f.write(''.join(f'v {a:.7f} {b:.7f} {c:.7f}\n' for a, b, c in W)); f.write(''.join(f'f {p+1} {q+1} {s_+1}\n' for p, q, s_ in T))
