import os

path = 'assets/3d_simulator/studio_engine.html'
with open(path, 'r') as f:
    content = f.read()

# Replace animate loop body
old_anim = """      // --- 스마트 타겟팅 조준선 색상 업데이트 로직 ---
      speakersGroup.children.forEach(spk => {
        let laser = null;
        spk.children.forEach(c => {
          if (c.userData.isTargetLaser) laser = c;
        });

        if (laser) {
          // 스피커가 바라보는 전방(Forward) 벡터 계산 (로컬 +Z)
          const spkWorldPos = new THREE.Vector3();
          laser.getWorldPosition(spkWorldPos);
          
          const forwardDir = new THREE.Vector3(0, 0, 1);
          forwardDir.applyQuaternion(spk.quaternion).normalize();

          // 마네킹 귀(listenerGroup) 목표 지점
          // listenerGroup 원점은 Y=0(바닥)에 있고 마네킹 전체가 거기에 들어감. 귀 높이는 currentEarLevel.
          const listenerPos = new THREE.Vector3();
          listenerGroup.getWorldPosition(listenerPos);
          const targetEarPos = new THREE.Vector3(listenerPos.x, currentEarLevel, listenerPos.z);

          // 스피커 위치(트위터 앞면)에서 귀 위치로 향하는 이상적인 궤적
          const dirToEar = new THREE.Vector3().subVectors(targetEarPos, spkWorldPos).normalize();

          // 스피커 지향 방향과 실제 귀 방향 사이의 오차 각도
          const dot = forwardDir.dot(dirToEar);
          const angleDeg = Math.acos(Math.max(-1, Math.min(1, dot))) * (180 / Math.PI);

          // 오차 12도 이내면 초록색(Hit), 벗어나면 빨간색(Miss)
          if (angleDeg <= 12.0) {
            laser.material.color.setHex(0x22c55e); // Green
            laser.material.opacity = 0.9;
          } else {
            laser.material.color.setHex(0xef4444); // Red
            laser.material.opacity = 0.25;
          }
        }
      });"""

new_anim = """      // --- 스마트 타겟팅 조준선 색상 및 Raycast 업데이트 로직 ---
      const laserRaycaster = new THREE.Raycaster();
      speakersGroup.children.forEach(spk => {
        let laser = null, hud = null, hitDot = null;
        spk.children.forEach(c => {
          if (c.userData.isTargetLaser) laser = c;
          if (c.userData.isHud) hud = c;
          if (c.userData.isHitDot) hitDot = c;
        });

        if (laser) {
          const spkWorldPos = new THREE.Vector3();
          laser.getWorldPosition(spkWorldPos);
          
          const forwardDir = new THREE.Vector3(0, 0, 1);
          forwardDir.applyQuaternion(spk.quaternion).normalize();

          // Raycast for Wall Clipping (against roomGroup)
          laserRaycaster.set(spkWorldPos, forwardDir);
          // Only intersect room walls (children of roomGroup, skipping GridHelper which is LineSegments)
          const roomMeshes = roomGroup.children.filter(c => c.type === 'LineSegments' || c.type === 'Mesh'); 
          // Actually, room is just a Wireframe (LineSegments). Raycasting against lines is finicky.
          // Wait, we need an invisible box for precise raycasting!
          // Let's create it if it doesn't exist.
          if (!roomGroup.userData.hitBox) {
             const bGeo = new THREE.BoxGeometry(currentRoom.width, currentRoom.height, currentRoom.depth);
             const bMat = new THREE.MeshBasicMaterial({visible: false, side: THREE.BackSide}); // back side so ray hits from inside
             const hitBox = new THREE.Mesh(bGeo, bMat);
             hitBox.position.y = currentRoom.height / 2; // room origin logic?
             roomGroup.add(hitBox);
             roomGroup.userData.hitBox = hitBox;
          } else {
             // update hitbox size if room changed
             roomGroup.userData.hitBox.scale.set(
               currentRoom.width / roomGroup.userData.hitBox.geometry.parameters.width,
               currentRoom.height / roomGroup.userData.hitBox.geometry.parameters.height,
               currentRoom.depth / roomGroup.userData.hitBox.geometry.parameters.depth
             );
             roomGroup.userData.hitBox.position.y = currentRoom.height / 2;
          }

          const intersects = laserRaycaster.intersectObject(roomGroup.userData.hitBox);
          let laserLen = 15;
          if (intersects.length > 0) {
            laserLen = intersects[0].distance;
            if (hitDot) {
              hitDot.visible = true;
              // move dot to local coordinate of the speaker
              const localHit = spk.worldToLocal(intersects[0].point.clone());
              hitDot.position.copy(localHit);
            }
          } else {
            if (hitDot) hitDot.visible = false;
          }

          // Update laser line length
          const posAttribute = laser.geometry.attributes.position;
          posAttribute.setZ(1, laserLen);
          posAttribute.needsUpdate = true;

          // Target logic (Listener Ear)
          const listenerPos = new THREE.Vector3();
          listenerGroup.getWorldPosition(listenerPos);
          const targetEarPos = new THREE.Vector3(listenerPos.x, currentEarLevel, listenerPos.z);
          const distToEar = spkWorldPos.distanceTo(targetEarPos);
          const dirToEar = new THREE.Vector3().subVectors(targetEarPos, spkWorldPos).normalize();
          
          const dot = forwardDir.dot(dirToEar);
          const angleDeg = Math.acos(Math.max(-1, Math.min(1, dot))) * (180 / Math.PI);

          let isHit = false;
          // 반경 0.25m 안에 들어왔는지 추가 체크 (거리와 각도 계산)
          const spreadAtEar = Math.tan(angleDeg * Math.PI / 180) * distToEar;
          if (spreadAtEar <= 0.25 && angleDeg <= 15.0) {
            isHit = true;
          }

          if (isHit) {
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
          }

          // Update HUD
          if (hud) {
             const ctx = hud.userData.hudCtx;
             ctx.clearRect(0, 0, 200, 40);
             ctx.fillStyle = 'rgba(15, 23, 42, 0.8)';
             ctx.beginPath();
             ctx.roundRect(0, 0, 200, 40, 8);
             ctx.fill();
             ctx.fillStyle = '#ffffff';
             ctx.font = 'bold 16px -apple-system, sans-serif';
             ctx.textAlign = 'center';
             ctx.textBaseline = 'middle';
             const delayMs = (distToEar / 343 * 1000).toFixed(1);
             ctx.fillText(`Dist: ${distToEar.toFixed(2)}m | ${delayMs}ms`, 100, 20);
             hud.userData.hudTex.needsUpdate = true;
          }
        }
      });"""

content = content.replace(old_anim, new_anim)

with open(path, 'w') as f:
    f.write(content)
print("Animate loop patched for Physics & HUD.")
