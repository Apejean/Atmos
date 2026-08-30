import re

def main():
    path = "assets/3d_simulator/studio_engine.html"
    with open(path, "r") as f:
        content = f.read()

    old_pointerdown = """    window.addEventListener('pointerdown', (e) => {
      mouse.x = (e.clientX / window.innerWidth) * 2 - 1;
      mouse.y = -(e.clientY / window.innerHeight) * 2 + 1;

      raycaster.setFromCamera(mouse, camera);
      const intersects = raycaster.intersectObjects(speakersGroup.children, true);

      if (intersects.length > 0) {
        let root = intersects[0].object;
        while (root && root.parent && root.parent !== speakersGroup) {
          root = root.parent;
        }
        if (root && root.userData && root.userData.speakerId) {
          const clickedId = root.userData.speakerId;
          selectedSpeakerId = clickedId;
          window.updateScene({ speakers: currentSpeakers, selectedSpeakerId: clickedId });

          if (window.SpeakerBridge) {
            window.SpeakerBridge.postMessage(JSON.stringify({
              type: 'SPEAKER_SELECTED',
              speakerId: clickedId
            }));
          }
        }
      }
    });"""

    new_drag_logic = """    let draggedSpeakerNode = null;
    let dragOffset = new THREE.Vector3();
    let dragPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);

    window.addEventListener('pointerdown', (e) => {
      mouse.x = (e.clientX / window.innerWidth) * 2 - 1;
      mouse.y = -(e.clientY / window.innerHeight) * 2 + 1;

      raycaster.setFromCamera(mouse, camera);
      const intersects = raycaster.intersectObjects(speakersGroup.children, true);

      if (intersects.length > 0) {
        let root = intersects[0].object;
        while (root && root.parent && root.parent !== speakersGroup) {
          root = root.parent;
        }
        if (root && root.userData && root.userData.speakerId) {
          const clickedId = root.userData.speakerId;
          selectedSpeakerId = clickedId;
          window.updateScene({ speakers: currentSpeakers, selectedSpeakerId: clickedId });

          if (window.SpeakerBridge) {
            window.SpeakerBridge.postMessage(JSON.stringify({
              type: 'SPEAKER_SELECTED',
              speakerId: clickedId
            }));
          }

          draggedSpeakerNode = root;
          controls.enabled = false;
          dragPlane.constant = -root.position.y;
          const intersectPoint = new THREE.Vector3();
          raycaster.ray.intersectPlane(dragPlane, intersectPoint);
          if (intersectPoint) {
            dragOffset.copy(intersectPoint).sub(root.position);
          }
        }
      }
    });

    window.addEventListener('pointermove', (e) => {
      if (!draggedSpeakerNode) return;
      mouse.x = (e.clientX / window.innerWidth) * 2 - 1;
      mouse.y = -(e.clientY / window.innerHeight) * 2 + 1;
      raycaster.setFromCamera(mouse, camera);

      const intersectPoint = new THREE.Vector3();
      raycaster.ray.intersectPlane(dragPlane, intersectPoint);
      if (intersectPoint) {
        const newPos = intersectPoint.sub(dragOffset);
        
        // Clamp to room bounds
        const halfW = currentRoom.width / 2;
        const halfD = currentRoom.depth / 2;
        newPos.x = Math.max(-halfW, Math.min(halfW, newPos.x));
        newPos.z = Math.max(-halfD, Math.min(halfD, newPos.z));
        
        draggedSpeakerNode.position.x = newPos.x;
        draggedSpeakerNode.position.z = newPos.z;
        
        // The beam (if visible) is attached to the speaker so it moves with it
        // We can send live updates to flutter or just wait for pointerup
        if (window.SpeakerBridge) {
          window.SpeakerBridge.postMessage(JSON.stringify({
            type: 'SPEAKER_DRAGGING',
            speakerId: draggedSpeakerNode.userData.speakerId,
            x: newPos.x + currentRoom.width / 2,
            y: newPos.z + currentRoom.depth / 2
          }));
        }
      }
    });

    window.addEventListener('pointerup', (e) => {
      if (draggedSpeakerNode) {
        controls.enabled = true;
        const finalX = draggedSpeakerNode.position.x + currentRoom.width / 2;
        const finalY = draggedSpeakerNode.position.z + currentRoom.depth / 2;
        
        if (window.SpeakerBridge) {
          window.SpeakerBridge.postMessage(JSON.stringify({
            type: 'SPEAKER_MOVED',
            speakerId: draggedSpeakerNode.userData.speakerId,
            x: finalX,
            y: finalY
          }));
        }
        draggedSpeakerNode = null;
      }
    });"""

    content = content.replace(old_pointerdown, new_drag_logic)
    with open(path, "w") as f:
        f.write(content)

main()
