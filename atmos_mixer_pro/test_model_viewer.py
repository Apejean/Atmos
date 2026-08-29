import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# Completely remove all gestures and just use ModelViewer with cameraControls: true
# Also let's fix the room setup button position

new_body = """      body: Stack(
        children: [
          // 1. Core 3D Orbit View
          Positioned.fill(
            child: ModelViewer(
              key: ValueKey('room_3d_viewer_${widget.activeRoom?.id ?? "def"}'),
              src: 'assets/models/room_with_listener.glb',
              alt: '3D Room Space',
              autoRotate: false,
              cameraControls: true, // Let WebGL handle the interactions!
              shadowIntensity: 0.6,
              shadowSoftness: 0.8,
              exposure: 1.1,
              backgroundColor: const Color(0xFF0E131A),
              cameraOrbit: '45deg 65deg 6.5m',
              minCameraOrbit: 'auto auto 1.5m',
              maxCameraOrbit: 'auto auto 25m',
              fieldOfView: '35deg',
              interactionPrompt: InteractionPrompt.none,
            ),
          ),

          // Heatmap Overlay
          if (widget.showHeatmap)
            Positioned.fill(
              child: IgnorePointer(
                child: CustomPaint(
                  painter: HeatmapPainter(speakers, roomWidth, roomDepth),
                ),
              ),
            ),
"""

# Replace the beginning of body: Stack(
content = re.sub(r'      body: Stack\(\n        children: \[\n          // 1\. Core 3D Orbit View.*?          // Heatmap Overlay', new_body + "          // Heatmap Overlay", content, flags=re.DOTALL)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
