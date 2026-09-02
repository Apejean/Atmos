import os

path = 'assets/3d_simulator/studio_engine.html'
with open(path, 'r') as f:
    content = f.read()

old_code = """            // Update isFixed
            mesh.userData.isFixed = sp.isFixed;
            
            // Update selection colors"""

new_code = """            // Update isFixed
            mesh.userData.isFixed = sp.isFixed;
            
            // Update dispersion cone radius dynamically
            const dispCone = mesh.children.find(c => c.geometry && c.geometry.type === 'ConeGeometry');
            if (dispCone && sp.dispersionAngle) {
              const dispersionAngle = (sp.dispersionAngle || 90) * Math.PI / 180;
              const beamLength = 1.8;
              const newRadius = Math.tan(dispersionAngle / 2) * beamLength;
              
              if (dispCone.geometry) dispCone.geometry.dispose();
              
              const newGeo = new THREE.ConeGeometry(newRadius, beamLength, 16, 1, true);
              newGeo.rotateX(-Math.PI / 2);
              newGeo.translate(0, 0, beamLength / 2);
              dispCone.geometry = newGeo;
            }

            // Update selection colors"""

content = content.replace(old_code, new_code)
with open(path, 'w') as f:
    f.write(content)
print("Dispersion update logic patched.")
