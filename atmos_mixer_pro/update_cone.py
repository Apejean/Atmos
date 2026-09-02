import os

path = 'assets/3d_simulator/studio_engine.html'
with open(path, 'r') as f:
    content = f.read()

# Replace the specific line that creates the cone geometry
old_line = "const coneGeo = new THREE.ConeGeometry(0.1, 0.4, 8);"
new_line = """
    // 1. 방사선 거리 동적 조절 (Dynamic Cone): 방의 가장 긴 변을 기준으로 길이 계산
    const maxRoomDim = Math.max(currentRoom.width || 10, currentRoom.depth || 10, currentRoom.height || 3);
    const dynamicConeLength = maxRoomDim * 1.5; // 방 끝까지 닿도록 충분히 길게 연장
    const dynamicConeRadius = dynamicConeLength * 0.25; // 길이에 비례해서 퍼지는 반경 조절
    const coneGeo = new THREE.ConeGeometry(dynamicConeRadius, dynamicConeLength, 16); // 각지게 보이지 않도록 세그먼트 증가
"""

if old_line not in content:
    print("Cone geometry creation line not found!")
    exit(1)

# Replace the specific line that translates the cone geometry to align its tip
old_translate_line = "coneGeo.translate(0, -0.2, 0);"
new_translate_line = "coneGeo.translate(0, -dynamicConeLength / 2, 0);"

if old_translate_line not in content:
    print("Cone translate line not found!")
    exit(1)

content = content.replace(old_line, new_line)
content = content.replace(old_translate_line, new_translate_line)

# Replace the cone material to make it transparent, gradient, or a wireframe to not block the view completely
old_mat_line = "const coneMat = new THREE.MeshBasicMaterial({ color: 0x38bdf8, wireframe: true, transparent: true, opacity: 0.3 });"
new_mat_line = """
    // 2. 스마트 타겟팅 룩 적용: 너무 가려지지 않게 끝으로 갈수록 투명해지는 느낌의 연한 색상 적용
    const coneMat = new THREE.MeshBasicMaterial({ 
      color: 0x38bdf8, 
      wireframe: true, 
      transparent: true, 
      opacity: 0.1 // 거대한 콘이 방을 꽉 채우므로 투명도를 많이 낮춤
    });

    // 3. 스마트 타겟팅 라인 (조준선 레이저) 추가 기능은 coneMesh 렌더링 이후에 별도 추가
"""
if old_mat_line in content:
    content = content.replace(old_mat_line, new_mat_line)

# Add the laser targeting line to the speaker group
target_marker = "spkGroup.add(coneMesh);"
laser_code = """
    spkGroup.add(coneMesh);

    // --- 스마트 타겟팅 라인 (조준선 레이저) ---
    const laserMat = new THREE.LineBasicMaterial({ 
      color: 0xff3333, // 기본은 빨간색 (타겟 빗나감) - Update 로직에서 초록색으로 바뀔 예정
      transparent: true, 
      opacity: 0.8,
      linewidth: 2
    });
    const laserPoints = [];
    laserPoints.push(new THREE.Vector3(0, 0, 0)); // 스피커 중심
    laserPoints.push(new THREE.Vector3(0, -dynamicConeLength, 0)); // Z축 앞쪽으로 길게 뻗음 (스피커 로컬 좌표계는 -Y 방향이 앞으로 정렬되어 있음)
    const laserGeo = new THREE.BufferGeometry().setFromPoints(laserPoints);
    const laserLine = new THREE.Line(laserGeo, laserMat);
    laserLine.userData.isLaser = true;
    spkGroup.add(laserLine);
"""

if target_marker in content:
    content = content.replace(target_marker, laser_code)

# Add logic in animate loop to change laser color based on targeting
animate_marker = "controls.update();"
raycast_logic = """
      controls.update();

      // --- 스마트 타겟팅 레이저 색상 업데이트 로직 ---
      // 스피커가 마네킹 귀(listenerGroup)를 정확히 향하는지 검사
      speakerMeshes.forEach(spkGroup => {
        let laserLine = null;
        spkGroup.children.forEach(child => {
           if (child.userData.isLaser) laserLine = child;
        });

        if (laserLine) {
          // 스피커의 현재 월드 위치와 앞쪽(Forward) 방향 벡터 계산
          const spkWorldPos = new THREE.Vector3();
          spkGroup.getWorldPosition(spkWorldPos);
          
          const forwardDir = new THREE.Vector3(0, -1, 0); // 스피커가 바라보는 로컬 앞쪽
          forwardDir.applyQuaternion(spkGroup.quaternion).normalize();

          // 마네킹 귀(listenerGroup)의 월드 위치 (로컬 0,0,0이 귀 높이에 맞춰져 있음)
          const listenerPos = new THREE.Vector3();
          listenerGroup.getWorldPosition(listenerPos);
          // listenerGroup의 위치 자체가 바닥(Y=0)에 있고 귀 높이만큼 올라가 있지 않으므로 
          // 현재 Ear Level 높이만큼 더해준 위치가 진짜 목표점입니다.
          const targetEarPos = new THREE.Vector3(listenerPos.x, currentEarLevel, listenerPos.z);

          // 스피커에서 귀까지의 방향 벡터
          const dirToEar = new THREE.Vector3().subVectors(targetEarPos, spkWorldPos).normalize();

          // 스피커가 바라보는 방향과 귀 방향의 각도(내적) 계산
          const dot = forwardDir.dot(dirToEar);
          const angle = Math.acos(dot) * (180 / Math.PI);

          // 지향각(Dispersion) 오차 허용 범위 (예: ±15도 이내면 초록색)
          if (angle < 15.0) {
            laserLine.material.color.setHex(0x22c55e); // Green (Hit)
            laserLine.material.opacity = 1.0;
          } else {
            laserLine.material.color.setHex(0xef4444); // Red (Miss)
            laserLine.material.opacity = 0.3; // 빗나갔을 때는 눈에 덜 띄게 투명도 낮춤
          }
        }
      });
"""
if animate_marker in content:
    content = content.replace(animate_marker, raycast_logic)


with open(path, 'w') as f:
    f.write(content)
print("Dynamic Cone & Smart Targeting Laser applied.")
