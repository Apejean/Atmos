import os

path = 'assets/3d_simulator/studio_engine.html'
with open(path, 'r') as f:
    content = f.read()

# 1. Fix the Room diagonal lines
old_room = """      // 4.2 3D Room Bounding Box Wireframe
      const boxGeo = new THREE.BoxGeometry(w, h, d);
      const wireGeo = new THREE.WireframeGeometry(boxGeo);"""
new_room = """      // 4.2 3D Room Bounding Box Wireframe (EdgesGeometry를 사용하여 대각선 제거)
      const boxGeo = new THREE.BoxGeometry(w, h, d);
      const wireGeo = new THREE.EdgesGeometry(boxGeo);"""
content = content.replace(old_room, new_room)

# 2. Fix the Animate Loop for Laser Color Updates
old_anim = """    function animate() {
      requestAnimationFrame(animate);
      controls.update();
      renderer.render(scene, camera);
    }"""

new_anim = """    function animate() {
      requestAnimationFrame(animate);
      controls.update();

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
            laser.material.opacity = 0.25;
          }
        }
      });

      renderer.render(scene, camera);
    }"""
content = content.replace(old_anim, new_anim)

with open(path, 'w') as f:
    f.write(content)
print("Laser animate loop and Room edges fixed.")
