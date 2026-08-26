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

  // Local state for instant UI updates before syncing to backend
  double _x = 0.5; // Normalized 0.0 to 1.0
  double _y = 0.5; // Normalized 0.0 to 1.0
  double _z = 0.5; // Elevation 0.0 to 1.0
  double _size = 0.2; // Spread 0.0 to 1.0
  
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

  Widget _buildPresetButton(String label, IconData icon, String value) {
    final isSelected = _preset == value;
    return InkWell(
      onTap: () {
        setState(() => _preset = value);
      },
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        decoration: BoxDecoration(
          color: isSelected ? AppColors.primaryNeon.withValues(alpha: 0.2) : Colors.white10,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isSelected ? AppColors.primaryNeon : Colors.transparent,
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 16, color: isSelected ? AppColors.primaryNeon : Colors.white54),
            const SizedBox(width: 6),
            Text(label, style: TextStyle(
              fontSize: 12,
              color: isSelected ? AppColors.primaryNeon : Colors.white54,
              fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
            )),
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      backgroundColor: Colors.transparent,
      child: Container(
        width: 500,
        decoration: BoxDecoration(
          color: AppColors.background,
          borderRadius: BorderRadius.circular(16),
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
              padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 16),
              decoration: const BoxDecoration(
                border: Border(bottom: BorderSide(color: Colors.white12)),
              ),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  const Text('3D Object Panner', style: TextStyle(color: Colors.white, fontSize: 18, fontWeight: FontWeight.bold)),
                  InkWell(
                    onTap: () => Navigator.of(context).pop(),
                    child: const Icon(Icons.close, color: Colors.white54, size: 20),
                  ),
                ],
              ),
            ),
            
            // Body
            Padding(
              padding: const EdgeInsets.all(24.0),
              child: Column(
                children: [
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      // Minimap
                      Expanded(
                        child: AspectRatio(
                          aspectRatio: 1.0,
                          child: Container(
                            decoration: BoxDecoration(
                              color: const Color(0xFF111111),
                              borderRadius: BorderRadius.circular(12),
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
                                    painter: _PannerGridPainter(),
                                    child: Stack(
                                      children: [
                                        // Center crosshair
                                        const Center(
                                          child: Icon(Icons.add, color: Colors.white12, size: 32),
                                        ),
                                        // The Puck
                                        Positioned(
                                          left: _x * constraints.maxWidth - 20,
                                          top: _y * constraints.maxHeight - 20,
                                          child: AnimatedBuilder(
                                            animation: _pulseController,
                                            builder: (context, child) {
                                              return Container(
                                                width: 40,
                                                height: 40,
                                                decoration: BoxDecoration(
                                                  shape: BoxShape.circle,
                                                  color: AppColors.primaryNeon.withValues(alpha: 0.2 + _pulseController.value * 0.2),
                                                  boxShadow: [
                                                    BoxShadow(
                                                      color: AppColors.primaryNeon.withValues(alpha: 0.5 * _size),
                                                      blurRadius: 10 + 40 * _size,
                                                      spreadRadius: 5 + 20 * _size,
                                                    ),
                                                  ],
                                                ),
                                                child: Center(
                                                  child: Container(
                                                    width: 12,
                                                    height: 12,
                                                    decoration: BoxDecoration(
                                                      color: AppColors.primaryNeon,
                                                      shape: BoxShape.circle,
                                                      border: Border.all(color: Colors.white, width: 1.5),
                                                    ),
                                                  ),
                                                ),
                                              );
                                            },
                                          ),
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
                      const SizedBox(width: 24),
                      
                      // Elevation Z-axis slider
                      Column(
                        children: [
                          const Text('Z', style: TextStyle(color: Colors.white54, fontWeight: FontWeight.bold)),
                          const SizedBox(height: 8),
                          SizedBox(
                            height: 250, // Matches minimap roughly
                            child: RotatedBox(
                              quarterTurns: 3,
                              child: Slider(
                                value: _z,
                                min: 0.0,
                                max: 1.0,
                                activeColor: Colors.cyanAccent,
                                inactiveColor: Colors.white12,
                                onChanged: (v) {
                                  setState(() => _z = v);
                                },
                                onChangeEnd: (_) => _updateBackend(),
                              ),
                            ),
                          ),
                          const SizedBox(height: 8),
                          Text('${(_z * 100).toInt()}', style: const TextStyle(color: Colors.cyanAccent, fontSize: 12)),
                        ],
                      ),
                    ],
                  ),
                  
                  const SizedBox(height: 24),
                  
                  // Bottom controls (Size and Presets)
                  Row(
                    children: [
                      // Size control
                      Expanded(
                        flex: 2,
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            const Text('Size (Spread)', style: TextStyle(color: Colors.white70, fontSize: 12)),
                            Slider(
                              value: _size,
                              min: 0.0,
                              max: 1.0,
                              activeColor: Colors.amberAccent,
                              inactiveColor: Colors.white12,
                              onChanged: (v) {
                                setState(() => _size = v);
                              },
                              onChangeEnd: (_) => _updateBackend(),
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(width: 16),
                      // Presets
                      Expanded(
                        flex: 3,
                        child: Wrap(
                          spacing: 8,
                          runSpacing: 8,
                          children: [
                            _buildPresetButton('Static', Icons.pan_tool_alt, 'None'),
                            _buildPresetButton('Circle', Icons.change_history, 'Circle'),
                            _buildPresetButton('Figure-8', Icons.all_inclusive, 'Figure-8'),
                          ],
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _PannerGridPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.white10
      ..strokeWidth = 1.0;
    
    // Draw grid lines
    const int lines = 4;
    final double stepX = size.width / lines;
    final double stepY = size.height / lines;
    
    for (int i = 1; i < lines; i++) {
      canvas.drawLine(Offset(i * stepX, 0), Offset(i * stepX, size.height), paint);
      canvas.drawLine(Offset(0, i * stepY), Offset(size.width, i * stepY), paint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
