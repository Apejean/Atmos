import 'package:atmos_mixer_pro/features/exhibition/widgets/room_zone_widget.dart';
import 'dart:ui';
import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;

class SpeakerCanvasScreen extends ConsumerStatefulWidget {
  const SpeakerCanvasScreen({super.key});

  @override
  ConsumerState<SpeakerCanvasScreen> createState() => _SpeakerCanvasScreenState();
}

class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> with SingleTickerProviderStateMixin {
  final TransformationController _transformationController = TransformationController();
  final FocusNode _canvasFocusNode = FocusNode();
  String? _inspectorSpeakerId;
  bool _showHeatmap = false;

  @override
  void initState() {
    super.initState();
    _transformationController.value = Matrix4.identity()
      ..setTranslationRaw(200, 150, 0.0);
  }

  @override
  void dispose() {
    _canvasFocusNode.dispose();
    _transformationController.dispose();
    super.dispose();
  }

  void _onSpeakerTap(String id) {
    setState(() {
      _inspectorSpeakerId = id;
    });
  }

  @override
  Widget build(BuildContext context) {
    final blueprint = ref.watch(blueprintProvider);
    final config = ref.watch(configProvider);
    final uiRooms = config?.rooms ?? [];
    final tabLength = uiRooms.isEmpty ? 1 : uiRooms.length;

    // The dark sci-fi background color
    final bgColor = const Color(0xFF0F111A);
    final panelColor = const Color(0xFF191D26);
    final borderColor = const Color(0xFF2C3240);
    final neonCyan = const Color(0xFF33D1FF);

    return DefaultTabController(
      length: tabLength,
      child: Builder(builder: (context) {
        final tabController = DefaultTabController.of(context);
        tabController.addListener(() {
          if (tabController.indexIsChanging && _inspectorSpeakerId != null) {
            setState(() { _inspectorSpeakerId = null; });
          }
        });

        return Scaffold(
          backgroundColor: bgColor,
          body: Column(
            children: [
              // HEADER (Top Bar)
              Container(
                height: 70,
                padding: const EdgeInsets.symmetric(horizontal: 24),
                decoration: BoxDecoration(
                  color: panelColor,
                  border: Border(bottom: BorderSide(color: borderColor)),
                ),
                child: Row(
                  children: [
                    // LOGO
                    Row(
                      children: [
                        Icon(Icons.graphic_eq, color: neonCyan, size: 28),
                        const SizedBox(width: 8),
                        Column(
                          mainAxisAlignment: MainAxisAlignment.center,
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: const [
                            Text('3D AUDIO', style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 16, height: 1.0)),
                            Text('SIMULATOR', style: TextStyle(color: Colors.white70, fontSize: 14, height: 1.0)),
                          ],
                        ),
                      ],
                    ),
                    const Spacer(),
                    
                    // SPL HEATMAP
                    const Text('SPL HEATMAP', style: TextStyle(color: Colors.white70, fontSize: 12, fontWeight: FontWeight.bold)),
                    const SizedBox(width: 8),
                    Switch(
                      value: _showHeatmap,
                      activeColor: neonCyan,
                      activeTrackColor: neonCyan.withValues(alpha: 0.3),
                      inactiveThumbColor: Colors.grey,
                      inactiveTrackColor: Colors.white10,
                      onChanged: (v) => setState(() => _showHeatmap = v),
                    ),
                    const SizedBox(width: 24),
                    
                    // EXPORT PDF REPORT
                    OutlinedButton.icon(
                      onPressed: () {},
                      icon: Icon(Icons.description, color: neonCyan, size: 16),
                      label: Text('EXPORT PDF REPORT', style: TextStyle(color: neonCyan)),
                      style: OutlinedButton.styleFrom(
                        side: BorderSide(color: neonCyan),
                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(4)),
                        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                      ),
                    ),
                    const SizedBox(width: 24),
                    
                    // USER ICON
                    const CircleAvatar(
                      backgroundColor: Colors.white10,
                      child: Icon(Icons.person, color: Colors.white70),
                    ),
                  ],
                ),
              ),

              // TAB BAR
              Container(
                color: panelColor,
                child: Align(
                  alignment: Alignment.centerLeft,
                  child: TabBar(
                    isScrollable: true,
                    indicatorColor: neonCyan,
                    labelColor: neonCyan,
                    unselectedLabelColor: Colors.white54,
                    tabs: uiRooms.isEmpty 
                      ? [const Tab(text: 'Default Room')]
                      : uiRooms.map((r) => Tab(text: r.name)).toList(),
                  ),
                ),
              ),

              // MAIN CONTENT
              Expanded(
                child: Row(
                  children: [
                    // CANVAS AREA
                    Expanded(
                      child: Stack(
                        children: [
                          // Interactive Viewer for Canvas
                          GestureDetector(
                            onTap: () {
                              _canvasFocusNode.requestFocus();
                              setState(() => _inspectorSpeakerId = null);
                            },
                            child: Container(
                              color: bgColor,
                              child: InteractiveViewer(
                                transformationController: _transformationController,
                                minScale: 0.1,
                                maxScale: 10.0,
                                constrained: false,
                                child: Stack(
                                  clipBehavior: Clip.none,
                                  children: [
                                    // Base dimensions
                                    Container(
                                      width: 800,
                                      height: 600,
                                      color: Colors.transparent,
                                    ),
                                    // Custom Painter for Grid and Blueprint
                                    Positioned.fill(
                                      child: CustomPaint(
                                        painter: _SciFiGridPainter(neonCyan: neonCyan),
                                      ),
                                    ),
                                    
                                    // Heatmap
                                    if (_showHeatmap)
                                      Positioned.fill(
                                        child: Consumer(
                                          builder: (context, ref, child) {
                                            final nodes = ref.watch(speakerLayoutProvider);
                                            return CustomPaint(
                                              painter: _HeatmapPainter(
                                                speakers: nodes,
                                                scale: blueprint.scale,
                                              ),
                                            );
                                          },
                                        ),
                                      ),
                                    
                                    // Speakers
                                    Consumer(
                                      builder: (context, ref, child) {
                                        final nodes = ref.watch(speakerLayoutProvider);
                                        return Stack(
                                          children: nodes.map((node) {
                                            return Positioned(
                                              left: node.x * blueprint.scale - 30, // Offset for center
                                              top: node.y * blueprint.scale - 30,
                                              child: GestureDetector(
                                                onTap: () => _onSpeakerTap(node.id),
                                                child: _SciFiSpeakerWidget(
                                                  node: node,
                                                  isSelected: _inspectorSpeakerId == node.id,
                                                  neonCyan: neonCyan,
                                                ),
                                              ),
                                            );
                                          }).toList(),
                                        );
                                      },
                                    ),
                                    
                                    // Center Listener
                                    Positioned(
                                      left: 400 - 24, // Assuming 800x600 center
                                      top: 300 - 24,
                                      child: Container(
                                        width: 48,
                                        height: 48,
                                        decoration: BoxDecoration(
                                          shape: BoxShape.circle,
                                          color: neonCyan.withValues(alpha: 0.1),
                                          border: Border.all(color: neonCyan, width: 2),
                                        ),
                                        child: Center(
                                          child: Icon(Icons.person, color: neonCyan),
                                        ),
                                      ),
                                    ),
                                  ],
                                ),
                              ),
                            ),
                          ),
                          
                          // FLOATING ROOM SETUP
                          Positioned(
                            left: 24,
                            bottom: 24,
                            child: _RoomSetupPanel(neonCyan: neonCyan, panelColor: panelColor, borderColor: borderColor),
                          ),
                        ],
                      ),
                    ),
                    
                    // RIGHT SIDEBAR (SPEAKER INSPECTOR)
                    if (_inspectorSpeakerId != null)
                      Consumer(
                        builder: (context, ref, child) {
                          final nodes = ref.watch(speakerLayoutProvider);
                          final node = nodes.firstWhere((n) => n.id == _inspectorSpeakerId, orElse: () => nodes.first);
                          return _SpeakerInspectorPanel(
                            node: node,
                            neonCyan: neonCyan,
                            panelColor: panelColor,
                            borderColor: borderColor,
                            onClose: () => setState(() => _inspectorSpeakerId = null),
                          );
                        },
                      ),
                  ],
                ),
              ),
            ],
          ),
        );
      }),
    );
  }
}

