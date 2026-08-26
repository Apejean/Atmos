import 'dart:ui';
import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';

class SpeakerCanvasScreen extends ConsumerStatefulWidget {
  const SpeakerCanvasScreen({super.key});

  @override
  ConsumerState<SpeakerCanvasScreen> createState() => _SpeakerCanvasScreenState();
}

class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> with SingleTickerProviderStateMixin {
  final TransformationController _topViewController = TransformationController();
  final TransformationController _isoViewController = TransformationController();
  final FocusNode _canvasFocusNode = FocusNode();
  String? _inspectorSpeakerId;
  bool _showHeatmap = false;
  double _targetSPL = 75.0;
  
  double _orbitAngleX = math.pi / 4; // 45 degrees
  double _orbitAngleY = math.pi / 6; // 30 degrees tilt

  @override
  void initState() {
    super.initState();
    _topViewController.value = Matrix4.identity()..setTranslationRaw(0, 0, 0.0);
    _isoViewController.value = Matrix4.identity()..setTranslationRaw(0, 0, 0.0);
  }

  @override
  void dispose() {
    _canvasFocusNode.dispose();
    _topViewController.dispose();
    _isoViewController.dispose();
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

    final bgColor = const Color(0xFF0F111A);
    final panelColor = const Color(0xFF191D26);
    final borderColor = const Color(0xFF2C3240);
    final neonCyan = const Color(0xFF33D1FF);

    return DefaultTabController(
      length: tabLength,
      child: Builder(builder: (context) {
        return Scaffold(
          backgroundColor: bgColor,
          floatingActionButton: FloatingActionButton.extended(
            backgroundColor: neonCyan,
            icon: const Icon(Icons.add, color: Colors.black),
            label: const Text('Add Speaker', style: TextStyle(color: Colors.black, fontWeight: FontWeight.bold)),
            onPressed: () {
              ref.read(speakerLayoutProvider.notifier).addSpeaker(
                SpeakerNode(
                  id: DateTime.now().millisecondsSinceEpoch.toString(),
                  x: blueprint.canvasWidthMeters / 2,
                  y: blueprint.canvasHeightMeters / 2,
                  heightZ: blueprint.roomHeightMeters, 
                  channel: 0,
                ),
              );
            },
          ),
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
                    // BACK & LOGO
                    IconButton(
                      icon: const Icon(Icons.arrow_back, color: Colors.white),
                      onPressed: () => Navigator.of(context).pop(),
                    ),
                    const SizedBox(width: 8),
                    Icon(Icons.graphic_eq, color: neonCyan, size: 28),
                    const SizedBox(width: 8),
                    const Text(
                      'Atmos Mixer Pro',
                      style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 18),
                    ),
                    const Spacer(),
                    
                    // TARGET SPL
                    const Text('Target SPL:', style: TextStyle(color: Colors.white70, fontSize: 12)),
                    SizedBox(
                      width: 120,
                      child: Slider(
                        value: _targetSPL,
                        min: 50.0, max: 95.0,
                        activeColor: neonCyan,
                        onChanged: (v) => setState(() => _targetSPL = v),
                      ),
                    ),
                    Text('${_targetSPL.toStringAsFixed(1)} dB', style: const TextStyle(color: Colors.white, fontSize: 12)),
                    
                    const SizedBox(width: 24),
                    
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
                    
                    OutlinedButton.icon(
                      onPressed: () {},
                      icon: Icon(Icons.description, color: neonCyan, size: 16),
                      label: Text('EXPORT PDF REPORT', style: TextStyle(color: neonCyan)),
                      style: OutlinedButton.styleFrom(
                        side: BorderSide(color: neonCyan),
                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(4)),
                      ),
                    ),
                  ],
                ),
              ),

