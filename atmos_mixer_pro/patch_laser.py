import os

path = 'assets/3d_simulator/studio_engine.html'
with open(path, 'r') as f:
    content = f.read()

# 1. Add Laser Line to speaker group
old_code = """      const dispCone = new THREE.Mesh(coneGeo, coneMat);
      dispCone.position.set(0, 0.0, frontZ);
      group.add(dispCone);"""

new_code = """      const dispCone = new THREE.Mesh(coneGeo, coneMat);
      dispCone.position.set(0, 0.0, frontZ);
      group.add(dispCone);

      // --- 스마트 타겟팅 라인 (조준선 레이저) ---
      // 룸의 대각선 길이를 대략적으로 구해서 넉넉한 레이저 길이 설정
      const maxRoomDim = Math.max(currentRoom.width || 15, currentRoom.depth || 15, currentRoom.height || 5) * 1.5;
      
      const laserMat = new THREE.LineBasicMaterial({ 
        color: 0xef4444, // 초기 색상: 빨강 (Miss)
        transparent: true, 
        opacity: 0.6,
        linewidth: 2
      });
      const laserPoints = [];
      laserPoints.push(new THREE.Vector3(0, 0, 0));
      laserPoints.push(new THREE.Vector3(0, 0, maxRoomDim)); // 스피커 Z축(앞)으로 레이저 발사
      const laserGeo = new THREE.BufferGeometry().setFromPoints(laserPoints);
      const laserLine = new THREE.Line(laserGeo, laserMat);
      // 위치를 콘(트위터 앞면)과 동일하게 맞춤
      laserLine.position.set(0, 0.0, frontZ);
      laserLine.userData.isTargetLaser = true;
      group.add(laserLine);"""

content = content.replace(old_code, new_code)

# 2. Add Animation Loop Logic to update Laser Color
anim_old = """    function animate() {
      requestAnimationFrame(animate);
      if (controls) controls.update();
      renderer.render(scene, camera);
    }"""

anim_new = """    function animate() {
      requestAnimationFrame(animate);
      if (controls) controls.update();

      // --- 스마트 타겟팅 조준선 색상 업데이트 로직 ---
      speakerMeshes.forEach(spk => {
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
            laser.material.opacity = 0.25; // 빗나갔을 땐 거슬리지 않게 투명하게
          }
        }
      });

      renderer.render(scene, camera);
    }"""

content = content.replace(anim_old, anim_new)

with open(path, 'w') as f:
    f.write(content)
print("Laser targeting applied successfully.")
