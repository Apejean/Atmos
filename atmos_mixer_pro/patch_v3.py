with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# 1. Remove Ch1~Ch7 bottom tabs
ch_tabs = """          // 3. Bottom Speaker Quick Selection Bar
          Positioned(
            left: 200,
            bottom: 24,
            right: widget.selectedSpeakerId != null ? 360 : 180,
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Row(
                children: [
                  for (final spk in speakers) ...[
                    Padding(
                      padding: const EdgeInsets.only(right: 8.0),
                      child: ActionChip(
                        avatar: CircleAvatar(
                          backgroundColor: widget.selectedSpeakerId == spk.id
                              ? Colors.lightBlueAccent
                              : const Color(0xFF2A3A4D),
                          child: Text(
                            '${spk.channel + 1}',
                            style: TextStyle(
                              fontSize: 11,
                              fontWeight: FontWeight.bold,
                              color: widget.selectedSpeakerId == spk.id
                                  ? Colors.black
                                  : Colors.white70,
                            ),
                          ),
                        ),
                        label: Text(
                          'CH ${spk.channel + 1} (${spk.x.toStringAsFixed(1)}, ${spk.y.toStringAsFixed(1)}, ${spk.heightZ.toStringAsFixed(1)}m)',
                          style: TextStyle(
                            fontSize: 12,
                            color: widget.selectedSpeakerId == spk.id
                                ? Colors.lightBlueAccent
                                : Colors.white70,
                            fontWeight: widget.selectedSpeakerId == spk.id
                                ? FontWeight.bold
                                : FontWeight.normal,
                          ),
                        ),
                        backgroundColor: widget.selectedSpeakerId == spk.id
                            ? const Color(0xFF1A2B3D)
                            : const Color(0xFF131B24).withValues(alpha: 0.9),
                        side: BorderSide(
                          color: widget.selectedSpeakerId == spk.id
                              ? Colors.lightBlueAccent.withValues(alpha: 0.8)
                              : Colors.white.withValues(alpha: 0.1),
                        ),
                        onPressed: () {
                          if (widget.onSpeakerTapped != null) {
                            widget.onSpeakerTapped!(spk.id);
                          }
                        },
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),"""
content = content.replace(ch_tabs, '')

# 2. Fix the ModelViewer bug.
# Let's completely replace the whole GestureDetector / Listener block with just ModelViewer.
# We will use innerModelViewerHtml to scale it natively inside WebGL!

model_viewer_block = """          // 1. Core 3D Orbit View with Native Trackpad Pinch & Mouse Wheel Zoom
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

new_model_viewer_block = """          // 1. Core 3D Orbit View (Native WebGL Zoom/Pan)
          Positioned.fill(
            child: ModelViewer(
              key: ValueKey('room_3d_viewer_${widget.activeRoom?.id ?? "def"}'), // Static key so it never rebuilds/flashes
              src: 'assets/models/room_with_listener.glb', // Use the mannequin!
              alt: '3D Room Space with Listener Mannequin',
              autoRotate: false,
              cameraControls: true, // Natively handles smooth zoom/pan/orbit without setState
              shadowIntensity: 0.6,
              shadowSoftness: 0.8,
              exposure: 1.1,
              backgroundColor: const Color(0xFF0E131A),
              cameraOrbit: '45deg 65deg 6.5m', // Initial orbit
              minCameraOrbit: 'auto auto 1.5m',
              maxCameraOrbit: 'auto auto 25m',
              fieldOfView: '35deg',
              interactionPrompt: InteractionPrompt.none,
              // scale 속성 주입 (GLB 내부 렌더러에 의해 방 비율이 조정됨)
              innerModelViewerHtml: '<model-viewer scale="${roomWidth / 6.0} ${roomHeight / 3.0} ${roomDepth / 4.5}"',
            ),
          ),"""
content = content.replace(model_viewer_block, new_model_viewer_block)


# 3. Clean up the manual _cameraDistance / _yaw / _pitch variables
state_vars = """  double _cameraDistance = 7.4; // 6.5m * 1.14 (adjusted for tighter field of view)
  double _yaw = 45.0; // 45deg
  double _pitch = 65.0; // 65deg

  void _resetCamera() {
    setState(() {
      _cameraDistance = 7.4;
      _yaw = 45.0;
      _pitch = 65.0;
    });
  }

  void _handlePointerSignal(PointerSignalEvent event) {
    if (event is PointerScrollEvent) {
      // Native macOS trackpad & mouse wheel
      final delta = event.scrollDelta.dy;
      setState(() {
        _cameraDistance = (_cameraDistance + delta * 0.01).clamp(1.5, 25.0);
      });
    }
  }

  double _baseDistance = 7.4;

  void _handleScaleStart(ScaleStartDetails details) {
    _baseDistance = _cameraDistance;
  }

  void _handleScaleUpdate(ScaleUpdateDetails details) {
    if (details.scale == 1.0) {
      // Panning (rotate camera)
      setState(() {
        _yaw = (_yaw - details.focalPointDelta.dx * 0.5) % 360;
        _pitch = (_pitch - details.focalPointDelta.dy * 0.5).clamp(5.0, 175.0);
      });
    } else {
      // Pinch to zoom
      setState(() {
        _cameraDistance = (_baseDistance / details.scale).clamp(1.5, 25.0);
      });
    }
  }"""
content = content.replace(state_vars, '')
content = content.replace("final orbitString = '${_yaw.toStringAsFixed(0)}deg ${_pitch.toStringAsFixed(0)}deg ${_cameraDistance.toStringAsFixed(1)}m';", "")

# 4. Remove 'import 'package:flutter/gestures.dart';' if it exists
content = content.replace("import 'package:flutter/gestures.dart';", "")

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
