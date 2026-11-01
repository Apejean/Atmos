# Blender(bpy): 서브디비전 → 복셀 리메시 → 수평 절단(측면 절단 없음) → 감축. 좌표는 원본(MakeHuman: Y 위, +Z 앞, dm) 그대로 내보냄
import sys, json, numpy as np, bpy, bmesh
src, out, cfgp = sys.argv[1], sys.argv[2], sys.argv[3]
cfg = json.load(open(cfgp))
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.wm.obj_import(filepath=src, forward_axis='NEGATIVE_Z', up_axis='Y')
ob = bpy.context.selected_objects[0]; bpy.context.view_layer.objects.active = ob
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
sd = ob.modifiers.new('sd', 'SUBSURF'); sd.levels = 2
bpy.ops.object.modifier_apply(modifier='sd')
r = ob.modifiers.new('rm', 'REMESH'); r.mode = 'VOXEL'; r.voxel_size = cfg['voxel']; r.adaptivity = 0
bpy.ops.object.modifier_apply(modifier='rm')
# 감축을 먼저(절단 뒤 남을 비율로 환산), 절단은 마지막 — 절단면 가장자리 접힘 방지
bottom = cfg['bottom_y']
zs = np.array([p.center[2] for p in ob.data.polygons])
keep_frac = float((zs > bottom).mean())
d = ob.modifiers.new('dec', 'DECIMATE'); d.ratio = min(1.0, cfg['target_tris']*0.97/2/(len(ob.data.polygons)*keep_frac))
bpy.ops.object.modifier_apply(modifier='dec')
bpy.ops.mesh.primitive_cube_add(size=1); box = bpy.context.active_object
box.scale = (40, 40, 40); box.location = (0, 0, bottom + 20)
bpy.context.view_layer.objects.active = ob
b = ob.modifiers.new('cut', 'BOOLEAN'); b.operation = 'INTERSECT'; b.object = box; b.solver = 'EXACT'
bpy.ops.object.modifier_apply(modifier='cut'); bpy.data.objects.remove(box)
t = ob.modifiers.new('tri', 'TRIANGULATE'); t.ngon_method = 'BEAUTY'; bpy.ops.object.modifier_apply(modifier='tri')
bm = bmesh.new(); bm.from_mesh(ob.data); print('non-manifold edges', sum(1 for e in bm.edges if not e.is_manifold)); bm.free()
P = np.array([v.co[:] for v in ob.data.vertices]); Fc = np.array([p.vertices[:] for p in ob.data.polygons])
A = np.stack([P[:, 0], P[:, 2], -P[:, 1]], 1)  # Blender → 원본 좌표
np.save(out + '.raw.v.npy', A); np.save(out + '.f.npy', Fc)
print('verts', len(A), 'tris', len(Fc))
