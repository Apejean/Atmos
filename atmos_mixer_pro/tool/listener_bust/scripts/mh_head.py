# MakeHuman 머리(귀 포함)를 고운 복셀로 다시 만들어 std8과 같은 좌표로 내보냄
import sys, json, bpy, numpy as np
src, out, ymin = sys.argv[1], sys.argv[2], float(sys.argv[3])
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.wm.obj_import(filepath=src, forward_axis='NEGATIVE_Z', up_axis='Y'); ob = bpy.context.selected_objects[0]
bpy.context.view_layer.objects.active = ob; bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
sd = ob.modifiers.new('sd', 'SUBSURF'); sd.levels = 2; bpy.ops.object.modifier_apply(modifier='sd')
bpy.ops.mesh.primitive_cube_add(size=1); box = bpy.context.active_object; box.scale = (40, 40, 40); box.location = (0, 0, ymin + 20)
bpy.context.view_layer.objects.active = ob
b = ob.modifiers.new('cut', 'BOOLEAN'); b.operation = 'INTERSECT'; b.object = box; b.solver = 'FLOAT'; bpy.ops.object.modifier_apply(modifier='cut'); bpy.data.objects.remove(box)
r = ob.modifiers.new('rm', 'REMESH'); r.mode = 'VOXEL'; r.voxel_size = float(sys.argv[4]); r.adaptivity = 0; bpy.ops.object.modifier_apply(modifier='rm')
t = ob.modifiers.new('tri', 'TRIANGULATE'); bpy.ops.object.modifier_apply(modifier='tri')
P = np.array([v.co[:] for v in ob.data.vertices]); F = np.array([p.vertices[:] for p in ob.data.polygons])
np.save(out + '.raw.v.npy', np.stack([P[:, 0], P[:, 2], -P[:, 1]], 1)); np.save(out + '.f.npy', F); print('verts', len(P), 'tris', len(F))
