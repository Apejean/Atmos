# 로프트 몸통의 옛 귀 흔적 제거: 머리 옆면을 x=f(y,z) 높이맵으로 보고 귀 영역을 이중조화(biharmonic) 보간으로 메움
import sys, json, numpy as np
from scipy.interpolate import griddata
from skimage.restoration import inpaint_biharmonic
src, out, cfgp = sys.argv[1:4]; cfg = json.load(open(cfgp))
V = np.load(src + '.v.npy').copy(); F = np.load(src + '.f.npy')
# 정점 법선
n = np.zeros_like(V); fn = np.cross(V[F[:, 1]]-V[F[:, 0]], V[F[:, 2]]-V[F[:, 0]])
for k in range(3): np.add.at(n, F[:, k], fn)
n /= np.linalg.norm(n, axis=1, keepdims=True) + 1e-12
yc, zc = cfg['c']; ry, rz = cfg['r']; Y0, Y1, Z0, Z1 = cfg['box']; h = cfg['h']
gy = np.arange(Y0, Y1+h, h); gz = np.arange(Z0, Z1+h, h); GY, GZ = np.meshgrid(gy, gz, indexing='ij')
for s in (1, -1):
    m = (s*V[:, 0] > 0.3) & (s*n[:, 0] > cfg.get("nmin", 0.35)) & (V[:, 1] > Y0-0.05) & (V[:, 1] < Y1+0.05) & (V[:, 2] > Z0-0.05) & (V[:, 2] < Z1+0.05)
    f = griddata(V[m][:, [1, 2]], s*V[m, 0], (GY, GZ), method='linear')
    e = np.sqrt(((GY-yc)/ry)**2 + ((GZ-zc)/rz)**2)
    mask = (e < 1) | np.isnan(f)
    fill = np.where(np.isnan(f), np.nanmean(f), f)
    fi = inpaint_biharmonic(fill, mask)
    # 정점 갱신: 귀 영역 안에서 메운 면보다 바깥이면 끌어들임(가장자리는 부드럽게)
    sel = np.where(m)[0]
    yi = (V[sel, 1]-Y0)/h; zi = (V[sel, 2]-Z0)/h
    from scipy.ndimage import map_coordinates
    target = map_coordinates(fi, [yi, zi], order=1, mode='nearest')
    ev = np.sqrt(((V[sel, 1]-yc)/ry)**2 + ((V[sel, 2]-zc)/rz)**2)
    t = np.clip((ev - cfg['soft'][0])/(cfg['soft'][1]-cfg['soft'][0]), 0, 1); w = 1 - t*t*(3-2*t)
    cur = s*V[sel, 0]; new = cur + w*(target - cur)
    V[sel, 0] = s*new
    print('side', s, 'max pull', round(float(np.max(cur-new)), 3))
np.save(out + '.v.npy', V); np.save(out + '.f.npy', F)
with open(out, 'w') as fo:
    fo.write(''.join(f'v {a:.7f} {b:.7f} {c:.7f}\n' for a, b, c in V)); fo.write(''.join(f'f {p+1} {q+1} {r+1}\n' for p, q, r in F))
