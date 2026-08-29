import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/features/exhibition/utils/glb_scaler.dart';
import 'dart:math' as math;
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
  String? _cameraOrbit;
  String _selectedView = 'Auto';
  String? _localGlbPath;
  double _lastW = -1, _lastD = -1, _lastH = -1;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _checkAndGenerateGlb();
  }

  @override
  void didUpdateWidget(Dynamic3DRoom oldWidget) {
    super.didUpdateWidget(oldWidget);
    _checkAndGenerateGlb();
  }

  Future<void> _checkAndGenerateGlb() async {
    final w = widget.activeRoom?.physicalWidth ?? 6.0;
    final d = widget.activeRoom?.physicalHeight ?? 4.5;
    final h = widget.activeRoom?.ceilingHeight ?? 3.0;
    
    if (w == _lastW && d == _lastD && h == _lastH && _localGlbPath != null) return;
    
    _lastW = w; _lastD = d; _lastH = h;
    final path = await GlbScaler.generateScaledRoom(w / 4.016, h / 2.616, d / 4.016);
    if (mounted) {
      setState(() {
        _localGlbPath = path;
      });
    }
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

    final maxDim = math.max(roomWidth, roomDepth);
    final orbitDist = (maxDim * 1.8).toStringAsFixed(1);




    // Build Hotspots for Speakers
    String innerHtml = '';
    for (var i = 0; i < speakers.length; i++) {
      final s = speakers[i];
      // physicalWidth maps to X. x=0 is left, x=width is right. Center is width/2
      final posX = (s.x / roomWidth * widget.activeRoom!.physicalWidth) - (widget.activeRoom!.physicalWidth / 2);
      final posZ = (s.y / roomDepth * widget.activeRoom!.physicalHeight) - (widget.activeRoom!.physicalHeight / 2);
      final posY = s.heightZ;
      
      final isSelected = widget.selectedSpeakerId == s.id;
      final color = isSelected ? '#ff3b30' : '#00ffff'; // Neon Blue
      final glow = isSelected ? '0 0 10px #ff3b30' : '0 0 8px #00ffff';
      
      innerHtml += '''
        <button slot="hotspot-spk-${s.id}" data-position="${posX}m ${posY}m ${posZ}m" data-normal="0 1 0" 
          style="
            background-color: rgba(14, 19, 26, 0.9); 
            border: 1px solid ${color}; 
            box-shadow: ${glow};
            border-radius: 6px; 
            padding: 4px 8px; 
            color: white; 
            font-family: sans-serif; 
            font-size: 11px; 
            font-weight: bold;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 4px;
            pointer-events: none;
          ">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="${color}" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon>
            <path d="M19.07 4.93a10 10 0 0 1 0 14.14M15.54 8.46a5 5 0 0 1 0 7.07"></path>
          </svg>
          CH ${s.channel}
        </button>
      ''';
    }

    return Scaffold(
      backgroundColor: const Color(0xFF0E131A),
      body: Stack(
        children: [
          // 1. Core 3D Orbit View
          Positioned.fill(
            child: _localGlbPath == null ? const Center(child: CircularProgressIndicator()) : GestureDetector(
              onDoubleTap: () {
                setState(() {
                  _selectedView = 'Auto';
                  _cameraOrbit = null; // Revert to dynamic auto calculation
                });
              },
              child: ModelViewer(
              key: ValueKey('room_3d_viewer_${widget.activeRoom?.id ?? "def"}_${roomWidth}_${roomDepth}_${roomHeight}'), // Recreate when scale changes
              src: _localGlbPath != null ? 'file://${_localGlbPath}' : 'assets/models/room_with_listener.glb', // Contains the listener mannequin
              alt: '3D Room Space',
              autoRotate: false,
              cameraControls: true, // Native zoom and pan!
              shadowIntensity: 0.6,
              shadowSoftness: 0.8,
              exposure: 1.1,
              backgroundColor: const Color(0xFF0E131A),
              cameraOrbit: _cameraOrbit ?? '45deg 65deg ${orbitDist}m',
              minCameraOrbit: 'auto auto 1.5m',
              maxCameraOrbit: 'auto auto 2000m',
              fieldOfView: 'auto',
              interactionPrompt: InteractionPrompt.none,
              innerModelViewerHtml: innerHtml,
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
          // Heatmap Overlay
          if (widget.showHeatmap)
            Positioned.fill(
              child: IgnorePointer(
                child: CustomPaint(
                  painter: HeatmapPainter(speakers, roomWidth, roomDepth),
                ),
              ),
            ),

          // 2. Top-Left Room Setup & Info Badge
          Positioned(
            top: 16,
            left: 16,
            child: Row(
              children: [
                OutlinedButton.icon(
                  icon: const Icon(Icons.tune_rounded, size: 16, color: Colors.lightBlueAccent),
                  label: const Text(
                    'ROOM SETUP',
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.bold,
                      letterSpacing: 0.5,
                    ),
                  ),
                  style: OutlinedButton.styleFrom(
                    foregroundColor: Colors.lightBlueAccent,
                    backgroundColor: const Color(0xFF161E28).withValues(alpha: 0.95),
                    side: BorderSide(color: Colors.lightBlueAccent.withValues(alpha: 0.7), width: 1.2),
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
                const SizedBox(width: 16),
                Container(
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
                      const SizedBox(width: 12),
                      Container(
                        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 0),
                        height: 24,
                        decoration: BoxDecoration(
                          color: Colors.black.withValues(alpha: 0.3),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: DropdownButtonHideUnderline(
                          child: DropdownButton<String>(
                            value: _selectedView,
                            icon: const Icon(Icons.arrow_drop_down, color: Colors.lightBlueAccent, size: 16),
                            dropdownColor: const Color(0xFF1B232E),
                            style: const TextStyle(color: Colors.lightBlueAccent, fontSize: 11, fontWeight: FontWeight.bold),
                            items: const [
                              DropdownMenuItem(value: 'Auto', child: Text('Zoom: Auto')),
                              DropdownMenuItem(value: 'Front', child: Text('Front View')),
                              DropdownMenuItem(value: 'Back', child: Text('Back View')),
                              DropdownMenuItem(value: 'Side(L)', child: Text('Left View')),
                              DropdownMenuItem(value: 'Side(R)', child: Text('Right View')),
                              DropdownMenuItem(value: 'Top', child: Text('Top View')),
                            ],
                            onChanged: (val) {
                              if (val == null) return;
                              final w = widget.activeRoom?.physicalWidth ?? 6.0;
                              final d = widget.activeRoom?.physicalHeight ?? 4.5;
                              final maxDim = w > d ? w : d;
                              final orbitDist = (maxDim * 1.8).toStringAsFixed(1);
                              setState(() {
                                _selectedView = val;
                                final r = orbitDist;
                                switch(val) {
                                  case 'Auto': _cameraOrbit = null; break;
                                  case 'Front': _cameraOrbit = '0deg 85deg auto'; break;
                                  case 'Back': _cameraOrbit = '180deg 85deg auto'; break;
                                  case 'Side(L)': _cameraOrbit = '90deg 85deg auto'; break;
                                  case 'Side(R)': _cameraOrbit = '-90deg 85deg auto'; break;
                                  case 'Top': _cameraOrbit = '0deg 0deg auto'; break;
                                }
                              });
                            },
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
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
