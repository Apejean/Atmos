# Blender: 로프트 몸통 피부를 귀 주변에서 MakeHuman 피부로 서서히 감싸 붙이고(Shrinkwrap+가중치), 귓바퀴만 합침
import sys, json, bpy, bmesh, numpy as np
body, mh, out, cfgp = sys.argv[1:5]; cfg = json.load(open(cfgp))
bpy.ops.wm.read_factory_settings(use_empty=True)
def imp(p):
    bpy.ops.wm.obj_import(filepath=p, forward_axis='NEGATIVE_Z', up_axis='Y'); o = bpy.context.selected_objects[0]
    bpy.context.view_layer.objects.active = o; bpy.ops.object.transform_apply(location=True, rotation=True, scale=True); return o
def act(o):
    bpy.ops.object.select_all(action='DESELECT'); o.select_set(True); bpy.context.view_layer.objects.active = o
def boolean(a, b, op, solver):
    act(a); m = a.modifiers.new('b', 'BOOLEAN'); m.operation = op; m.object = b; m.solver = solver
    bpy.ops.object.modifier_apply(modifier='b')
def ell(co, c, r):  # Blender 좌표 정점 → 타원체 정규 거리(좌우 대칭)
    x, by, bz = co; ay, az = bz, -by
    return np.sqrt(((abs(x)-c[0])/r[0])**2 + ((ay-c[1])/r[1])**2 + ((az-c[2])/r[2])**2)
def ss(t): t = min(max(t, 0.0), 1.0); return t*t*(3-2*t)
B = imp(body); M = imp(mh)
# 1) 몸통을 고르게 다시 짜기
act(B); r = B.modifiers.new('rm', 'REMESH'); r.mode = 'VOXEL'; r.voxel_size = cfg['voxel']; r.adaptivity = 0
bpy.ops.object.modifier_apply(modifier='rm')
# 2) 가중치: 안쪽 타원체=1 → 바깥 타원체=0
vg = B.vertex_groups.new(name='w'); c = cfg['c']; ri = cfg['r_in']; ro = cfg['r_out']
for v in B.data.vertices:
    ei = ell(v.co, c, ri); eo = ell(v.co, c, ro)
    # 안쪽 경계(ei=1)에서 바깥 경계(eo=1)까지 보간
    t = 1.0 if ei <= 1 else (0.0 if eo >= 1 else 1 - ss((ei-1)/max(1e-6, (ei-1) + (1-eo))))
    ee = ell(v.co, cfg['c_ear'], cfg['r_ear'])
    if ee < 1: t *= ss((ee - 0.6)/0.4)
    if t > 0: vg.add([v.index], t, 'REPLACE')
# 3) Shrinkwrap(가장 가까운 표면점)으로 MakeHuman 피부에 붙이기
if not cfg.get('no_wrap'):
  act(B); s = B.modifiers.new('sw', 'SHRINKWRAP'); s.target = M; s.wrap_method = 'NEAREST_SURFACEPOINT'; s.vertex_group = 'w'
  bpy.ops.object.modifier_apply(modifier='sw')
# 3b) 귀 영역 안에서는 몸통 피부를 안쪽으로 물려 MakeHuman 귀(귓구멍·오목한 곳)가 그대로 드러나게
B.data.update()
dep = cfg.get('recess', 0.12); fall = cfg.get('recess_fall', 0.35)
for v in B.data.vertices:
    e = ell(v.co, cfg['c_ear'], cfg['r_ear'])
    if e < 1:
        k = dep*ss((1-e)/fall)
        x, by, bz = v.co; dx, dz = x, (-by) - cfg.get('axis_z', 0.1)  # Atmos z = -by
        n = (dx*dx + dz*dz) ** 0.5 + 1e-9
        v.co.x = x - k*dx/n; v.co.y = by + k*dz/n  # Atmos z 감소 = Blender y 증가