              // MAIN CONTENT
              Expanded(
                child: Row(
                  children: [
                    // DUAL VIEWPORT AREA
                    Expanded(
                      child: Column(
                        children: [
                          // 1. TOP VIEW (2D)
                          Expanded(
                            flex: 1,
                            child: Container(
                              decoration: BoxDecoration(
                                border: Border(bottom: BorderSide(color: borderColor)),
                              ),
                              child: Stack(
                                children: [
                                  InteractiveViewer(
                                    transformationController: _topViewController,
                                    boundaryMargin: const EdgeInsets.all(double.infinity),
                                    minScale: 0.1, maxScale: 10.0,
                                    constrained: false,
                                    child: SizedBox(
                                      width: 800, height: 600,
                                      child: Stack(
                                        children: [
                                          CustomPaint(
                                            size: const Size(800, 600),
                                            painter: _TopViewGridPainter(neonCyan: neonCyan, blueprint: blueprint),
                                          ),
                                          if (_showHeatmap)
                                            CustomPaint(
                                              size: const Size(800, 600),
                                              painter: _HeatmapPainter(
                                                speakers: ref.watch(speakerLayoutProvider),
                                                blueprint: blueprint,
                                                isTopView: true,
                                                targetSPL: _targetSPL,
                                              ),
                                            ),
                                          ...ref.watch(speakerLayoutProvider).map((node) {
                                            return Positioned(
                                              left: node.x * blueprint.scale - 20,
                                              top: node.y * blueprint.scale - 20,
                                              child: GestureDetector(
                                                onTap: () => _onSpeakerTap(node.id),
                                                onPanUpdate: (d) {
                                                  final newX = node.x + d.delta.dx / blueprint.scale;
                                                  final newY = node.y + d.delta.dy / blueprint.scale;
                                                  ref.read(speakerLayoutProvider.notifier).updateSpeaker(
                                                    node.copyWith(x: newX, y: newY)
                                                  );
                                                },
                                                child: _SciFiSpeakerWidget(
                                                  node: node,
                                                  isSelected: _inspectorSpeakerId == node.id,
                                                  neonCyan: neonCyan,
                                                ),
                                              ),
                                            );
                                          }).toList(),
                                        ],
                                      ),
                                    ),
                                  ),
                                  const Positioned(
                                    top: 10, left: 10,
                                    child: Text('TOP VIEW (XY)', style: TextStyle(color: Colors.white54, fontWeight: FontWeight.bold)),
                                  ),
                                ],
                              ),
                            ),
                          ),
                          
                          // 2. ISOMETRIC 3D VIEW (Orbit)
                          Expanded(
                            flex: 2,
                            child: GestureDetector(
                              onPanUpdate: (details) {
                                setState(() {
                                  _orbitAngleX += details.delta.dx * 0.01;
                                  _orbitAngleY -= details.delta.dy * 0.01;
                                  _orbitAngleY = _orbitAngleY.clamp(-math.pi/2, math.pi/2);
                                });
                              },
                              child: Stack(
                                children: [
                                  InteractiveViewer(
                                    transformationController: _isoViewController,
                                    boundaryMargin: const EdgeInsets.all(double.infinity),
                                    minScale: 0.1, maxScale: 10.0,
                                    constrained: false,
                                    child: SizedBox(
                                      width: 1200, height: 800,
                                      child: Stack(
                                        children: [
                                          CustomPaint(
                                            size: const Size(1200, 800),
                                            painter: _IsoViewPainter(
                                              neonCyan: neonCyan,
                                              blueprint: blueprint,
                                              orbitAngleX: _orbitAngleX,
                                              orbitAngleY: _orbitAngleY,
                                            ),
                                          ),
                                          if (_showHeatmap)
                                            CustomPaint(
                                              size: const Size(1200, 800),
                                              painter: _HeatmapPainter(
                                                speakers: ref.watch(speakerLayoutProvider),
                                                blueprint: blueprint,
                                                isTopView: false,
                                                orbitAngleX: _orbitAngleX,
                                                orbitAngleY: _orbitAngleY,
                                                targetSPL: _targetSPL,
                                              ),
                                            ),
                                          ...ref.watch(speakerLayoutProvider).map((node) {
                                            final proj = _IsoProjector(
                                              scale: blueprint.scale,
                                              cx: 600, cy: 400,
                                              roomW: blueprint.canvasWidthMeters,
                                              roomD: blueprint.canvasHeightMeters,
                                              angleX: _orbitAngleX, angleY: _orbitAngleY,
                                            ).project(node.x, node.y, node.heightZ);
                                            
                                            return Positioned(
                                              left: proj.dx - 20,
                                              top: proj.dy - 20,
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
                                        ],
                                      ),
                                    ),
                                  ),
                                  const Positioned(
                                    top: 10, left: 10,
                                    child: Text('3D ISOMETRIC VIEW (Drag to Orbit, Scroll to Zoom)', style: TextStyle(color: Colors.white54, fontWeight: FontWeight.bold)),
                                  ),
                                ],
                              ),
                            ),
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
                          return Align(
                            alignment: Alignment.centerRight,
                            child: SpeakerInspectorPanel(
                              selectedSpeaker: node,
                            ),
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

class _IsoProjector {
  final double scale;
  final double cx, cy;
  final double roomW, roomD;
  final double angleX, angleY;

  _IsoProjector({required this.scale, required this.cx, required this.cy, required this.roomW, required this.roomD, required this.angleX, required this.angleY});

  Offset project(double xMeters, double yMeters, double zMeters) {
    // Relative to center
    double dx = (xMeters - roomW / 2) * scale;
    double dy = (yMeters - roomD / 2) * scale;
    double dz = zMeters * scale;

    // Rotation around Z (Pan/OrbitX)
    double rx = dx * math.cos(angleX) - dy * math.sin(angleX);
    double ry = dx * math.sin(angleX) + dy * math.cos(angleX);

    // Rotation around X (Tilt/OrbitY)
    double sy = ry * math.cos(angleY) - dz * math.sin(angleY);
    double sz = ry * math.sin(angleY) + dz * math.cos(angleY); // actually projected out, but sz is depth

    return Offset(cx + rx, cy + sy);
  }
}

class _TopViewGridPainter extends CustomPainter {
  final Color neonCyan;
  final BlueprintData blueprint;

  _TopViewGridPainter({required this.neonCyan, required this.blueprint});

  @override
  void paint(Canvas canvas, Size size) {
    final paintLine = Paint()..color = Colors.white30..strokeWidth = 1.0..style = PaintingStyle.stroke;
    final w = blueprint.canvasWidthMeters * blueprint.scale;
    final h = blueprint.canvasHeightMeters * blueprint.scale;
    canvas.drawRect(Rect.fromLTWH(0, 0, w, h), paintLine);
    
    // grid
    for(double x=0; x<=w; x+=blueprint.scale) canvas.drawLine(Offset(x, 0), Offset(x, h), paintLine);
    for(double y=0; y<=h; y+=blueprint.scale) canvas.drawLine(Offset(0, y), Offset(w, y), paintLine);
    
    // listener
    canvas.drawCircle(Offset(w/2, h/2), 10, Paint()..color=neonCyan.withValues(alpha:0.5));
  }
  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => true;
}

class _IsoViewPainter extends CustomPainter {
  final Color neonCyan;
  final BlueprintData blueprint;
  final double orbitAngleX;
  final double orbitAngleY;

  _IsoViewPainter({required this.neonCyan, required this.blueprint, required this.orbitAngleX, required this.orbitAngleY});

  @override
  void paint(Canvas canvas, Size size) {
    final proj = _IsoProjector(
      scale: blueprint.scale, cx: size.width/2, cy: size.height/2,
      roomW: blueprint.canvasWidthMeters, roomD: blueprint.canvasHeightMeters,
      angleX: orbitAngleX, angleY: orbitAngleY,
    );

    final line = Paint()..color = Colors.white30..strokeWidth = 1.0;
    final w = blueprint.canvasWidthMeters;
    final d = blueprint.canvasHeightMeters;
    final h = blueprint.roomHeightMeters;

    for (double x = 0; x <= w; x += 1.0) canvas.drawLine(proj.project(x, 0, 0), proj.project(x, d, 0), line);
    for (double y = 0; y <= d; y += 1.0) canvas.drawLine(proj.project(0, y, 0), proj.project(w, y, 0), line);

    final wall = Paint()..color = neonCyan.withValues(alpha:0.4)..strokeWidth = 2.0;
    final corners = [
      [proj.project(0,0,0), proj.project(0,0,h)], [proj.project(w,0,0), proj.project(w,0,h)],
      [proj.project(0,d,0), proj.project(0,d,h)], [proj.project(w,d,0), proj.project(w,d,h)]
    ];
    for (var c in corners) canvas.drawLine(c[0], c[1], wall);
    
    final tops = [proj.project(0,0,h), proj.project(w,0,h), proj.project(w,d,h), proj.project(0,d,h)];
    for(int i=0; i<4; i++) canvas.drawLine(tops[i], tops[(i+1)%4], wall);

    canvas.drawCircle(proj.project(w/2, d/2, blueprint.listeningHeightMeters), 8, Paint()..color=neonCyan);
  }
  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => true;
}

class _HeatmapPainter extends CustomPainter {
  final List<SpeakerNode> speakers;
  final BlueprintData blueprint;
  final bool isTopView;
  final double orbitAngleX;
  final double orbitAngleY;
  final double targetSPL;

  _HeatmapPainter({required this.speakers, required this.blueprint, required this.isTopView, this.orbitAngleX=0, this.orbitAngleY=0, required this.targetSPL});

  @override
  void paint(Canvas canvas, Size size) {
    if (speakers.isEmpty) return;

    for (var spk in speakers) {
      // 3. 히트맵 렌더링 반경(Rx, Ry) 공식에 CoverageDistance와 Dispersion 값 대입
      final dist = spk.dispersionDistance * blueprint.scale;
      final rx = dist * math.tan((spk.dispersionAngle / 2) * math.pi / 180.0);
      final ry = dist * math.tan((spk.dispersionAngleV / 2) * math.pi / 180.0);
      
      final H = math.max(0.1, spk.heightZ - blueprint.listeningHeightMeters);
      final tilt = spk.pitchTilt * math.pi / 180.0;
      final projOffset = (H / math.tan(tilt)) * blueprint.scale;

      final Gradient thermalGradient = RadialGradient(
        colors: [
          const Color(0xFFFF0000).withValues(alpha: 0.8), // Red
          const Color(0xFFFFFF00).withValues(alpha: 0.6), // Yellow
          const Color(0xFF00FF00).withValues(alpha: 0.4), // Green
          const Color(0xFF00FFFF).withValues(alpha: 0.2), // Cyan
          const Color(0x00000000),
        ],
        stops: const [0.0, 0.2, 0.45, 0.75, 1.0],
      );

      final Paint heat = Paint()..blendMode = BlendMode.screen;
      heat.shader = thermalGradient.createShader(Rect.fromCircle(center: Offset.zero, radius: math.max(rx, ry)));
      
      canvas.save();
      if (isTopView) {
        canvas.translate(spk.x * blueprint.scale, spk.y * blueprint.scale);
        canvas.rotate(spk.rotation * math.pi / 180.0);
        canvas.translate(0, projOffset);
        canvas.scale(rx / math.max(rx, ry), ry / math.max(rx, ry));
      } else {
        final proj = _IsoProjector(
          scale: blueprint.scale, cx: size.width/2, cy: size.height/2,
          roomW: blueprint.canvasWidthMeters, roomD: blueprint.canvasHeightMeters,
          angleX: orbitAngleX, angleY: orbitAngleY,
        );
        // Map the center of the heatmap (which is on the floor) to screen
        // We know its world coordinates:
        double hx = spk.x - (H / math.tan(tilt)) * math.sin(spk.rotation * math.pi / 180.0);
        double hy = spk.y + (H / math.tan(tilt)) * math.cos(spk.rotation * math.pi / 180.0);
        
        final centerScreen = proj.project(hx, hy, blueprint.listeningHeightMeters);
        canvas.translate(centerScreen.dx, centerScreen.dy);
        
        // Rotate and scale according to orbit
        canvas.rotate(orbitAngleX);
        canvas.scale(1.0, math.cos(orbitAngleY));
        canvas.rotate(spk.rotation * math.pi / 180.0);
        canvas.scale(rx / math.max(rx, ry), ry / math.max(rx, ry));
      }
      
      canvas.drawCircle(Offset.zero, math.max(rx, ry), heat);
      canvas.restore();
    }
  }
  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => true;
}

class _SciFiSpeakerWidget extends StatelessWidget {
  final SpeakerNode node;
  final bool isSelected;
  final Color neonCyan;

  const _SciFiSpeakerWidget({required this.node, required this.isSelected, required this.neonCyan});

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 40,
      height: 40,
      decoration: BoxDecoration(
        color: isSelected ? neonCyan.withValues(alpha: 0.3) : Colors.black87,
        border: Border.all(color: isSelected ? neonCyan : Colors.white54, width: 2),
        shape: BoxShape.circle,
      ),
      child: Center(
        child: Text(
          'CH${node.channel+1}',
          style: TextStyle(color: isSelected ? Colors.black : Colors.white, fontSize: 10, fontWeight: FontWeight.bold),
        ),
      ),
    );
  }
}
