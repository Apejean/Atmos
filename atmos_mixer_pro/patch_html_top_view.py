import re

def main():
    path = "assets/3d_simulator/studio_engine.html"
    with open(path, "r") as f:
        content = f.read()

    # Add toggleTopView function
    if "window.toggleTopView =" not in content:
        insert_code = """
    let isTopView = false;
    let savedCameraPos = new THREE.Vector3();
    let savedTarget = new THREE.Vector3();

    window.toggleTopView = function() {
      isTopView = !isTopView;
      if (isTopView) {
        // Save current state
        savedCameraPos.copy(camera.position);
        savedTarget.copy(controls.target);
        
        // Move to top view
        // The room max size is around 20-30m, so height of 20m should see everything
        const roomMaxDim = currentRoom ? Math.max(currentRoom.width, currentRoom.depth) : 10;
        const h = Math.max(roomMaxDim * 1.5, 10);
        camera.position.set(0, h, 0);
        controls.target.set(0, 0, 0);
        
        // Disable rotation so it stays 2D
        controls.enableRotate = false;
        // Optionally lock to orthographic style if wanted, but just top-down perspective is fine
      } else {
        // Restore state
        camera.position.copy(savedCameraPos);
        controls.target.copy(savedTarget);
        controls.enableRotate = true;
      }
      controls.update();
    };
"""
        # Insert before window.onload or at the end of the script
        content = content.replace(
            "function animate() {",
            insert_code + "\n    function animate() {"
        )

    with open(path, "w") as f:
        f.write(content)

main()
