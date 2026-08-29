import json
import struct

def parse_glb(file_path):
    with open(file_path, 'rb') as f:
        f.read(12)
        chunk0_length = struct.unpack('<I', f.read(4))[0]
        f.read(4)
        chunk0_data = f.read(chunk0_length)
        gltf = json.loads(chunk0_data.decode('utf-8'))
        print(json.dumps(gltf.get('nodes', []), indent=2))

parse_glb('assets/models/room_with_listener.glb')
