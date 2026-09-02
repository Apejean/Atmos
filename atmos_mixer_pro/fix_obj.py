import sys

def cap_obj(input_path, output_path):
    with open(input_path, 'r') as f:
        lines = f.readlines()
        
    vertices = []
    for line in lines:
        if line.startswith('v '):
            parts = line.split()
            vertices.append([float(parts[1]), float(parts[2]), float(parts[3])])
            
    # Find minimum Y (bottom)
    min_y = min(v[1] for v in vertices)
    
    # Find all vertices at min_y (with some tolerance)
    bottom_vertices_indices = []
    for i, v in enumerate(vertices):
        if abs(v[1] - min_y) < 0.01:
            bottom_vertices_indices.append(i + 1) # OBJ indices are 1-based
            
    if not bottom_vertices_indices:
        print("No bottom vertices found")
        return
        
    # Calculate centroid
    cx = sum(vertices[i-1][0] for i in bottom_vertices_indices) / len(bottom_vertices_indices)
    cy = min_y
    cz = sum(vertices[i-1][2] for i in bottom_vertices_indices) / len(bottom_vertices_indices)
    
    # Write new file
    with open(output_path, 'w') as f:
        f.writelines(lines)
        
        # Add centroid vertex
        f.write(f"v {cx} {cy} {cz}\n")
        centroid_idx = len(vertices) + 1
        
        # Create a triangle fan from centroid to the boundary
        # Assuming vertices are somewhat ordered. If not, this might create intersecting faces
        # For a proper cap, we need to sort them radially around the centroid
        
        import math
        def get_angle(idx):
            v = vertices[idx-1]
            return math.atan2(v[2] - cz, v[0] - cx)
            
        bottom_vertices_indices.sort(key=get_angle)
        
        f.write("g cap\n")
        f.write("usemtl cap_mat\n")
        
        for i in range(len(bottom_vertices_indices)):
            v1 = bottom_vertices_indices[i]
            v2 = bottom_vertices_indices[(i+1) % len(bottom_vertices_indices)]
            # Normal direction depends on winding order. Try one, if it's wrong, invert.
            f.write(f"f {v1} {v2} {centroid_idx}\n")
            
    print(f"Capped OBJ written to {output_path} with {len(bottom_vertices_indices)} triangles.")

if __name__ == '__main__':
    cap_obj('assets/models/listener_bust.obj', 'assets/models/listener_bust_capped.obj')
