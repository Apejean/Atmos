# 측정 치수 → 매끈한 로프트 흉상(새 형상) + 단순 타원 귀
import sys, json, numpy as np
import os
MPFB = os.environ.get('MPFB_DATA', '/tmp/mpfb2/src/mpfb/data')  # MakeHuman(MPFB2) 데이터 폴더
WORK = os.environ.get('BUST_WORK', 'work')  # 중간 산출물 폴더
from scipy import ndimage as ndi
cfg = json.load(open(sys.argv[1])); out = sys.argv[2]
D = np.load(f'{WORK}/raw_profile.npy')
# 균일한 높이 간격으로 재샘플(행 간격이 바뀌는 곳의 꺾임 방지)
Yu = np.arange(D[0, 0], D[-1, 0], cfg.get('dy', 0.01)); Yu = np.append(Yu, D[-1, 0])
Y = Yu; CZ = np.interp(Yu, D[:, 0], D[:, 1]); R = np.stack([np.interp(Yu, D[:, 0], D[:, 2+j]) for j in range(D.shape[1]-2)], 1)
NA = R.shape[1]; th = np.linspace(0, 2*np.pi, NA, endpoint=False)
# 1) 귀 띠 제거: ±x 방향 각도에서 귀 위·아래 행 사이 보간
e0, e1 = cfg['ear_band']; aw = np.radians(cfg['ear_ang']); tw = np.radians(cfg.get('ear_taper', 15))
off = np.radians(cfg.get('ear_back_deg', 0))  # 귀는 머리 옆에서 약간 뒤쪽(-z)
cR = -off; cL = np.pi + off
def adist(a, b): d = np.abs(a - b) % (2*np.pi); return np.minimum(d, 2*np.pi - d)
dth = np.minimum(adist(th, cR), adist(th, cL))
def ss(t): t = np.clip(t, 0, 1); return t*t*(3-2*t)
wth = 1 - ss((dth - aw)/tw)
pad = cfg.get('ear_pad', 0.12)
i0 = np.searchsorted(Y, e0 - pad) - 1; i1 = np.searchsorted(Y, e1 + pad) + 1
for i in range(i0+1, i1):
    t = ss((Y[i]-Y[i0])/(Y[i1]-Y[i0]))
    interp = (1-t)*R[i0] + t*R[i1]
    wy = ss((Y[i]-(e0-pad))/pad) * ss(((e1+pad)-Y[i])/pad)
    w = wth*wy*ss((R[i] - interp)/cfg.get('ear_prot', 0.05))  # 실제로 튀어나온 곳만
    R[i] = (1-w)*R[i] + w*np.minimum(interp, R[i])  # 귀 돌출만 제거(머리를 더 키우지 않음)
# 1b) 귀 자리 메우기: 정규화 컨볼루션(주변 머리 표면에서 부드럽게 채움)
if cfg.get('fill', False):
    off = np.radians(cfg['fill_back']); cR, cL = -off, np.pi + off
    def adist(p, q): d = np.abs(p - q) % (2*np.pi); return np.minimum(d, 2*np.pi - d)
    def ss(t): t = np.clip(t, 0, 1); return t*t*(3-2*t)
    da = np.minimum(adist(th, cR), adist(th, cL))
    ma = 1 - ss((da - np.radians(cfg['fill_ang']))/np.radians(cfg['fill_taper']))
    y0, y1, pd = cfg['fill_y'][0], cfg['fill_y'][1], cfg['fill_pad']
    my = ss((Y - (y0 - pd))/pd) * ss(((y1 + pd) - Y)/pd)
    M = my[:, None]*ma[None, :]; Hd = (M > 0.02).astype(float)
    # 행 간격이 고르지 않으니 y 시그마는 행 단위 근사
    sy_, sa_ = cfg['fill_sy'], cfg['fill_sa']
    num = ndi.gaussian_filter1d(ndi.gaussian_filter1d(R*(1-Hd), sa_, axis=1, mode='wrap'), sy_, axis=0, mode='nearest')
    den = ndi.gaussian_filter1d(ndi.gaussian_filter1d(1-Hd, sa_, axis=1, mode='wrap'), sy_, axis=0, mode='nearest')
    Rf = num/np.maximum(den, 1e-6)
    R = (1-M)*R + M*Rf
