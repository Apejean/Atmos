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

  const Dynamic3DRoom({
    super.key,
    this.onSpeakerTapped,
    this.selectedSpeakerId,
    this.activeRoom,
    this.showHeatmap = false,
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
      // Very basic isometric projection approximation
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
  @override
  Widget build(BuildContext context) {
    final allSpeakers = ref.watch(speakerLayoutProvider);
    final speakers = allSpeakers.where((s) => s.roomId == null || s.roomId == widget.activeRoom?.id).toList();
    final bp = ref.watch(blueprintProvider);

    final roomWidth = widget.activeRoom?.physicalWidth ?? bp.canvasWidthMeters;
    final roomDepth = widget.activeRoom?.physicalHeight ?? bp.canvasHeightMeters;
    final roomHeight = widget.activeRoom?.ceilingHeight ?? 3.0;
    final roomLabel = widget.activeRoom?.label ?? 'Room 1';

    return Scaffold(
      backgroundColor: const Color(0xFF0E131A),
      body: Stack(
        children: [
          // 1. Core 3D Orbit View: 3D Wireframe Room + 4x4 Floor Grid + Centered Listener Mannequin (W/2, D/2, 1.2m)
          Positioned.fill(
            child: ModelViewer(
              key: ValueKey('room_3d_viewport_${widget.activeRoom?.id ?? "def"}'),
              src: 'assets/models/room_with_listener.glb',
              alt: '3D Room Space with Listener Mannequin',
              autoRotate: false,
              cameraControls: true,
              shadowIntensity: 0.6,
              shadowSoftness: 0.8,
              exposure: 1.1,
              backgroundColor: const Color(0xFF0E131A),
              cameraOrbit: '45deg 65deg 6.5m',
              minCameraOrbit: 'auto auto 2.0m',
              maxCameraOrbit: 'auto auto 25.0m',
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

          // 2. Top-Left Room & Viewport Info Badge

          Positioned(
            top: 16,
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
                    '$roomLabel: ${roomWidth.toStringAsFixed(1)}m × ${roomDepth.toStringAsFixed(1)}m × ${roomHeight.toStringAsFixed(1)}m | Listener at (${(roomWidth / 2).toStringAsFixed(1)}, ${(roomDepth / 2).toStringAsFixed(1)}, 1.2m)',
                    style: const TextStyle(
                      color: Colors.white70,
                      fontSize: 12,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ],
              ),
            ),
          ),

          // 3. Floating Action Button: Add Speaker (Bottom Right)
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
