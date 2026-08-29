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

  const Dynamic3DRoom({
    super.key,
    this.onSpeakerTapped,
    this.selectedSpeakerId,
    this.activeRoom,
  });

  @override
  ConsumerState<Dynamic3DRoom> createState() => _Dynamic3DRoomState();
}

class _Dynamic3DRoomState extends ConsumerState<Dynamic3DRoom> {
  double _cameraDistance = 6.5;
  String _cameraAngle = '45deg 65deg';
  bool _isTopView = false;

  void _zoomIn() {
    setState(() {
      _cameraDistance = (_cameraDistance - 1.0).clamp(2.0, 15.0);
    });
  }

  void _zoomOut() {
    setState(() {
      _cameraDistance = (_cameraDistance + 1.0).clamp(2.0, 15.0);
    });
  }

  void _resetCamera() {
    setState(() {
      _cameraDistance = 6.5;
      _cameraAngle = '45deg 65deg';
      _isTopView = false;
    });
  }

  void _toggleTopView() {
    setState(() {
      _isTopView = !_isTopView;
      _cameraAngle = _isTopView ? '0deg 5deg' : '45deg 65deg';
    });
  }

  @override
  Widget build(BuildContext context) {
    final speakers = ref.watch(speakerLayoutProvider);
    final bp = ref.watch(blueprintProvider);

    final roomWidth = widget.activeRoom?.physicalWidth ?? bp.canvasWidthMeters;
    final roomDepth = widget.activeRoom?.physicalHeight ?? bp.canvasHeightMeters;
    final roomHeight = widget.activeRoom?.ceilingHeight ?? 3.0;
    final roomLabel = widget.activeRoom?.label ?? 'Room 1';

    final orbitString = '$_cameraAngle ${_cameraDistance.toStringAsFixed(1)}m';

    return Scaffold(
      backgroundColor: const Color(0xFF0E131A),
      body: Stack(
        children: [
          // 1. Core 3D Orbit View: 3D Wireframe Room + 4x4 Grid (Without Mannequin)
          Positioned.fill(
            child: ModelViewer(
              key: ValueKey('room_3d_viewer_${widget.activeRoom?.id ?? "def"}_${_cameraDistance.toStringAsFixed(1)}_$_cameraAngle'),
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
              maxCameraOrbit: 'auto auto 20m',
              fieldOfView: '35deg',
              interactionPrompt: InteractionPrompt.none,
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
                ],
              ),
            ),
          ),

          // 3. Right-Side Zoom & Camera Navigation Control Pod
          Positioned(
            top: 68,
            right: widget.selectedSpeakerId != null ? 360 : 16,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 6),
              decoration: BoxDecoration(
                color: const Color(0xFF161E28).withValues(alpha: 0.90),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.white.withValues(alpha: 0.12)),
                boxShadow: [
                  BoxShadow(
                    color: Colors.black.withValues(alpha: 0.4),
                    blurRadius: 10,
                    offset: const Offset(0, 4),
                  ),
                ],
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  // Zoom In Button (+)
                  _buildNavIconButton(
                    icon: Icons.add_rounded,
                    tooltip: 'Zoom In',
                    onTap: _zoomIn,
                  ),
                  const SizedBox(height: 4),

                  // Current Distance / Zoom Scale
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                    child: Text(
                      '${_cameraDistance.toStringAsFixed(1)}m',
                      style: const TextStyle(
                        color: Colors.white60,
                        fontSize: 10,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ),
                  const SizedBox(height: 4),

                  // Zoom Out Button (-)
                  _buildNavIconButton(
                    icon: Icons.remove_rounded,
                    tooltip: 'Zoom Out',
                    onTap: _zoomOut,
                  ),

                  const Divider(color: Colors.white12, height: 12, indent: 4, endIndent: 4),

                  // Reset Camera View (⟲)
                  _buildNavIconButton(
                    icon: Icons.restart_alt_rounded,
                    tooltip: 'Reset View',
                    onTap: _resetCamera,
                  ),
                  const SizedBox(height: 4),

                  // Top-Down / 3D Toggle
                  _buildNavIconButton(
                    icon: _isTopView ? Icons.view_in_ar_rounded : Icons.grid_view_rounded,
                    tooltip: _isTopView ? 'Switch to 3D Orbit' : 'Switch to Top View',
                    color: _isTopView ? Colors.lightBlueAccent : Colors.white70,
                    onTap: _toggleTopView,
                  ),
                ],
              ),
            ),
          ),

          // 4. Bottom Speaker Quick Selection Bar
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

          // 5. Floating Action Button: Add Speaker
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

  Widget _buildNavIconButton({
    required IconData icon,
    required String tooltip,
    required VoidCallback onTap,
    Color color = Colors.white70,
  }) {
    return Tooltip(
      message: tooltip,
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(6),
          child: Container(
            width: 34,
            height: 34,
            alignment: Alignment.center,
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(6),
              border: Border.all(color: Colors.white.withValues(alpha: 0.08)),
            ),
            child: Icon(icon, color: color, size: 18),
          ),
        ),
      ),
    );
  }
}
