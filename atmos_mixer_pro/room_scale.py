import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# I will add the scaling logic back carefully!
# In <model-viewer>, we can use style="transform: scaleX(...) scaleY(...) scaleZ(...);" 
# Actually, the proper way to scale in model-viewer is via scale="x y z"

old_model_viewer = """          Positioned.fill(
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
          ),"""

new_model_viewer = """          Positioned.fill(
            child: ModelViewer(
              key: ValueKey('room_3d_viewer_${widget.activeRoom?.id ?? "def"}'), // Only recreate when room ID changes
              src: 'assets/models/room_with_listener.glb', // Contains the listener mannequin
              alt: '3D Room Space',
              autoRotate: false,
              cameraControls: true, // Native zoom and pan!
              shadowIntensity: 0.6,
              shadowSoftness: 0.8,
              exposure: 1.1,
              backgroundColor: const Color(0xFF0E131A),
              cameraOrbit: '45deg 65deg 6.5m',
              minCameraOrbit: 'auto auto 1.5m',
              maxCameraOrbit: 'auto auto 25m',
              fieldOfView: '35deg',
              interactionPrompt: InteractionPrompt.none,
              innerModelViewerHtml: '<model-viewer scale="${roomWidth / 6.0} ${roomHeight / 3.0} ${roomDepth / 4.5}"></model-viewer>',
            ),
          ),"""

content = content.replace(old_model_viewer, new_model_viewer)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
