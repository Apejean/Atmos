import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:model_viewer_plus/model_viewer_plus.dart';

import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';

class Dynamic3DRoom extends ConsumerStatefulWidget {
  final Function(String)? onSpeakerTapped;
  final String? selectedSpeakerId;
  final RoomZone? activeRoom;
  final bool showHeatmap;
  final VoidCallback? onOpenRoomSetup;

  const Dynamic3DRoom({
    super.key,
    this.onSpeakerTapped,
    this.selectedSpeakerId,
    this.activeRoom,
    this.showHeatmap = false,
    this.onOpenRoomSetup,
  });

  @override
  ConsumerState<Dynamic3DRoom> createState() => _Dynamic3DRoomState();
}


class HeatmapPainter extends CustomPainter {
  final List<SpeakerNode> speakers;
  final double roomWidth;
  final double roomDepth;

  HeatmapPainter(this.speakers, this.roomWidth, this.roomDepth);

  @override
  void paint(Canvas canvas, Size size) {
    if (speakers.isEmpty || roomWidth == 0 || roomDepth == 0) return;

    final double scaleX = size.width / roomWidth;
    final double scaleY = size.height / roomDepth;

    for (final spk in speakers) {
      final double projX = (spk.x * scaleX);
      final double projY = (spk.y * scaleY);
      
      final rect = Rect.fromCircle(center: Offset(projX, projY), radius: size.width * 0.4);
      final paint = Paint()
        ..shader = RadialGradient(
          colors: [
            Colors.red.withValues(alpha: 0.6),
            Colors.orange.withValues(alpha: 0.4),
            Colors.green.withValues(alpha: 0.2),
            Colors.blue.withValues(alpha: 0.05),
            Colors.transparent,
          ],
          stops: const [0.0, 0.3, 0.6, 0.8, 1.0],
        ).createShader(rect)
        ..blendMode = BlendMode.screen;
      
      canvas.drawRect(rect, paint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => true;
}

class _Dynamic3DRoomState extends ConsumerState<Dynamic3DRoom> {
  double _cameraDistance = 6.5;
  double _basePinchDistance = 6.5;
  double _yaw = 45.0;
  double _pitch = 65.0;

  void _handlePointerSignal(PointerSignalEvent event) {
    if (event is PointerScrollEvent) {
      // Handles Windows Mouse Wheel & Mac 2-Finger Trackpad Scroll
      final delta = event.scrollDelta.dy;
      if (delta != 0) {
        setState(() {
          // Smooth zoom scale: scroll up (negative) -> zoom in, scroll down (positive) -> zoom out
          final zoomFactor = delta > 0 ? 1.08 : 0.92;
          _cameraDistance = (_cameraDistance * zoomFactor).clamp(1.5, 25.0);
        });
      }
    }
  }

  void _handleScaleStart(ScaleStartDetails details) {
    _basePinchDistance = _cameraDistance;
  }

  void _handleScaleUpdate(ScaleUpdateDetails details) {
    // 1. Mac Trackpad Pinch-to-Zoom
    if (details.scale != 1.0) {
      setState(() {
        _cameraDistance = (_basePinchDistance / details.scale).clamp(1.5, 25.0);
      });
    }
    // 2. Trackpad / Mouse Drag Orbit Rotation
    else if (details.focalPointDelta.dx != 0 || details.focalPointDelta.dy != 0) {
      setState(() {
        _yaw = (_yaw - details.focalPointDelta.dx * 0.4) % 360;
        _pitch = (_pitch - details.focalPointDelta.dy * 0.3).clamp(5.0, 85.0);
      });
    }
  }

  void _resetCamera() {
    setState(() {
      _cameraDistance = 6.5;
      _yaw = 45.0;
      _pitch = 65.0;
    });
  }

  @override
  Widget build(BuildContext context) {
    final allSpeakers = ref.watch(speakerLayoutProvider);
    final speakers = allSpeakers.where((s) => s.roomId == null || s.roomId == widget.activeRoom?.id).toList();
    final bp = ref.watch(blueprintProvider);

    final roomWidth = widget.activeRoom?.physicalWidth ?? bp.canvasWidthMeters;
    final roomDepth = widget.activeRoom?.physicalHeight ?? bp.canvasHeightMeters;
    final roomHeight = widget.activeRoom?.ceilingHeight ?? 3.0;
    final roomLabel = widget.activeRoom?.label ?? 'Room 1';

    final orbitString = '${_yaw.toStringAsFixed(0)}deg ${_pitch.toStringAsFixed(0)}deg ${_cameraDistance.toStringAsFixed(1)}m';

    return Scaffold(
      backgroundColor: const Color(0xFF0E131A),
      body: Stack(
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

          // 2. Top-Left Room & Viewport Info Badge
          Positioned(
            top: 68,
            left: 16,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
              decoration: BoxDecoration(
                color: const Color(0xFF161E28).withValues(alpha: 0.85),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
                boxShadow: [
                  BoxShadow(
                    color: Colors.black.withValues(alpha: 0.3),
                    blurRadius: 10,
                  ),
                ],
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(Icons.view_in_ar_rounded, size: 16, color: Colors.lightBlueAccent),
                  const SizedBox(width: 8),
                  Text(
                    '$roomLabel: ${roomWidth.toStringAsFixed(1)}m × ${roomDepth.toStringAsFixed(1)}m × ${roomHeight.toStringAsFixed(1)}m | 4×4 Grid',
                    style: const TextStyle(
                      color: Colors.white70,
                      fontSize: 12,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  const SizedBox(width: 10),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(
                      color: Colors.black.withValues(alpha: 0.3),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      'Zoom: ${_cameraDistance.toStringAsFixed(1)}m',
                      style: const TextStyle(
                        color: Colors.lightBlueAccent,
                        fontSize: 10,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),

          // 3. Bottom Speaker Quick Selection Bar
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
          ),

          // 4. Floating Action Button: Add Speaker
          Positioned(
            bottom: 24,
            right: widget.selectedSpeakerId != null ? 360 : 24,
            child: FloatingActionButton.extended(
              onPressed: () {
                final newId = 'spk_${DateTime.now().millisecondsSinceEpoch}';
                final nextChannel = speakers.isEmpty
                    ? 0
                    : (speakers.map((s) => s.channel).reduce((a, b) => a > b ? a : b) + 1);
                final newNode = SpeakerNode(
                  id: newId,
                  roomId: widget.activeRoom?.id,
                  x: roomWidth / 2,
                  y: roomDepth / 2,
                  heightZ: 1.5,
                  channel: nextChannel,
                );
                ref.read(speakerLayoutProvider.notifier).addSpeaker(newNode);
                if (widget.onSpeakerTapped != null) {
                  widget.onSpeakerTapped!(newId);
                }
              },
              backgroundColor: const Color(0xFF2563EB),
              foregroundColor: Colors.white,
              elevation: 6,
              icon: const Icon(Icons.add_rounded),
              label: const Text(
                'Add Speaker',
                style: TextStyle(fontWeight: FontWeight.bold),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
