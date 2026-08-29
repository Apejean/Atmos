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
          // 1. Core 3D Orbit View
          Positioned.fill(
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
              scale: '${roomWidth / 6.0} ${roomHeight / 3.0} ${roomDepth / 4.5}',
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
                    '$roomLabel: ${roomWidth.toStringAsFixed(1)}m × ${roomDepth.toStringAsFixed(1)}m × ${roomHeight.toStringAsFixed(1)}m | 4x4 Grid',
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
                      'Zoom: Auto',
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

          // 3. Bottom-Left Room Setup Button
          Positioned(
            bottom: 24,
            left: 24,
            child: OutlinedButton.icon(
              icon: const Icon(Icons.tune_rounded, size: 16, color: Colors.lightBlueAccent),
              label: const Text(
                'ROOM SETUP',
                style: TextStyle(
                  fontSize: 13,
                  fontWeight: FontWeight.bold,
                  letterSpacing: 0.5,
                ),
              ),
              style: OutlinedButton.styleFrom(
                foregroundColor: Colors.lightBlueAccent,
                backgroundColor: const Color(0xFF161E28).withValues(alpha: 0.95),
                side: BorderSide(color: Colors.lightBlueAccent.withValues(alpha: 0.7), width: 1.5),
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                elevation: 6,
              ),
              onPressed: () {
                if (widget.onOpenRoomSetup != null) {
                  widget.onOpenRoomSetup!();
                }
              },
            ),
          ),

          // 4. Bottom-Right Floating Action Button: Add Speaker
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
