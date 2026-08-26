import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/trajectory_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/trajectory.dart';

class ObjectPannerModal extends ConsumerStatefulWidget {
  final String? trackId;

  const ObjectPannerModal({super.key, this.trackId});

  @override
  ConsumerState<ObjectPannerModal> createState() => _ObjectPannerModalState();
}

class _ObjectPannerModalState extends ConsumerState<ObjectPannerModal> with SingleTickerProviderStateMixin {
  TrajectoryModel? _activeTrajectory;

  // Values normalized 0.0 to 1.0
  double _x = 0.5; 
  double _y = 0.5; 
  double _z = 0.5; 
  double _size = 0.2; 
  
  String _preset = 'None';
  late AnimationController _pulseController;

  @override
  void initState() {
    super.initState();
    _pulseController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1500),
    )..repeat(reverse: true);
    
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _loadState();
    });
  }

  @override
  void dispose() {
    _pulseController.dispose();
    super.dispose();
  }

  void _loadState() {
    if (widget.trackId == null) return;
    final trajectories = ref.read(trajectoryProvider);
    try {
      _activeTrajectory = trajectories.firstWhere((t) => t.audioTrackId == widget.trackId);
      setState(() {
        _size = _activeTrajectory!.size;
        if (_activeTrajectory!.waypoints.isNotEmpty) {
          final pos = _activeTrajectory!.waypoints.first.position;
          _x = pos.dx.clamp(0.0, 1.0);
          _y = pos.dy.clamp(0.0, 1.0);
          _z = _activeTrajectory!.waypoints.first.heightZ.clamp(0.0, 1.0);
        }
      });
    } catch (e) {
      // not found
    }
  }

  void _updateBackend() {
    if (_activeTrajectory != null) {
      final updatedWaypoints = List<Waypoint>.from(_activeTrajectory!.waypoints);
      if (updatedWaypoints.isEmpty) {
        updatedWaypoints.add(Waypoint(position: Offset(_x, _y), heightZ: _z));
      } else {
        updatedWaypoints[0] = Waypoint(position: Offset(_x, _y), heightZ: _z);
      }

      final updated = _activeTrajectory!.copyWith(
        size: _size,
        waypoints: updatedWaypoints,
      );
      ref.read(trajectoryProvider.notifier).updateTrajectory(updated, immediate: true);
    }
  }

  String _formatVal(double val) {
    // 0.0 to 1.0 -> -1.0 to +1.0 for display
    final mapped = (val - 0.5) * 2.0;
    return mapped > 0 ? '+${mapped.toStringAsFixed(3)}' : mapped.toStringAsFixed(3);
  }

  String _formatZ(double val) {
    return '+${val.toStringAsFixed(3)}';
  }

  Widget _buildReadout(String label, String value) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        Text(label, style: const TextStyle(color: Colors.white54, fontSize: 10)),
        const SizedBox(height: 2),
        Text(value, style: const TextStyle(color: Colors.white, fontSize: 13, fontFamily: 'Monospace')),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      backgroundColor: Colors.transparent,
      child: Container(
        width: 450,
        decoration: BoxDecoration(
          color: const Color(0xFF1E1E1E), // Dark charcoal
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: Colors.white12),
          boxShadow: [
            BoxShadow(color: Colors.black.withValues(alpha: 0.5), blurRadius: 20, spreadRadius: 5),
          ],
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            // Header
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
              decoration: const BoxDecoration(
                border: Border(bottom: BorderSide(color: Colors.white10)),
              ),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  const Text('3D Object Panner', style: TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.bold)),
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      DropdownButton<String>(
                        value: _preset,
                        isDense: true,
                        dropdownColor: const Color(0xFF2C2C2C),
                        underline: const SizedBox(),
                        icon: const Icon(Icons.arrow_drop_down, color: Colors.white54, size: 16),
                        style: const TextStyle(color: Colors.white70, fontSize: 12),
                        items: ['None', 'Circle', 'Figure-8'].map((p) => DropdownMenuItem(value: p, child: Text(p))).toList(),
                        onChanged: (v) {
                          if (v != null) setState(() => _preset = v);
                        },
                      ),
                      const SizedBox(width: 16),
                      InkWell(
                        onTap: () => Navigator.of(context).pop(),
                        child: const Icon(Icons.close, color: Colors.white54, size: 18),
                      ),
                    ],
                  ),
                ],
              ),
            ),
            
            // Numeric Readouts
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 12.0),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                children: [
                  _buildReadout('Left/Right', _formatVal(_x)),
                  _buildReadout('Back/Front', _formatVal(1.0 - _y)), // Inverse Y for Back/Front (0 is front, 1 is back)
                  _buildReadout('Elevation', _formatZ(_z)),
                  _buildReadout('Size', _size.toStringAsFixed(3)),
                ],
              ),
            ),

            // Top-down Minimap
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 32.0),
              child: Column(
                children: [
                  const Text('Front', style: TextStyle(color: Colors.white54, fontSize: 10)),
                  const SizedBox(height: 4),
                  Row(
                    children: [
                      const Text('Left', style: TextStyle(color: Colors.white54, fontSize: 10)),
                      const SizedBox(width: 8),
                      Expanded(
                        child: AspectRatio(
                          aspectRatio: 1.0,
                          child: Container(
                            decoration: BoxDecoration(
                              color: const Color(0xFF111111),
                              borderRadius: BorderRadius.circular(4),
                              border: Border.all(color: Colors.white24),
                            ),
                            child: LayoutBuilder(
                              builder: (context, constraints) {
                                return GestureDetector(
                                  onPanUpdate: (details) {
                                    setState(() {
                                      _x = (_x + details.delta.dx / constraints.maxWidth).clamp(0.0, 1.0);
                                      _y = (_y + details.delta.dy / constraints.maxHeight).clamp(0.0, 1.0);
                                    });
                                    _updateBackend();
                                  },
                                  onPanDown: (details) {
                                    setState(() {
                                      _x = (details.localPosition.dx / constraints.maxWidth).clamp(0.0, 1.0);
                                      _y = (details.localPosition.dy / constraints.maxHeight).clamp(0.0, 1.0);
                                    });
                                    _updateBackend();
                                  },
                                  child: CustomPaint(
                                    painter: _GridPainter(divisions: 4),
                                    child: Stack(
                                      children: [
                                        // Center Head Icon
                                        const Center(
                                          child: Icon(Icons.person, color: Colors.white24, size: 24),
                                        ),
                                        // Puck
                                        Positioned(
                                          left: _x * constraints.maxWidth - 12,
                                          top: _y * constraints.maxHeight - 12,
                                          child: _Puck(size: _size, pulse: _pulseController.value),
                                        ),
                                      ],
                                    ),
                                  ),
                                );
                              },
                            ),
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      const Text('Right', style: TextStyle(color: Colors.white54, fontSize: 10)),
                    ],
                  ),
                  const SizedBox(height: 4),
                  const Text('Back', style: TextStyle(color: Colors.white54, fontSize: 10)),
                ],
              ),
            ),
            
            const SizedBox(height: 24),

            // Side-view Elevation Minimap
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 56.0),
              child: Row(
                children: [
                  Expanded(
                    child: SizedBox(
                      height: 100, // Wide rectangular aspect
                      child: Container(
                        decoration: BoxDecoration(
                          color: const Color(0xFF111111),
                          borderRadius: BorderRadius.circular(4),
                          border: Border.all(color: Colors.white24),
                        ),
                        child: LayoutBuilder(
                          builder: (context, constraints) {
                            return GestureDetector(
                              onPanUpdate: (details) {
                                setState(() {
                                  // Z is inverted in UI (0 at bottom, 1 at top)
                                  _z = (_z - details.delta.dy / constraints.maxHeight).clamp(0.0, 1.0);
                                });
                                _updateBackend();
                              },
                              onPanDown: (details) {
                                setState(() {
                                  _z = (1.0 - details.localPosition.dy / constraints.maxHeight).clamp(0.0, 1.0);
                                });
                                _updateBackend();
                              },
                              child: CustomPaint(
                                painter: _GridPainter(divisions: 2, isHorizontal: true),
                                child: Stack(
                                  children: [
                                    // Bottom center Head Icon
                                    const Align(
                                      alignment: Alignment.bottomCenter,
                                      child: Icon(Icons.person, color: Colors.white24, size: 24),
                                    ),
                                    // Puck (Fixed X in center for this view, only Z moves)
                                    Positioned(
                                      left: constraints.maxWidth / 2 - 12,
                                      top: (1.0 - _z) * constraints.maxHeight - 12,
                                      child: _Puck(size: _size, pulse: _pulseController.value),
                                    ),
                                  ],
                                ),
                              ),
                            );
                          },
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  const Column(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('Top', style: TextStyle(color: Colors.white54, fontSize: 10)),
                      SizedBox(height: 60),
                      Text('Ear Level', style: TextStyle(color: Colors.white54, fontSize: 10)),
                    ],
                  ),
                ],
              ),
            ),
            
            const SizedBox(height: 24),
            
            // Minimal Size Slider at bottom
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 56.0, vertical: 12.0),
              child: Row(
                children: [
                  const Text('Size', style: TextStyle(color: Colors.white54, fontSize: 12)),
                  const SizedBox(width: 12),
                  Expanded(
                    child: SliderTheme(
                      data: SliderThemeData(
                        trackHeight: 2,
                        thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
                        overlayShape: const RoundSliderOverlayShape(overlayRadius: 14),
                      ),
                      child: Slider(
                        value: _size,
                        min: 0.0,
                        max: 1.0,
                        activeColor: AppColors.primaryNeon,
                        inactiveColor: Colors.white24,
                        onChanged: (v) {
                          setState(() => _size = v);
                          _updateBackend();
                        },
                      ),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 12),
          ],
        ),
      ),
    );
  }
}

