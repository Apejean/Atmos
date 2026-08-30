import re

def main():
    path = "assets/3d_simulator/studio_engine.html"
    with open(path, "r") as f:
        content = f.read()

    # 1. Update mannequin listener (ear position, tilt, head shape)
    old_mannequin = """    // --- 5. 3D Listener Mannequin Bust Builder ---
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
      rightEar.position.set(0.13, earY, 0);"""
      
    new_mannequin = """    // --- 5. 3D Listener Mannequin Bust Builder ---
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
      const headCenterY = earLevel + 0.07;
      const noseY = earLevel + 0.04;
      const earY = earLevel;
      const neckY = earLevel - 0.10;
      const bustY = earLevel - 0.22;

      // Head (Anatomical shape: wider back, jawline)
      const headGeo = new THREE.SphereGeometry(0.14, 32, 24);
      headGeo.scale(0.85, 1.15, 1.1); // Slightly deeper for occipital volume
      const headMesh = new THREE.Mesh(headGeo, mannequinMat);
      // Move head slightly backward so nose and face don't stick out too much
      headMesh.position.set(0, headCenterY, -0.015);

      // Nose (Forward indicator along +Z)
      const noseGeo = new THREE.ConeGeometry(0.025, 0.06, 16);
      noseGeo.rotateX(Math.PI / 2);
      const noseMesh = new THREE.Mesh(noseGeo, mannequinMat);
      noseMesh.position.set(0, noseY, 0.14);

      // Left Pinna Ear (Moved back, tilted)
      const earGeo = new THREE.SphereGeometry(0.035, 16, 16);
      earGeo.scale(0.3, 1.2, 0.6);
      const leftEar = new THREE.Mesh(earGeo, mannequinMat);
      leftEar.position.set(-0.13, earY, -0.025);
      leftEar.rotateX(0.2); // ~11-12 degrees tilt

      // Right Pinna Ear (Moved back, tilted)
      const rightEar = new THREE.Mesh(earGeo, mannequinMat);
      rightEar.position.set(0.13, earY, -0.025);
      rightEar.rotateX(0.2); // ~11-12 degrees tilt"""
      
    content = content.replace(old_mannequin, new_mannequin)

    # 2. Fix Drag clamping, isFixed, pointer events
    # Looking for raycaster and drag logic
    # Also need to make sure dragging doesn't freeze flutter
    
    # We will replace the whole pointer event section
    old_pointer_down = """    renderer.domElement.addEventListener('pointerdown', (e) => {
      const rect = renderer.domElement.getBoundingClientRect();
      mouse.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      mouse.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
      raycaster.setFromCamera(mouse, camera);

      const intersects = raycaster.intersectObjects(scene.children, true);
      let clickedId = null;

      for (let i = 0; i < intersects.length; i++) {
        let root = intersects[i].object;
        while (root.parent && root.parent !== scene) root = root.parent;
        
        if (root.userData && root.userData.speakerId) {
          const isFixed = root.userData.isFixed;
          if (!isFixed) {
            isDragging = true;
            draggedSpeakerNode = root;
            controls.enabled = false; // Disable orbit while dragging
            
            dragPlane.position.y = draggedSpeakerNode.position.y;
            const planeIntersects = raycaster.intersectObject(dragPlane);
            if (planeIntersects.length > 0) {
              dragOffset.copy(planeIntersects[0].point).sub(draggedSpeakerNode.position);
            }
          }
          clickedId = root.userData.speakerId;
          break; // Stop at first speaker
        }
      }

      if (clickedId !== null) {
        if (window.onSpeakerTapped) window.onSpeakerTapped(clickedId);
        // Post message for flutter
        if (window.flutter_inappwebview) {
           window.flutter_inappwebview.callHandler('SPEAKER_TAPPED', clickedId);
        } else {
           console.log('SPEAKER_TAPPED:', clickedId);
        }
      }
    });"""

    old_pointer_move = """    renderer.domElement.addEventListener('pointermove', (e) => {
      if (!isDragging || !draggedSpeakerNode) return;
      
      const rect = renderer.domElement.getBoundingClientRect();
      mouse.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      mouse.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
      raycaster.setFromCamera(mouse, camera);

      const planeIntersects = raycaster.intersectObject(dragPlane);
      if (planeIntersects.length > 0) {
        const newPos = planeIntersects[0].point.sub(dragOffset);
        
        // Clamp to room bounds (account for 0.2m speaker mesh radius)
        const halfW = (currentRoom.width / 2) - 0.2;
        const halfD = (currentRoom.depth / 2) - 0.2;
        
        if (window.isSnapEnabled) {
          newPos.x = Math.round(newPos.x * 10) / 10;
          newPos.z = Math.round(newPos.z * 10) / 10;
        }
        
        newPos.x = Math.max(-halfW, Math.min(halfW, newPos.x));
        newPos.z = Math.max(-halfD, Math.min(halfD, newPos.z));
        
        draggedSpeakerNode.position.x = newPos.x;
        draggedSpeakerNode.position.z = newPos.z;

        // Post message to flutter that it moved
        const payload = JSON.stringify({
           id: draggedSpeakerNode.userData.speakerId,
           x: newPos.x,
           y: -newPos.z  // Map back to Dart's Y-up 2D coordinate system
        });
        
        if (window.flutter_inappwebview) {
           window.flutter_inappwebview.callHandler('SPEAKER_MOVED', payload);
        }
      }
    });"""

    old_pointer_up = """    renderer.domElement.addEventListener('pointerup', () => {
      isDragging = false;
      draggedSpeakerNode = null;
      controls.enabled = true;
    });"""

    new_pointer_events = """
    // Pointer Dragging and State Logic
    let pointerDownTime = 0;
    
    renderer.domElement.addEventListener('pointerdown', (e) => {
      const rect = renderer.domElement.getBoundingClientRect();
      mouse.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      mouse.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
      raycaster.setFromCamera(mouse, camera);

      const intersects = raycaster.intersectObjects(scene.children, true);
      let clickedId = null;
      let clickedSpeakerMesh = null;
      pointerDownTime = Date.now();

      for (let i = 0; i < intersects.length; i++) {
        let root = intersects[i].object;
        while (root.parent && root.parent !== scene) root = root.parent;
        
        if (root.userData && root.userData.speakerId) {
          clickedId = root.userData.speakerId;
          clickedSpeakerMesh = root;
          break; // Stop at first speaker
        }
      }

      if (clickedId !== null) {
        // Find the actual model data to check true isFixed state
        const spkData = currentSpeakers.find(s => s.id === clickedId);
        const isFixed = spkData ? spkData.isFixed : root.userData.isFixed;
        
        if (!isFixed) {
          isDragging = true;
          draggedSpeakerNode = clickedSpeakerMesh;
          controls.enabled = false; // Disable orbit while dragging
          
          dragPlane.position.y = draggedSpeakerNode.position.y;
          const planeIntersects = raycaster.intersectObject(dragPlane);
          if (planeIntersects.length > 0) {
            dragOffset.copy(planeIntersects[0].point).sub(draggedSpeakerNode.position);
          }
        }
        
        if (window.onSpeakerTapped) window.onSpeakerTapped(clickedId);
        // Post message for flutter
        if (window.flutter_inappwebview) {
           window.flutter_inappwebview.callHandler('SPEAKER_TAPPED', clickedId);
        } else {
           console.log('SPEAKER_TAPPED:', clickedId);
        }
      }
    });

    let throttleDragTimer = null;

    renderer.domElement.addEventListener('pointermove', (e) => {
      if (!isDragging || !draggedSpeakerNode) return;
      
      const rect = renderer.domElement.getBoundingClientRect();
      mouse.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      mouse.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
      raycaster.setFromCamera(mouse, camera);

      const planeIntersects = raycaster.intersectObject(dragPlane);
      if (planeIntersects.length > 0) {
        const newPos = planeIntersects[0].point.sub(dragOffset);
        
        // Clamp to room bounds (0.25m margin as requested)
        const margin = 0.25;
        const halfW = (currentRoom.width / 2) - margin;
        const halfD = (currentRoom.depth / 2) - margin;
        
        if (window.isSnapEnabled) {
          newPos.x = Math.round(newPos.x * 10) / 10;
          newPos.z = Math.round(newPos.z * 10) / 10;
        }
        
        newPos.x = Math.max(-halfW, Math.min(halfW, newPos.x));
        newPos.z = Math.max(-halfD, Math.min(halfD, newPos.z));
        
        draggedSpeakerNode.position.x = newPos.x;
        draggedSpeakerNode.position.z = newPos.z;

        // Visual only updates immediately
        
        // Flutter state update throttled/debounced (Only send SPEAKER_DRAGGING for light UI updates)
        if (!throttleDragTimer && window.flutter_inappwebview) {
          throttleDragTimer = setTimeout(() => {
            const payload = JSON.stringify({
              id: draggedSpeakerNode.userData.speakerId,
              x: draggedSpeakerNode.position.x,
              y: -draggedSpeakerNode.position.z
            });
            window.flutter_inappwebview.callHandler('SPEAKER_DRAGGING', payload);
            throttleDragTimer = null;
          }, 100); // 10fps throttle for dragging to prevent freeze
        }
      }
    });
    
    function handlePointerRelease() {
      if (isDragging && draggedSpeakerNode) {
        // Send final authoritative state to Flutter on pointer up
        const payload = JSON.stringify({
           id: draggedSpeakerNode.userData.speakerId,
           x: draggedSpeakerNode.position.x,
           y: -draggedSpeakerNode.position.z
        });
        if (window.flutter_inappwebview) {
           window.flutter_inappwebview.callHandler('SPEAKER_MOVED', payload);
        }
      }
      isDragging = false;
      draggedSpeakerNode = null;
      controls.enabled = true;
    }

    renderer.domElement.addEventListener('pointerup', handlePointerRelease);
    renderer.domElement.addEventListener('pointerleave', handlePointerRelease);
    renderer.domElement.addEventListener('pointercancel', handlePointerRelease);
    window.addEventListener('blur', handlePointerRelease);
"""

    content = content.replace(old_pointer_down, "")
    content = content.replace(old_pointer_move, "")
    content = content.replace(old_pointer_up, new_pointer_events)

    # 3. Ensure window.updateScene exists because it errored in logs
    if "window.updateScene =" not in content:
        # Wait, the logs showed "updateScene is not defined". I need to find where it is or why it was missing.
        # Oh, the user's previous code might have had it removed or it was never there and called from Dart?
        print("updateScene is missing in JS?")

    with open(path, "w") as f:
        f.write(content)

main()