// ============================================================================
// CUSTOM PAINTERS & WIDGETS
// ============================================================================

class _SciFiGridPainter extends CustomPainter {
  final Color neonCyan;
  _SciFiGridPainter({required this.neonCyan});

  @override
  void paint(Canvas canvas, Size size) {
    // Draw Dark Grid
    final gridPaint = Paint()
      ..color = Colors.white.withValues(alpha: 0.05)
      ..strokeWidth = 1.0;
      
    final step = 50.0;
    for (double x = 0; x <= size.width; x += step) {
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), gridPaint);
    }
    for (double y = 0; y <= size.height; y += step) {
      canvas.drawLine(Offset(0, y), Offset(size.width, y), gridPaint);
    }
    
    // Draw Outer Room Wireframe
    final roomPaint = Paint()
      ..color = Colors.white30
      ..strokeWidth = 2.0
      ..style = PaintingStyle.stroke;
    
    final roomRect = Rect.fromLTWH(100, 100, size.width - 200, size.height - 200);
    canvas.drawRect(roomRect, roomPaint);
    
    // Draw Corner Lines (Isometric depth illusion on 2D plane)
    canvas.drawLine(Offset(100, 100), Offset(130, 130), roomPaint);
    canvas.drawLine(Offset(size.width-100, 100), Offset(size.width-130, 130), roomPaint);
    canvas.drawLine(Offset(100, size.height-100), Offset(130, size.height-130), roomPaint);
    canvas.drawLine(Offset(size.width-100, size.height-100), Offset(size.width-130, size.height-130), roomPaint);
    
    final innerRect = Rect.fromLTWH(130, 130, size.width - 260, size.height - 260);
    canvas.drawRect(innerRect, roomPaint);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class _SciFiSpeakerWidget extends StatelessWidget {
  final SpeakerNode node;
  final bool isSelected;
  final Color neonCyan;

  const _SciFiSpeakerWidget({required this.node, required this.isSelected, required this.neonCyan});

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 60,
      height: 60,
      decoration: BoxDecoration(
        color: isSelected ? neonCyan.withValues(alpha: 0.2) : Colors.transparent,
        border: isSelected ? Border.all(color: neonCyan, width: 2) : null,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Stack(
        alignment: Alignment.center,
        children: [
          // Basic Box Representation
          Container(
            width: 30,
            height: 30,
            decoration: BoxDecoration(
              border: Border.all(color: neonCyan, width: 1.5),
              borderRadius: BorderRadius.circular(4),
            ),
            child: Center(
              child: Container(
                width: 15,
                height: 15,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  border: Border.all(color: neonCyan, width: 1.5),
                ),
              ),
            ),
          ),
          // Waves (simplified static)
          Positioned(
            right: 0,
            child: Icon(Icons.wifi, color: neonCyan, size: 20),
          ),
          // Label
          Positioned(
            top: -10,
            child: Text(
              node.id,
              style: TextStyle(color: neonCyan, fontSize: 10, fontWeight: FontWeight.bold),
            ),
          ),
        ],
      ),
    );
  }
}