class _GridPainter extends CustomPainter {
  final int divisions;
  final bool isHorizontal;

  _GridPainter({required this.divisions, this.isHorizontal = false});

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.white.withValues(alpha: 0.1)
      ..strokeWidth = 1.0;
    
    if (!isHorizontal) {
      final double stepX = size.width / divisions;
      for (int i = 1; i < divisions; i++) {
        canvas.drawLine(Offset(i * stepX, 0), Offset(i * stepX, size.height), paint);
      }
    }
    
    final int rows = isHorizontal ? 2 : divisions;
    final double stepY = size.height / rows;
    for (int i = 1; i < rows; i++) {
      canvas.drawLine(Offset(0, i * stepY), Offset(size.width, i * stepY), paint);
    }
    
    // Draw crosshairs if square map
    if (!isHorizontal) {
      final centerPaint = Paint()
        ..color = Colors.white.withValues(alpha: 0.2)
        ..strokeWidth = 1.5;
      canvas.drawLine(Offset(size.width / 2, 0), Offset(size.width / 2, size.height), centerPaint);
      canvas.drawLine(Offset(0, size.height / 2), Offset(size.width, size.height / 2), centerPaint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class _Puck extends StatelessWidget {
  final double size;
  final double pulse;

  const _Puck({required this.size, required this.pulse});

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 24,
      height: 24,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: AppColors.primaryNeon.withValues(alpha: 0.2 + pulse * 0.2),
        boxShadow: [
          BoxShadow(
            color: AppColors.primaryNeon.withValues(alpha: 0.5 * size),
            blurRadius: 5 + 20 * size,
            spreadRadius: 2 + 10 * size,
          ),
        ],
      ),
      child: Center(
        child: Container(
          width: 8,
          height: 8,
          decoration: BoxDecoration(
            color: AppColors.primaryNeon,
            shape: BoxShape.circle,
            border: Border.all(color: Colors.white, width: 1.0),
          ),
        ),
      ),
    );
  }
}
