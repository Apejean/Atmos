import re

def main():
    path = "assets/3d_simulator/studio_engine.html"
    with open(path, "r") as f:
        content = f.read()

    # Add global isSnapEnabled
    if "window.isSnapEnabled =" not in content:
        content = content.replace(
            "let isDragging = false;",
            "let isDragging = false;\n    window.isSnapEnabled = false;"
        )
        
    # Update dragging logic to snap
    old_clamp = """        // Clamp to room bounds (account for 0.2m speaker mesh radius)
        const halfW = (currentRoom.width / 2) - 0.2;
        const halfD = (currentRoom.depth / 2) - 0.2;
        newPos.x = Math.max(-halfW, Math.min(halfW, newPos.x));
        newPos.z = Math.max(-halfD, Math.min(halfD, newPos.z));"""

    new_clamp = """        // Clamp to room bounds (account for 0.2m speaker mesh radius)
        const halfW = (currentRoom.width / 2) - 0.2;
        const halfD = (currentRoom.depth / 2) - 0.2;
        
        if (window.isSnapEnabled) {
          newPos.x = Math.round(newPos.x * 10) / 10;
          newPos.z = Math.round(newPos.z * 10) / 10;
        }
        
        newPos.x = Math.max(-halfW, Math.min(halfW, newPos.x));
        newPos.z = Math.max(-halfD, Math.min(halfD, newPos.z));"""

    content = content.replace(old_clamp, new_clamp)

    with open(path, "w") as f:
        f.write(content)

main()