class _RoomSetupPanel extends StatelessWidget {
  final Color neonCyan;
  final Color panelColor;
  final Color borderColor;

  const _RoomSetupPanel({required this.neonCyan, required this.panelColor, required this.borderColor});

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 250,
      decoration: BoxDecoration(
        color: panelColor.withValues(alpha: 0.9),
        border: Border.all(color: borderColor),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            decoration: BoxDecoration(
              border: Border(bottom: BorderSide(color: borderColor)),
            ),
            child: Row(
              children: [
                Icon(Icons.desktop_windows, color: neonCyan, size: 16),
                const SizedBox(width: 8),
                const Text('ROOM SETUP', style: TextStyle(color: Colors.white, fontSize: 12, fontWeight: FontWeight.bold)),
                const Spacer(),
                const Icon(Icons.close, color: Colors.white54, size: 16),
              ],
            ),
          ),
          Padding(
            padding: const EdgeInsets.all(12),
            child: Column(
              children: [
                _buildField('Width:', '6.0 m', neonCyan, borderColor),
                const SizedBox(height: 8),
                _buildField('Depth:', '4.5 m', neonCyan, borderColor),
                const SizedBox(height: 8),
                _buildField('Ceiling Height:', '3.0 m', neonCyan, borderColor),
                const SizedBox(height: 8),
                _buildField('Ear Level:', '1.2 m', neonCyan, borderColor),
              ],
            ),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                TextButton(
                  onPressed: () {},
                  child: const Text('Apply', style: TextStyle(color: Colors.white)),
                ),
                TextButton(
                  onPressed: () {},
                  child: const Text('Cancel', style: TextStyle(color: Colors.white54)),
                ),
              ],
            ),
          )
        ],
      ),
    );
  }

  Widget _buildField(String label, String val, Color neon, Color border) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(label, style: const TextStyle(color: Colors.white70, fontSize: 12)),
        Container(
          width: 80,
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
          decoration: BoxDecoration(
            border: Border.all(color: neon),
            borderRadius: BorderRadius.circular(4),
          ),
          child: Text(val, style: const TextStyle(color: Colors.white, fontSize: 12), textAlign: TextAlign.right),
        ),
      ],
    );
  }
}