# 1c) 귀 돌출만 제거: 각도 방향 회색 열림(좁은 돌출 제거, 넓은 머리 형태 유지). 차이가 0인 곳은 그대로라 경계 자국이 없음
if cfg.get('open', False):
    def adist(p, q): d = np.abs(p - q) % (2*np.pi); return np.minimum(d, 2*np.pi - d)
    def ss(t): t = np.clip(t, 0, 1); return t*t*(3-2*t)
    da = np.minimum(adist(th, 0), adist(th, np.pi))
    wa = 1 - ss((da - np.radians(cfg['open_ang']))/np.radians(20))
    y0, y1, pd = cfg['open_y'][0], cfg['open_y'][1], 0.12
    for i in range(len(Y)):
        wy = ss((Y[i]-(y0-pd))/pd) * ss(((y1+pd)-Y[i])/pd)
        if wy <= 0: continue
        Ro = ndi.grey_opening(R[i], size=cfg['open_win'], mode='wrap')
        R[i] = R[i] - wy*wa*(R[i]-Ro)
# 1d) 귀 자리 메우기: 귀 섹터 안을 양쪽 가장자리 값·기울기로 잇는 3차 곡선(머리 곡률 유지)으로 대체, 튀어나온 부분만 깎음
if cfg.get('ear_spline', False):
    def ss_(t): t = np.clip(t, 0, 1); return t*t*(3-2*t)
    y0, y1, pd = cfg['es_y'][0], cfg['es_y'][1], cfg['es_pad']
    hw = int(round(np.radians(cfg['es_hw'])/(2*np.pi)*NA)); off = np.radians(cfg['es_back'])
    for cen in (-off, np.pi + off):
        c = int(round((cen % (2*np.pi))/(2*np.pi)*NA))
        ia, ib = c - hw, c + hw
        idx = np.arange(ia, ib+1) % NA; t = np.linspace(0, 1, len(idx)); L = len(idx)-1
        for i in range(len(Y)):
            wy = ss_((Y[i]-(y0-pd))/pd) * ss_(((y1+pd)-Y[i])/pd)
            if wy <= 0: continue
            r = R[i]; p0 = r[ia % NA]; p1 = r[ib % NA]
            m0 = (r[ia % NA] - r[(ia-3) % NA])/3*L; m1 = (r[(ib+3) % NA] - r[ib % NA])/3*L
            h00 = 2*t**3-3*t**2+1; h10 = t**3-2*t**2+t; h01 = -2*t**3+3*t**2; h11 = t**3-t**2
            fit = h00*p0 + h10*m0 + h01*p1 + h11*m1
            cur = r[idx]; R[i, idx] = cur - wy*np.maximum(cur - fit, 0)
