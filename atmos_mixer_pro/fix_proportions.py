import os

path = 'assets/3d_simulator/studio_engine.html'
with open(path, 'r') as f:
    content = f.read()

start_marker = "function renderListenerOBJ(obj, earLevel) {"
end_marker = "function buildListenerMannequin(earLevel = 1.2) {"

start_idx = content.find(start_marker)
end_idx = content.find(end_marker)

if start_idx == -1 or end_idx == -1:
    print("Markers not found!")
    exit(1)

replacement = """function renderListenerOBJ(obj, earLevel) {
      // 1. 로직 프로 스타일 무광 스튜디오 그레이 머티리얼 적용
      const dummyMat = new THREE.MeshStandardMaterial({
        color: 0x8a9ba8,
        roughness: 0.55,
        metalness: 0.05
      });
      obj.traverse((child) => {
        if (child.isMesh) {
          child.material = dummyMat;
          child.castShadow = true;
          child.receiveShadow = true;
        }
      });

      // 2. 바운딩 박스 기반 크기 계산
      const box = new THREE.Box3().setFromObject(obj);
      const size = new THREE.Vector3();
      box.getSize(size);
      const center = new THREE.Vector3();
      box.getCenter(center);

      // 3. 완벽한 1:1 해부학적 비율 유지 (Uniform Scale)
      // 마네킹의 귀 위치(모델 최상단에서 약 7.3% 아래)를 earLevel에 정확히 맞추도록
      // '전체 마네킹'을 왜곡 없이 비례해서 줄이거나 키웁니다.
      const localEarY = size.y * 0.927;
      const scale = earLevel / localEarY;

      // 찌그러짐, 찢어짐 없는 100% 균일 스케일 적용
      obj.scale.set(scale, scale, scale);

      // 4. 발을 정확히 Y=0 바닥(Sweet Spot Ring)에 고정
      obj.position.set(-center.x * scale, -box.min.y * scale, -center.z * scale);
      listenerGroup.add(obj);
      console.log(`[ATMOS 3D] Uniform Mannequin Rendered: earLevel=${earLevel}m, totalHeight=${(size.y * scale).toFixed(2)}m`);
    }

    """

new_content = content[:start_idx] + replacement + content[end_idx:]

with open(path, 'w') as f:
    f.write(new_content)
print("Mannequin uniform scale patch applied.")