class _SpeakerInspectorPanel extends ConsumerStatefulWidget {
  final SpeakerNode node;
  final Color neonCyan;
  final Color panelColor;
  final Color borderColor;
  final VoidCallback onClose;

  const _SpeakerInspectorPanel({
    required this.node,
    required this.neonCyan,
    required this.panelColor,
    required this.borderColor,
    required this.onClose,
  });

  @override
  ConsumerState<_SpeakerInspectorPanel> createState() => _SpeakerInspectorPanelState();
}

class _SpeakerInspectorPanelState extends ConsumerState<_SpeakerInspectorPanel> {
  late double h;
  late double t;
  late double p;

  @override
  void initState() {
    super.initState();
    h = widget.node.heightZ;
    t = widget.node.pitchTilt;
    p = widget.node.rotation;
  }

  @override
  void didUpdateWidget(covariant _SpeakerInspectorPanel oldWidget) {
    if (oldWidget.node.id != widget.node.id) {
      h = widget.node.heightZ;
      t = widget.node.pitchTilt;
      p = widget.node.rotation;
    }
    super.didUpdateWidget(oldWidget);
  }

  void _sync() {
    ref.read(speakerLayoutProvider.notifier).updateSpeaker(
      widget.node.copyWith(heightZ: h, pitchTilt: t, rotation: p)
    );
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 300,
      decoration: BoxDecoration(
        color: widget.panelColor,
        border: Border(left: BorderSide(color: widget.borderColor)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header
          Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Icon(Icons.volume_up, color: widget.neonCyan, size: 20),
                const SizedBox(width: 8),
                const Text('SPEAKER INSPECTOR', style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
              ],
            ),
          ),
          const Divider(color: Colors.white10, height: 1),
          
          Expanded(
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: [
                Text('Selected Speaker: ${widget.node.id}', style: const TextStyle(color: Colors.white70)),
                const SizedBox(height: 24),

                // Height Z
                const Text('HEIGHT (Z)', style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
                const SizedBox(height: 16),
                Container(
                  padding: const EdgeInsets.all(16),
                  decoration: BoxDecoration(
                    border: Border.all(color: widget.borderColor),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Row(
                    children: [
                      // Vertical Slider fake
                      RotatedBox(
                        quarterTurns: 3,
                        child: Slider(
                          value: h, min: 0, max: 20,
                          activeColor: widget.neonCyan,
                          onChanged: (v) { setState(() => h = v); _sync(); },
                        ),
                      ),
                      const Spacer(),
                      Column(
                        children: [
                          Icon(Icons.unfold_more, color: widget.neonCyan, size: 32),
                          const SizedBox(height: 8),
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                            decoration: BoxDecoration(border: Border.all(color: widget.neonCyan), borderRadius: BorderRadius.circular(4)),
                            child: Text('${h.toStringAsFixed(1)} m', style: const TextStyle(color: Colors.white)),
                          ),
                        ],
                      )
                    ],
                  ),
                ),
                const SizedBox(height: 24),

                // Tilt Angle
                const Text('TILT ANGLE', style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
                const SizedBox(height: 16),
                Container(
                  padding: const EdgeInsets.all(16),
                  decoration: BoxDecoration(
                    border: Border.all(color: widget.borderColor),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Column(
                    children: [
                      Slider(
                        value: t, min: -90, max: 90,
                        activeColor: widget.neonCyan,
                        onChanged: (v) { setState(() => t = v); _sync(); },
                      ),
                      Row(
                        mainAxisAlignment: MainAxisAlignment.end,
                        children: [
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                            decoration: BoxDecoration(border: Border.all(color: widget.neonCyan), borderRadius: BorderRadius.circular(4)),
                            child: Text('${t.toStringAsFixed(0)}°', style: const TextStyle(color: Colors.white)),
                          ),
                        ],
                      )
                    ],
                  ),
                ),
                const SizedBox(height: 24),

                // Pan Angle
                const Text('PAN ANGLE', style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
                const SizedBox(height: 16),
                Container(
                  padding: const EdgeInsets.all(16),
                  decoration: BoxDecoration(
                    border: Border.all(color: widget.borderColor),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Column(
                    children: [
                      Slider(
                        value: p, min: -180, max: 180,
                        activeColor: widget.neonCyan,
                        onChanged: (v) { setState(() => p = v); _sync(); },
                      ),
                      Row(
                        mainAxisAlignment: MainAxisAlignment.end,
                        children: [
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                            decoration: BoxDecoration(border: Border.all(color: widget.neonCyan), borderRadius: BorderRadius.circular(4)),
                            child: Text('${p.toStringAsFixed(0)}°', style: const TextStyle(color: Colors.white)),
                          ),
                        ],
                      )
                    ],
                  ),
                ),
              ],
            ),
          ),
          
          // Bottom Buttons
          Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Expanded(child: OutlinedButton(onPressed: (){}, style: OutlinedButton.styleFrom(side: const BorderSide(color: Colors.white54)), child: const Text('Mute', style: TextStyle(color: Colors.white)))),
                const SizedBox(width: 8),
                Expanded(child: OutlinedButton(onPressed: (){}, style: OutlinedButton.styleFrom(side: const BorderSide(color: Colors.white54)), child: const Text('Solo', style: TextStyle(color: Colors.white)))),
                const SizedBox(width: 8),
                Expanded(flex: 2, child: OutlinedButton(onPressed: (){}, style: OutlinedButton.styleFrom(side: const BorderSide(color: Colors.white54)), child: const Text('Remove Speaker', style: TextStyle(color: Colors.white, fontSize: 12)))),
              ],
            ),
          )
        ],
      ),
    );
  }
}

class _HeatmapPainter extends CustomPainter {
  final List<SpeakerNode> speakers;
  final double scale;

  _HeatmapPainter({required this.speakers, required this.scale});

  @override
  void paint(Canvas canvas, Size size) {
    if (speakers.isEmpty) return;
    for (var spk in speakers) {
      final center = Offset(spk.x * scale, spk.y * scale);
      final radius = 200.0;
      final rect = Rect.fromCircle(center: center, radius: radius);
      
      final gradient = RadialGradient(
        colors: [
          Colors.blue.withValues(alpha: 0.8),
          Colors.cyan.withValues(alpha: 0.4),
          Colors.transparent,
        ],
        stops: const [0.0, 0.4, 1.0],
      );

      final paint = Paint()
        ..shader = gradient.createShader(rect)
        ..blendMode = BlendMode.screen;

      canvas.drawCircle(center, radius, paint);
    }
  }

  @override
  bool shouldRepaint(covariant _HeatmapPainter oldDelegate) => true;
}
