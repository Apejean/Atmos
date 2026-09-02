import os

path = 'assets/3d_simulator/studio_engine.html'
with open(path, 'r') as f:
    content = f.read()

# Fix the rotation update in updateScene
old_update = """            const yawRad = ((sp.rotation || 0) * Math.PI) / 180;
            const pitchRad = ((sp.pitchTilt || 0) * Math.PI) / 180;
            mesh.rotation.set(-pitchRad, -yawRad, 0);"""

new_update = """            const yawRad = ((sp.rotation || 0) * Math.PI) / 180;
            const pitchRad = ((sp.pitchTilt || 0) * Math.PI) / 180;
            mesh.rotation.order = 'YXZ';
            mesh.rotation.y = yawRad;
            mesh.rotation.x = -pitchRad;"""

content = content.replace(old_update, new_update)
with open(path, 'w') as f:
    f.write(content)

print("Rotation update bug fixed.")
