import sys
import trimesh
import math

def scale_scene(w, d, h, in_path, out_path):
    scene = trimesh.load(in_path, force='scene')
    
    # geometry_0 is room, geometry_1 is mannequin
    room_mesh = None
    mannequin_mesh = None
    
    for name, geom in scene.geometry.items():
        if len(geom.vertices) < 1000:
            room_mesh = geom
        else:
            mannequin_mesh = geom
            
    if room_mesh is None:
        return
        
    bounds = room_mesh.bounds
    orig_w = bounds[1][0] - bounds[0][0]
    orig_h = bounds[1][1] - bounds[0][1]
    orig_d = bounds[1][2] - bounds[0][2]
    
    scale_x = float(w) / orig_w
    scale_y = float(h) / orig_h
    scale_z = float(d) / orig_d
    
    matrix = trimesh.transformations.scale_and_translate(
        scale=[scale_x, scale_y, scale_z],
        translate=[0,0,0]
    )
    room_mesh.apply_transform(matrix)
    
    scene.export(out_path)

if __name__ == '__main__':
    scale_scene(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5])
