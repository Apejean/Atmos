# MakeHuman 머리에서 귓바퀴를 뺀 피부만 강하게 매끈하게(감싸 붙일 대상 피부의 잔굴곡 제거)
import sys, json, bpy, numpy as np
src, out, it, lam = sys.argv[1], sys.argv[2], int(sys.argv[3]), float(sys.argv[4])
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.wm.obj_import(filepath=src, forward_axis='NEGATIVE_Z', up_axis='Y'); ob = bpy.context.selected_objects[0]
bpy.context.view_layer.objects.active = ob; bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
def ss(t): t = min(max(t, 0.0), 1.0); return t*t*(3-2*t)
c = (0.97, 2.655, -0.12); r = (0.19, 0.55, 0.36)
vg = ob.vertex_groups.new(name='skin')
for v in ob.data.vertices:
    x, by, bz = v.co; ay, az = bz, -by
    e = ((abs(x)-c[0])/r[0])**2 + ((ay-c[1])/r[1])**2 + ((az-c[2])/r[2])**2
    w = ss((e**0.5 - 0.95)/0.3)
    if w > 0: vg.add([v.index], w, 'REPLACE')
L = ob.modifiers.new('lap', 'LAPLACIANSMOOTH'); L.iterations = it; L.lambda_factor = lam; L.use_volume_preserve = True; L.use_normalized = True; L.vertex_group = 'skin'
bpy.ops.object.modifier_apply(modifier='lap')
bpy.ops.wm.obj_export(filepath=out, forward_axis='NEGATIVE_Z', up_axis='Y', export_materials=False, export_normals=False, export_uv=False)
print('done', len(ob.data.vertices))
