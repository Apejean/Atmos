import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# Replace the leftover manual gesture logic with a clean ModelViewer that uses native controls.
broken_stack = """      body: Stack(
        children: [
          // 1. Core 3D Orbit View with Native Trackpad Pinch & Mouse Wheel Zoom
          Positioned.fill(
            child: Listener(
              onPointerSignal: _handlePointerSignal,
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onScaleStart: _handleScaleStart,
                onScaleUpdate: _handleScaleUpdate,
                onDoubleTap: _resetCamera,
                child: ModelViewer(
                  key: ValueKey('room_3d_viewer_${widget.activeRoom?.id ?? "def"}_${_cameraDistance.toStringAsFixed(1)}_${_yaw.toStringAsFixed(0)}_${_pitch.toStringAsFixed(0)}'),
                  src: 'assets/models/room_frame.glb',
                  alt: '3D Room Wireframe & 4x4 Grid',
                  autoRotate: false,
                  cameraControls: true,
                  shadowIntensity: 0.6,
                  shadowSoftness: 0.8,
                  exposure: 1.1,
                  backgroundColor: const Color(0xFF0E131A),
                  cameraOrbit: orbitString,
                  minCameraOrbit: 'auto auto 1.5m',
                  maxCameraOrbit: 'auto auto 25m',
                  fieldOfView: '35deg',
                  interactionPrompt: InteractionPrompt.none,
                ),
              ),
            ),
          ),"""

fixed_stack = """      body: Stack(
        children: [
          // 1. Core 3D Orbit View (Native WebGL Controls)
          Positioned.fill(
            child: ModelViewer(
              key: ValueKey('room_3d_viewer_${widget.activeRoom?.id ?? "def"}'), // Static key so it never flashes
              src: 'assets/models/room_with_listener.glb', // User mannequin
              alt: '3D Room Space with Listener Mannequin',
              autoRotate: false,
              cameraControls: true,
              shadowIntensity: 0.6,
              shadowSoftness: 0.8,
              exposure: 1.1,
              backgroundColor: const Color(0xFF0E131A),
              cameraOrbit: '45deg 65deg 6.5m',
              minCameraOrbit: 'auto auto 1.5m',
              maxCameraOrbit: 'auto auto 25m',
              fieldOfView: '35deg',
              interactionPrompt: InteractionPrompt.none,
              // scale 속성 주입 (GLB 내부 렌더러에 의해 방 비율이 조정됨)
              innerModelViewerHtml: '<model-viewer scale="${roomWidth / 6.0} ${roomHeight / 3.0} ${roomDepth / 4.5}"',
            ),
          ),"""

if broken_stack in content:
    content = content.replace(broken_stack, fixed_stack)
    content = content.replace("final orbitString = '${_yaw.toStringAsFixed(0)}deg ${_pitch.toStringAsFixed(0)}deg ${_cameraDistance.toStringAsFixed(1)}m';", "")

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
