# 시험용: MakeHuman 기본 메시(CC0)에 타깃(CC0)을 가중합해 남성 흉상 원형 생성
import sys, gzip, json, numpy as np
import os
MPFB = os.environ.get('MPFB_DATA', '/tmp/mpfb2/src/mpfb/data')  # MakeHuman(MPFB2) 데이터 폴더
WORK = os.environ.get('BUST_WORK', 'work')  # 중간 산출물 폴더
D = MPFB
cfg = json.load(open(sys.argv[1])); out = sys.argv[2]
V = []; F = []; G = []; g = None
for l in open(f'{D}/3dobjs/base.obj'):
    if l.startswith('v '): V.append([float(t) for t in l.split()[1:4]])
    elif l.startswith('g '): g = l.split()[1]
    elif l.startswith('f '):
        F.append([int(t.split('/')[0])-1 for t in l.split()[1:]]); G.append(g)
V = np.array(V)
def add(name, w):
    if abs(w) < 1e-6: return
    for l in gzip.open(f'{D}/targets/{name}.target.gz', 'rt'):
        p = l.split()
        if len(p) == 4 and not l.startswith('#'):
            V[int(p[0])] += w*np.array([float(p[1]), float(p[2]), float(p[3])])
# 매크로: 남성·젊음, 근육/체중 보간
m, wt = cfg['muscle'], cfg['weight']
def split(v, lo, mid, hi):  # 0..1 → (이름, 가중치) 목록
    return [(mid, 1-(v-0.5)*2), (hi, (v-0.5)*2)] if v >= 0.5 else [(lo, (0.5-v)*2), (mid, 1-(0.5-v)*2)]
add('macrodetails/caucasian-male-young', 1.0)
for mn, mw in split(m, 'minmuscle', 'averagemuscle', 'maxmuscle'):
    for wn, ww in split(wt, 'minweight', 'averageweight', 'maxweight'):
        add(f'macrodetails/universal-male-young-{mn}-{wn}', mw*ww)
        if cfg.get('proportions', 0) > 0:
            add(f'macrodetails/proportions/male-young-{mn}-{wn}-idealproportions', mw*ww*cfg['proportions'])
for k, w in cfg.get('targets', {}).items():
    if k.startswith('sym:'):  # 좌우 대칭 타깃
        add(k[4:].replace('{s}', 'l'), w); add(k[4:].replace('{s}', 'r'), w)
    else: add(k, w)
# 팔 내리기: 어깨 관절 기준 Z축 회전, x 방향 부드러운 가중치(간이 스키닝)
ang = np.radians(cfg.get('arm_down_deg', 0))
if ang:
    jv = {}
    for i, gg in enumerate(G):
        if gg in ('joint-l-shoulder', 'joint-r-shoulder', 'joint-l-clavicle', 'joint-r-clavicle'): jv.setdefault(gg, set()).update(F[i])
    for gg, sgn in (('joint-l-shoulder', 1), ('joint-r-shoulder', -1)):
        pj = V[list(jv[gg])].mean(0); pc = V[list(jv[gg.replace('shoulder', 'clavicle')])].mean(0)
        mix = cfg.get('arm_pivot_mix', 0.0); pj = (1-mix)*pj + mix*pc
        if np.sign(pj[0]) != sgn: sgn = -sgn
        t = np.clip((sgn*V[:, 0] - (sgn*pj[0] - cfg.get('arm_blend_in', 0.6)))/(cfg.get('arm_blend_in', 0.6)+cfg.get('arm_blend_out', 0.3)), 0, 1)
        w = t*t*(3-2*t)
        th = -sgn*ang*w; c, s_ = np.cos(th), np.sin(th)
        dx, dy = V[:, 0]-pj[0], V[:, 1]-pj[1]
        V[:, 0] = pj[0] + c*dx - s_*dy; V[:, 1] = pj[1] + s_*dx + c*dy
