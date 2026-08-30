import re

with open('assets/3d_simulator/studio_engine.html', 'r') as f:
    content = f.read()

old_logic = """      if (data.speakers) {
        currentSpeakers = data.speakers;
        while (speakersGroup.children.length > 0) {
          const obj = speakersGroup.children[0];
          speakersGroup.remove(obj);
        }

        currentSpeakers.forEach(sp => {
          const mesh = createSpeaker3DMesh(sp);
          speakersGroup.add(mesh);
        });
      }"""

new_logic = """      if (data.speakers) {
        currentSpeakers = data.speakers;
        const validIds = new Set(currentSpeakers.map(s => s.id));
        
        // Remove deleted speakers
        for (let i = speakersGroup.children.length - 1; i >= 0; i--) {
          const child = speakersGroup.children[i];
          if (!validIds.has(child.userData.speakerId)) {
            speakersGroup.remove(child);
          }
        }

        // Update or create speakers
        currentSpeakers.forEach(sp => {
          let mesh = speakersGroup.children.find(c => c.userData.speakerId === sp.id);
          const isSelected = (sp.id === selectedSpeakerId);
          
          if (mesh) {
            // Update position
            const posX = sp.x - (currentRoom.width / 2);
            const posZ = sp.y - (currentRoom.depth / 2);
            const posY = sp.z || 1.8;
            mesh.position.set(posX, posY, posZ);
            
            // Update rotation
            const yawRad = ((sp.rotation || 0) * Math.PI) / 180;
            const pitchRad = ((sp.pitchTilt || 0) * Math.PI) / 180;
            mesh.rotation.set(-pitchRad, -yawRad, 0);

            // Update isFixed
            mesh.userData.isFixed = sp.isFixed;
            
            // Update selection colors
            const colorCabinet = isSelected ? 0x1e293b : 0x181e28;
            const colorWire = isSelected ? 0x38bdf8 : 0x475569;
            const colorDome = isSelected ? 0x38bdf8 : 0x0ea5e9;
            const emissiveDome = isSelected ? 0x0284c7 : 0x0369a1;
            
            if (mesh.children.length > 3) {
              if (mesh.children[0].material) mesh.children[0].material.color.setHex(colorCabinet);
              if (mesh.children[1].material) mesh.children[1].material.color.setHex(colorWire);
              if (mesh.children[3].material) {
                mesh.children[3].material.color.setHex(colorDome);
                mesh.children[3].material.emissive.setHex(emissiveDome);
              }
            }
          } else {
            mesh = createSpeaker3DMesh(sp);
            speakersGroup.add(mesh);
          }
        });
      }"""

if old_logic in content:
    content = content.replace(old_logic, new_logic)
    with open('assets/3d_simulator/studio_engine.html', 'w') as f:
        f.write(content)
    print("Patched successfully.")
else:
    print("Old logic not found!")
