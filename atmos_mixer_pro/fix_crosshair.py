import os

path = 'assets/3d_simulator/studio_engine.html'
with open(path, 'r') as f:
    content = f.read()

# Introduce anyHit logic
old_loop_start = """      const laserRaycaster = new THREE.Raycaster();
      speakersGroup.children.forEach(spk => {"""

new_loop_start = """      const laserRaycaster = new THREE.Raycaster();
      let anyHit = false;
      let crosshairSpkPos = null;
      speakersGroup.children.forEach(spk => {"""

content = content.replace(old_loop_start, new_loop_start)

old_crosshair_logic = """          if (isHit) {
            laser.material.color.setHex(0x22c55e); 
            laser.material.opacity = 0.9;
            if (hitDot) hitDot.material.color.setHex(0x22c55e);
            
            // Draw Crosshair on Mannequin (Global state managed elsewhere, let's create a ring)
            if (!listenerGroup.userData.crosshair) {
               const ringGeo = new THREE.RingGeometry(0.2, 0.25, 32);
               const ringMat = new THREE.MeshBasicMaterial({ color: 0x22c55e, transparent: true, opacity: 0.8, side: THREE.DoubleSide });
               const crosshair = new THREE.Mesh(ringGeo, ringMat);
               crosshair.position.y = currentEarLevel;
               listenerGroup.add(crosshair);
               listenerGroup.userData.crosshair = crosshair;
            }
            listenerGroup.userData.crosshair.visible = true;
            listenerGroup.userData.crosshair.lookAt(spkWorldPos); // face the speaker
          } else {
            laser.material.color.setHex(0xef4444); 
            laser.material.opacity = 0.25;
            if (hitDot) hitDot.material.color.setHex(0xef4444);
            
            if (listenerGroup.userData.crosshair) {
               listenerGroup.userData.crosshair.visible = false;
            }
          }"""

new_crosshair_logic = """          if (isHit) {
            anyHit = true;
            crosshairSpkPos = spkWorldPos;
            laser.material.color.setHex(0x22c55e); 
            laser.material.opacity = 0.9;
            if (hitDot) hitDot.material.color.setHex(0x22c55e);
          } else {
            laser.material.color.setHex(0xef4444); 
            laser.material.opacity = 0.25;
            if (hitDot) hitDot.material.color.setHex(0xef4444);
          }"""

content = content.replace(old_crosshair_logic, new_crosshair_logic)

old_loop_end = """        }
      });

      renderer.render(scene, camera);"""

new_loop_end = """        }
      });

      // Manage crosshair visibility globally
      if (anyHit) {
         if (!listenerGroup.userData.crosshair) {
            const ringGeo = new THREE.RingGeometry(0.2, 0.25, 32);
            const ringMat = new THREE.MeshBasicMaterial({ color: 0x22c55e, transparent: true, opacity: 0.8, side: THREE.DoubleSide });
            const crosshair = new THREE.Mesh(ringGeo, ringMat);
            crosshair.position.y = currentEarLevel;
            listenerGroup.add(crosshair);
            listenerGroup.userData.crosshair = crosshair;
         }
         listenerGroup.userData.crosshair.visible = true;
         if (crosshairSpkPos) listenerGroup.userData.crosshair.lookAt(crosshairSpkPos);
      } else {
         if (listenerGroup.userData.crosshair) {
            listenerGroup.userData.crosshair.visible = false;
         }
      }

      renderer.render(scene, camera);"""

content = content.replace(old_loop_end, new_loop_end)

with open(path, 'w') as f:
    f.write(content)

print("Crosshair logic fixed.")