# 두개골 돔 둥글게: 눈썹 위 정점을 구면 쪽으로 보간(얼굴 앞쪽·귀는 약하게)
cr = cfg.get('cranium_round', 0)
if cr:
    jid = {}
    for i, gg in enumerate(G):
        if gg in ('joint-l-eye', 'joint-r-eye', 'joint-head'): jid.setdefault(gg, set()).update(F[i])
    eye = 0.5*(V[list(jid['joint-l-eye'])].mean(0) + V[list(jid['joint-r-eye'])].mean(0))
    bodyv = np.array(sorted({v for i, gg in enumerate(G) if gg == 'body' for v in F[i]}))
    top = V[bodyv, 1].max()
    yb = eye[1] + cfg.get('cranium_start', 0.15)*(top - eye[1])
    earv = set()
    import glob
    for f in glob.glob(f'{D}/targets/ears/*.target.gz'):
        for l in gzip.open(f, 'rt'):
            p = l.split()
            if len(p) == 4: earv.add(int(p[0]))
    hv = bodyv[(V[bodyv, 1] > yb) & (np.abs(V[bodyv, 0]) < 1.2)]
    hv = np.array([v for v in hv if v not in earv])  # 귀 제외
    zc = V[hv, 2].mean()
    c = np.array([0.0, yb, zc])
    d = V[hv] - c; r = np.linalg.norm(d, axis=1); R = np.percentile(r, 70)
    t = np.clip((V[hv, 1]-yb)/(top-yb)/0.35, 0, 1); w = cr*t*t*(3-2*t)
    w *= np.clip(1 - np.maximum(0, V[hv, 2]-zc)/R*1.2, 0, 1)
    V[hv] = c + d/r[:, None]*(r + w*(np.maximum(R, r)-r))[:, None]
    print('cranium R', round(R, 3), 'verts', len(hv))
# 어깨 경사: 목 밖(|x|>x0) 정점을 바깥으로 갈수록 내림, 목 아래에서만(부드러운 y 가중치)
sd_k = cfg.get('shoulder_drop', 0); tf = cfg.get('torso_forward', 0)
if sd_k or tf:
    jn = {}
    for i, gg in enumerate(G):
        if gg in ('joint-neck', 'joint-head'): jn.setdefault(gg, set()).update(F[i])
    yn = V[list(jn['joint-neck'])].mean(0)[1]; yh = V[list(jn['joint-head'])].mean(0)[1]
    def ramp(a, lo, hi):
        t = np.clip((a-lo)/(hi-lo), 0, 1); return t*t*(3-2*t)
    if sd_k:
        x0 = cfg.get('shoulder_x0', 0.55)
        wy = 1 - ramp(V[:, 1], yn - 0.2, yn + 0.4)
        ax = np.abs(V[:, 0]); dx = np.where(ax > x0, ax - x0, 0)
        dx = np.sqrt(dx**2 + 0.05**2) - 0.05  # 부드러운 꺾임
        sat = cfg.get('shoulder_sat', 99.0); dx = sat*np.tanh(dx/sat)  # 바깥쪽은 포화
        V[:, 1] -= sd_k*dx*wy
    if tf:  # 머리 아래 몸통을 앞으로(머리는 그대로)
        V[:, 2] += tf*(1 - ramp(V[:, 1], yn - 0.6, yh))
np.save(out + '.full.npy', V)
keep = set(cfg.get('groups', ['body', 'helper-l-eye', 'helper-r-eye']))
sel = [i for i, gg in enumerate(G) if gg in keep]
used = sorted({v for i in sel for v in F[i]}); remap = {v: k for k, v in enumerate(used)}
with open(out, 'w') as f:
    f.write(''.join(f'v {a:.6f} {b:.6f} {c:.6f}\n' for a, b, c in V[used]))
    for i in sel: f.write('f ' + ' '.join(str(remap[v]+1) for v in F[i]) + '\n')
print('verts', len(used), 'faces', len(sel), 'bbox', V[used].min(0).round(3), V[used].max(0).round(3))
