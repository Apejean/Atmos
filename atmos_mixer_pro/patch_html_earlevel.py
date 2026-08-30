import re

def main():
    path = "assets/3d_simulator/studio_engine.html"
    with open(path, "r") as f:
        content = f.read()

    # The earLevel is normally 1.2m
    # We want a function to update earLevel
    
    old_build = """    // --- 5. 3D Listener Mannequin Bust Builder ---
    function buildListenerMannequin() {
      while (listenerGroup.children.length > 0) {
        const obj = listenerGroup.children[0];
        if (obj.geometry) obj.geometry.dispose();
        listenerGroup.remove(obj);
      }

      const mannequinMat = new THREE.MeshStandardMaterial({
        color: 0x8ca0b9,
        roughness: 0.4,
        metalness: 0.1
      });

      // Head (Egg shape)
      const headGeo = new THREE.SphereGeometry(0.14, 32, 24);
      headGeo.scale(0.85, 1.15, 1.0);
      const headMesh = new THREE.Mesh(headGeo, mannequinMat);
      headMesh.position.set(0, 1.25, 0);

      // Nose (Forward indicator along +Z)
      const noseGeo = new THREE.ConeGeometry(0.025, 0.06, 16);
      noseGeo.rotateX(Math.PI / 2);
      const noseMesh = new THREE.Mesh(noseGeo, mannequinMat);
      noseMesh.position.set(0, 1.24, 0.14);

      // Left Pinna Ear
      const earGeo = new THREE.SphereGeometry(0.035, 16, 16);
      earGeo.scale(0.3, 1.2, 0.6);
      const leftEar = new THREE.Mesh(earGeo, mannequinMat);
      leftEar.position.set(-0.13, 1.25, 0);

      // Right Pinna Ear
      const rightEar = new THREE.Mesh(earGeo, mannequinMat);
      rightEar.position.set(0.13, 1.25, 0);

      // Neck
      const neckGeo = new THREE.CylinderGeometry(0.06, 0.07, 0.12, 24);
      const neckMesh = new THREE.Mesh(neckGeo, mannequinMat);
      neckMesh.position.set(0, 1.10, 0);

      // Bust / Shoulder Base
      const bustGeo = new THREE.CylinderGeometry(0.12, 0.22, 0.15, 32);
      bustGeo.scale(1.4, 1.0, 0.8);
      const bustMesh = new THREE.Mesh(bustGeo, mannequinMat);
      bustMesh.position.set(0, 0.98, 0);

      // Ground Sweet Spot Ring
      const ringGeo = new THREE.RingGeometry(0.35, 0.38, 32);
      ringGeo.rotateX(-Math.PI / 2);
      const ringMat = new THREE.MeshBasicMaterial({ color: 0x38bdf8, side: THREE.DoubleSide, transparent: true, opacity: 0.5 });
      const ringMesh = new THREE.Mesh(ringGeo, ringMat);
      ringMesh.position.set(0, 0.02, 0);

      listenerGroup.add(headMesh);
      listenerGroup.add(noseMesh);
      listenerGroup.add(leftEar);
      listenerGroup.add(rightEar);
      listenerGroup.add(neckMesh);
      listenerGroup.add(bustMesh);
      listenerGroup.add(ringMesh);
    }"""

    new_build = """    // --- 5. 3D Listener Mannequin Bust Builder ---
    function buildListenerMannequin(earLevel = 1.2) {
      while (listenerGroup.children.length > 0) {
        const obj = listenerGroup.children[0];
        if (obj.geometry) obj.geometry.dispose();
        listenerGroup.remove(obj);
      }

      const mannequinMat = new THREE.MeshStandardMaterial({
        color: 0x8ca0b9,
        roughness: 0.4,
        metalness: 0.1
      });

      // The ear hole is at Y = earLevel
      // Head center is slightly above ear level
      const headCenterY = earLevel + 0.05;
      const noseY = earLevel + 0.04;
      const earY = earLevel;
      const neckY = earLevel - 0.10;
      const bustY = earLevel - 0.22;

      // Head (Egg shape)
      const headGeo = new THREE.SphereGeometry(0.14, 32, 24);
      headGeo.scale(0.85, 1.15, 1.0);
      const headMesh = new THREE.Mesh(headGeo, mannequinMat);
      headMesh.position.set(0, headCenterY, 0);

      // Nose (Forward indicator along +Z)
      const noseGeo = new THREE.ConeGeometry(0.025, 0.06, 16);
      noseGeo.rotateX(Math.PI / 2);
      const noseMesh = new THREE.Mesh(noseGeo, mannequinMat);
      noseMesh.position.set(0, noseY, 0.14);

      // Left Pinna Ear
      const earGeo = new THREE.SphereGeometry(0.035, 16, 16);
      earGeo.scale(0.3, 1.2, 0.6);
      const leftEar = new THREE.Mesh(earGeo, mannequinMat);
      leftEar.position.set(-0.13, earY, 0);

      // Right Pinna Ear
      const rightEar = new THREE.Mesh(earGeo, mannequinMat);
      rightEar.position.set(0.13, earY, 0);

      // Neck
      const neckGeo = new THREE.CylinderGeometry(0.06, 0.07, 0.12, 24);
      const neckMesh = new THREE.Mesh(neckGeo, mannequinMat);
      neckMesh.position.set(0, neckY, 0);

      // Bust / Shoulder Base
      const bustGeo = new THREE.CylinderGeometry(0.12, 0.22, 0.15, 32);
      bustGeo.scale(1.4, 1.0, 0.8);
      const bustMesh = new THREE.Mesh(bustGeo, mannequinMat);
      bustMesh.position.set(0, bustY, 0);

      // Ground Sweet Spot Ring (always fixed near 0)
      const ringGeo = new THREE.RingGeometry(0.35, 0.38, 32);
      ringGeo.rotateX(-Math.PI / 2);
      const ringMat = new THREE.MeshBasicMaterial({ color: 0x38bdf8, side: THREE.DoubleSide, transparent: true, opacity: 0.5 });
      const ringMesh = new THREE.Mesh(ringGeo, ringMat);
      ringMesh.position.set(0, 0.02, 0);

      listenerGroup.add(headMesh);
      listenerGroup.add(noseMesh);
      listenerGroup.add(leftEar);
      listenerGroup.add(rightEar);
      listenerGroup.add(neckMesh);
      listenerGroup.add(bustMesh);
      listenerGroup.add(ringMesh);
    }
    
    // Global method to update ear level
    window.updateEarLevel = function(level) {
      buildListenerMannequin(level);
    };"""

    content = content.replace(old_build, new_build)

    with open(path, "w") as f:
        f.write(content)

main()