# 4) 귓바퀴 덩어리: MakeHuman ∩ 귀 타원체, 살짝 안으로 넣어 피부와 겹치지 않게
for sgn in (1, -1):
    M2 = M.copy(); M2.data = M.data.copy(); bpy.context.collection.objects.link(M2)
    ce = cfg['c_ear']; re_ = cfg['r_ear']
    bpy.ops.mesh.primitive_uv_sphere_add(segments=64, ring_count=32, radius=1, location=(sgn*ce[0], -ce[2], ce[1]))
    S = bpy.context.active_object; S.scale = (re_[0], re_[2], re_[1]); bpy.ops.object.transform_apply(scale=True)
    boolean(M2, S, 'INTERSECT', 'FLOAT'); bpy.data.objects.remove(S)
    M2.location.x = -sgn*cfg['sink']; act(M2); bpy.ops.object.transform_apply(location=True)
    # 불리언 대신 합치기(join) 후 복셀 리메시로 부피 합집합 — 자기교차에 강함
    bpy.ops.object.select_all(action='DESELECT'); M2.select_set(True); B.select_set(True); bpy.context.view_layer.objects.active = B
    bpy.ops.object.join()
bpy.data.objects.remove(M)
# 5) 다시 고르게 짜고 귀뿌리 이음만 살짝 스무딩, 감축
act(B); r = B.modifiers.new('rm', 'REMESH'); r.mode = 'VOXEL'; r.voxel_size = cfg['voxel']; r.adaptivity = 0
bpy.ops.object.modifier_apply(modifier='rm')
# 전체 가벼운 라플라시안 스무딩(작은 울퉁불퉁 제거, 형태 유지)
if cfg.get('lap_iter', 0):
    act(B); L = B.modifiers.new('lap', 'LAPLACIANSMOOTH'); L.iterations = cfg['lap_iter']; L.lambda_factor = cfg.get('lap_lambda', 0.3)
    L.use_volume_preserve = True; L.use_normalized = True; bpy.ops.object.modifier_apply(modifier='lap')
lo, hi = cfg['band']
for v in B.data.vertices: v.select = bool(lo < ell(v.co, cfg['c_ear'], cfg['r_ear']) < hi)
bpy.ops.object.mode_set(mode='EDIT'); bpy.ops.mesh.select_mode(type='VERT'); bpy.ops.mesh.vertices_smooth(factor=0.5, repeat=cfg['smooth_rep']); bpy.ops.object.mode_set(mode='OBJECT')
# 귀 주변 피부만 추가로 매끈하게: 귀 타원체 밖 띠(가중치 점감), 귓바퀴는 제외
if cfg.get('skin_iter', 0):
    vg2 = B.vertex_groups.new(name='skin'); a0, a1, a2 = cfg['skin_band']
    for v in B.data.vertices:
        e = ell(v.co, cfg['c_ear'], cfg['r_ear'])
        w = ss((e - a0)/(a1 - a0)) * (1 - ss((e - a1)/(a2 - a1)))
        if w > 0: vg2.add([v.index], w, 'REPLACE')
    act(B); L = B.modifiers.new('lap2', 'LAPLACIANSMOOTH'); L.iterations = cfg['skin_iter']; L.lambda_factor = cfg.get('skin_lambda', 0.5)
    L.use_volume_preserve = True; L.use_normalized = True; L.vertex_group = 'skin'; bpy.ops.object.modifier_apply(modifier='lap2')
t0 = B.modifiers.new('t0', 'TRIANGULATE'); bpy.ops.object.modifier_apply(modifier='t0')
d = B.modifiers.new('dec', 'DECIMATE'); d.ratio = min(1.0, cfg['target_tris']/len(B.data.polygons)); bpy.ops.object.modifier_apply(modifier='dec')
t = B.modifiers.new('tri', 'TRIANGULATE'); bpy.ops.object.modifier_apply(modifier='tri')
bm = bmesh.new(); bm.from_mesh(B.data); print('non-manifold', sum(1 for e in bm.edges if not e.is_manifold)); bm.free()
P = np.array([v.co[:] for v in B.data.vertices]); F = np.array([p.vertices[:] for p in B.data.polygons])
A = np.stack([P[:, 0], P[:, 2], -P[:, 1]], 1); np.save(out + '.v.npy', A); np.save(out + '.f.npy', F); print('verts', len(A), 'tris', len(F))
