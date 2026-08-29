import json
import struct

def parse_glb(file_path):
    with open(file_path, 'rb') as f:
        magic = f.read(4)
        version = struct.unpack('<I', f.read(4))[0]
        length = struct.unpack('<I', f.read(4))[0]
        
        chunk0_length = struct.unpack('<I', f.read(4))[0]
        chunk0_type = f.read(4)
        chunk0_data = f.read(chunk0_length)
        
        gltf_json = json.loads(chunk0_data.decode('utf-8'))
        print(json.dumps(gltf_json['meshes'], indent=2))

parse_glb('assets/models/room_with_listener.glb')