# 2) 각도 방향 저차 푸리에(세부 제거) + 좌우 대칭
R = 0.5*(R + R[:, (np.pi - th) % (2*np.pi) * NA/(2*np.pi) .astype(int) % NA]) if False else R
mir = np.round(((np.pi - th) % (2*np.pi)) / (2*np.pi) * NA).astype(int) % NA
R = 0.5*(R + R[:, mir])
Rraw = R.copy()
# 각도·높이 2D 가우시안(각도는 순환) — 세부를 흐리게
sy = np.where(Y > cfg['head_from'], cfg['sy_head'], cfg['sy_body'])
Rb = ndi.gaussian_filter1d(R, cfg['sth'], axis=1, mode='wrap')
R = np.stack([ndi.gaussian_filter1d(Rb, s_, axis=0, mode='nearest')[i] for i, s_ in enumerate(sy)]) if False else Rb
R1 = ndi.gaussian_filter1d(Rb, cfg['sy_body'], axis=0, mode='nearest'); R2 = ndi.gaussian_filter1d(Rb, cfg['sy_head'], axis=0, mode='nearest')
w = np.clip((Y - cfg['head_from'])/0.5, 0, 1); w = (w*w*(3-2*w))[:, None]; R = (1-w)*R1 + w*R2
# 정수리 근처는 행 간격이 촘촘하므로 높이 스무딩을 줄임(정수리 띠 자국 방지)
wt = np.clip((Y - 3.3)/0.45, 0, 1); wt = (wt*wt*(3-2*wt))[:, None]
R3 = ndi.gaussian_filter1d(Rb, 1.0, axis=0, mode='nearest'); R = (1-wt)*R + wt*R3
CZ = ndi.gaussian_filter1d(CZ, cfg['sy_head'], mode='nearest')
# 얼굴 정면(+z, θ=90°) 쪽은 흐림을 약하게 — 이목구비 윤곽 유지, 뒤통수는 매끈하게
if cfg.get('face_sth'):
    Rf = ndi.gaussian_filter1d(R0 := ndi.gaussian_filter1d(Rraw, cfg['face_sth'], axis=1, mode='wrap'), cfg['face_sy'], axis=0, mode='nearest')
    d = np.abs(((th - np.pi/2) + np.pi) % (2*np.pi) - np.pi)
    t = np.clip((d - np.radians(55))/np.radians(25), 0, 1); wf = 1 - t*t*(3-2*t)
    yy = np.clip((Y - 1.3)/0.2, 0, 1)*np.clip((3.3 - Y)/0.2, 0, 1)
    W = yy[:, None]*wf[None, :]
    R = (1-W)*R + W*Rf
np.save(out + '.prof.npy', np.column_stack([Y, CZ, R]))
# 4) 고해상도 링으로 재샘플
NO = cfg['n_ang']; tho = np.linspace(0, 2*np.pi, NO, endpoint=False)
Ro = np.stack([np.interp(tho, np.append(th, 2*np.pi), np.append(r, r[0])) for r in R])
top = 3.97
yl = np.concatenate([np.linspace(0, Y[-1], cfg['n_lev'])])
Rl = np.stack([np.interp(yl, Y, Ro[:, j]) for j in range(NO)], 1); CZl = np.interp(yl, Y, CZ)
Rl[0] = Ro[0]; yl[0] = 0.0
Vs = []; 
for i, y in enumerate(yl):
    Vs.append(np.stack([Rl[i]*np.cos(tho), np.full(NO, y), CZl[i] + Rl[i]*np.sin(tho)], 1))
V = np.concatenate(Vs)
# 정수리 극점: 마지막 링 위를 구면 캡으로 닫음
last = Vs[-1]; rr = Rl[-1].mean(); h = top - yl[-1]
caps = []
for k in range(1, 1):
    f = np.cos(np.pi/2*k/6); caps.append(np.stack([last[:, 0]*f, np.full(NO, yl[-1] + h*np.sin(np.pi/2*k/6)), CZl[-1] + (last[:, 2]-CZl[-1])*f], 1))
V = np.concatenate([V] + caps + [[[0, top, CZl[-1]]], [[0, 0, CZl[0]]]])
nl = len(yl); F = []
for i in range(nl-1):
    for j in range(NO):
        a = i*NO + j; b = i*NO + (j+1) % NO; c = (i+1)*NO + (j+1) % NO; d = (i+1)*NO + j
        F += [[a, b, c], [a, c, d]]
tp = len(V)-2; bp = len(V)-1
for j in range(NO):
    F.append([(nl-1)*NO + j, (nl-1)*NO + (j+1) % NO, tp]); F.append([bp, (j+1) % NO, j])
F = np.array(F)
# 바깥 법선 확인
a, b, c = V[F[:, 0]], V[F[:, 1]], V[F[:, 2]]
if np.einsum('ij,ij->i', a, np.cross(b, c)).sum() < 0: F = F[:, ::-1]
np.save(out + '.v.npy', V); np.save(out + '.f.npy', F)
with open(out, 'w') as f:
    f.write(''.join(f'v {x:.7f} {y:.7f} {z:.7f}\n' for x, y, z in V)); f.write(''.join(f'f {p+1} {q+1} {r+1}\n' for p, q, r in F))
print('body verts', len(V), 'tris', len(F))
