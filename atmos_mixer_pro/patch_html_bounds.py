import re

def main():
    path = "assets/3d_simulator/studio_engine.html"
    with open(path, "r") as f:
        content = f.read()

    # Modify dragging clamp logic
    old_clamp = """        // Clamp to room bounds
        const halfW = currentRoom.width / 2;
        const halfD = currentRoom.depth / 2;
        newPos.x = Math.max(-halfW, Math.min(halfW, newPos.x));
        newPos.z = Math.max(-halfD, Math.min(halfD, newPos.z));"""

    new_clamp = """        // Clamp to room bounds (account for 0.2m speaker mesh radius)
        const halfW = (currentRoom.width / 2) - 0.2;
        const halfD = (currentRoom.depth / 2) - 0.2;
        newPos.x = Math.max(-halfW, Math.min(halfW, newPos.x));
        newPos.z = Math.max(-halfD, Math.min(halfD, newPos.z));"""

    content = content.replace(old_clamp, new_clamp)

    # Disable dragging for fixed speakers
    old_pointerdown_check = """        if (root && root.userData && root.userData.speakerId) {
          const clickedId = root.userData.speakerId;
          selectedSpeakerId = clickedId;"""

    new_pointerdown_check = """        if (root && root.userData && root.userData.speakerId) {
          const clickedId = root.userData.speakerId;
          const isFixed = root.userData.isFixed;
          selectedSpeakerId = clickedId;"""
          
    old_drag_set = """          draggedSpeakerNode = root;
          controls.enabled = false;
          dragPlane.constant = -root.position.y;"""
          
    new_drag_set = """          if (!isFixed) {
            draggedSpeakerNode = root;
            controls.enabled = false;
            dragPlane.constant = -root.position.y;
            const intersectPoint = new THREE.Vector3();
            raycaster.ray.intersectPlane(dragPlane, intersectPoint);
            if (intersectPoint) {
              dragOffset.copy(intersectPoint).sub(root.position);
            }
          }"""

    # We need to correctly patch the drag set without breaking existing logic
    # Let's use re.sub for safety
    content = re.sub(
        r"draggedSpeakerNode = root;\s*controls\.enabled = false;\s*dragPlane\.constant = -root\.position\.y;\s*const intersectPoint = new THREE\.Vector3\(\);\s*raycaster\.ray\.intersectPlane\(dragPlane, intersectPoint\);\s*if \(intersectPoint\) \{\s*dragOffset\.copy\(intersectPoint\)\.sub\(root\.position\);\s*\}",
        "if (!isFixed) {\n            draggedSpeakerNode = root;\n            controls.enabled = false;\n            dragPlane.constant = -root.position.y;\n            const intersectPoint = new THREE.Vector3();\n            raycaster.ray.intersectPlane(dragPlane, intersectPoint);\n            if (intersectPoint) {\n              dragOffset.copy(intersectPoint).sub(root.position);\n            }\n          }",
        content
    )
    
    content = content.replace(old_pointerdown_check, new_pointerdown_check)
    
    # Also pass isFixed in createSpeaker3DMesh
    old_userdata = """      group.userData = { speakerId: speakerData.id };"""
    new_userdata = """      group.userData = { speakerId: speakerData.id, isFixed: speakerData.isFixed };"""
    content = content.replace(old_userdata, new_userdata)

    with open(path, "w") as f:
        f.write(content)

main()
