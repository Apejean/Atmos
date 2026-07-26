import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:uuid/uuid.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/speaker_node_widget.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/trajectory.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/trajectory_state.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';

const double _gridSize = 50.0;
const double _canvasWidth = 2000.0;
const double _canvasHeight = 2000.0;
const double _speakerSize = 60.0;

enum CanvasMode {
  speaker,
  room,
  trajectory,
}

class SpeakerCanvasScreen extends ConsumerStatefulWidget {
  const SpeakerCanvasScreen({super.key});

  @override
  ConsumerState<SpeakerCanvasScreen> createState() => _SpeakerCanvasScreenState();
}

class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {
  final TransformationController _transformationController = TransformationController();
  Size _viewportSize = const Size(800, 600);
  CanvasMode _currentMode = CanvasMode.speaker;
  String? _drawingTrajectoryId;

  @override
  void initState() {
    super.initState();
    // Center the view initially
    _transformationController.value = Matrix4.identity()
      ..translate(-_canvasWidth / 2 + 400, -_canvasHeight / 2 + 300, 0.0);
  }

  @override
  void dispose() {
    _transformationController.dispose();
    super.dispose();
  }

  Offset _getCanvasCenter() {
    final centerMatrix = _transformationController.value.clone()..invert();
    final viewportCenter = Offset(_viewportSize.width / 2, _viewportSize.height / 2);
    return MatrixUtils.transformPoint(centerMatrix, viewportCenter);
  }

  void _addSpeaker() {
    final canvasCenter = _getCanvasCenter();
    
    double cx = (canvasCenter.dx / _gridSize).round() * _gridSize;
    double cy = (canvasCenter.dy / _gridSize).round() * _gridSize;
    
    cx = cx.clamp(0.0, _canvasWidth - _speakerSize);
    cy = cy.clamp(0.0, _canvasHeight - _speakerSize);

    final newNode = SpeakerNode(
      id: const Uuid().v4(),
      x: cx,
      y: cy,
      channel: 0,
    );
    ref.read(speakerLayoutProvider.notifier).addSpeaker(newNode);
  }

  void _addRoom() {
    final canvasCenter = _getCanvasCenter();
    double cx = (canvasCenter.dx / _gridSize).round() * _gridSize;
    double cy = (canvasCenter.dy / _gridSize).round() * _gridSize;

    final newRoom = RoomZone(
      id: const Uuid().v4(),
      x: cx,
      y: cy,
      width: 300.0,
      height: 200.0,
    );
    ref.read(roomZoneProvider.notifier).addRoomZone(newRoom);
  }

  void _addTrajectoryPoint(Offset localPosition) {
    if (_drawingTrajectoryId == null) {
      _drawingTrajectoryId = const Uuid().v4();
      final t = Trajectory(id: _drawingTrajectoryId!, waypoints: [
        TrajectoryWaypoint(localPosition.dx, localPosition.dy)
      ]);
      ref.read(trajectoryProvider.notifier).addTrajectory(t);
    } else {
      final trajectories = ref.read(trajectoryProvider);
      final t = trajectories.firstWhere((element) => element.id == _drawingTrajectoryId);
      final updated = t.copyWith(waypoints: [...t.waypoints, TrajectoryWaypoint(localPosition.dx, localPosition.dy)]);
      ref.read(trajectoryProvider.notifier).updateTrajectory(updated);
    }
  }

  void _endTrajectory() {
    _drawingTrajectoryId = null;
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final nodes = ref.watch(speakerLayoutProvider);
    final rooms = ref.watch(roomZoneProvider);
    final trajectories = ref.watch(trajectoryProvider);

    return Scaffold(
      backgroundColor: AppColors.background,
      appBar: AppBar(
        title: Row(
          children: [
            const Text('Exhibition Canvas'),
            const SizedBox(width: 16),
            ToggleButtons(
              isSelected: [
                _currentMode == CanvasMode.speaker,
                _currentMode == CanvasMode.room,
                _currentMode == CanvasMode.trajectory,
              ],
              onPressed: (index) {
                setState(() {
                  _currentMode = CanvasMode.values[index];
                  _drawingTrajectoryId = null;
                });
              },
              color: Colors.white70,
              selectedColor: AppColors.primaryNeon,
              fillColor: AppColors.primaryNeon.withOpacity(0.1),
              borderColor: Colors.white24,
              selectedBorderColor: AppColors.primaryNeon,
              children: const [
                Padding(padding: EdgeInsets.symmetric(horizontal: 12), child: Text('Speakers')),
                Padding(padding: EdgeInsets.symmetric(horizontal: 12), child: Text('Rooms')),
                Padding(padding: EdgeInsets.symmetric(horizontal: 12), child: Text('Trajectories')),
              ],
            ),
            const SizedBox(width: 16),
            Consumer(
              builder: (context, ref, child) {
                final isMasterMuted = ref.watch(engineStateProvider.select((state) => state.masterMuteActive));
                if (!isMasterMuted) return const SizedBox.shrink();
                return Container(
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
                  decoration: BoxDecoration(
                    color: Colors.orange.shade800,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: const Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(Icons.warning_amber_rounded, color: Colors.white, size: 16),
                      SizedBox(width: 8),
                      Text(
                        'MASTER MUTE ACTIVE',
                        style: TextStyle(
                          color: Colors.white,
                          fontWeight: FontWeight.bold,
                          fontSize: 12,
                          letterSpacing: 1.0,
                        ),
                      ),
                    ],
                  ),
                );
              },
            ),
          ],
        ),
        backgroundColor: Colors.black,
      ),
      body: LayoutBuilder(
        builder: (context, constraints) {
          _viewportSize = Size(constraints.maxWidth, constraints.maxHeight);
          return InteractiveViewer(
            transformationController: _transformationController,
            boundaryMargin: const EdgeInsets.all(double.infinity),
            minScale: 0.1,
            maxScale: 2.0,
            constrained: false,
            // Disable default pan/zoom when drawing trajectories
            panEnabled: _currentMode != CanvasMode.trajectory,
            scaleEnabled: _currentMode != CanvasMode.trajectory,
            child: GestureDetector(
              onTapUp: (details) {
                if (_currentMode == CanvasMode.trajectory) {
                  _addTrajectoryPoint(details.localPosition);
                }
              },
              onDoubleTap: () {
                if (_currentMode == CanvasMode.trajectory) {
                  _endTrajectory();
                }
              },
              child: SizedBox(
                width: _canvasWidth,
                height: _canvasHeight,
                child: Stack(
                  clipBehavior: Clip.none,
                  children: [
                    // Grid Background
                    Positioned.fill(
                      child: CustomPaint(
                        painter: _GridPainter(),
                      ),
                    ),
                    
                    // Rooms
                    ...rooms.map((room) {
                      return Positioned(
                        left: room.x,
                        top: room.y,
                        width: room.width,
                        height: room.height,
                        child: GestureDetector(
                          onPanUpdate: _currentMode == CanvasMode.room ? (details) {
                            final scale = _transformationController.value.getMaxScaleOnAxis();
                            final currentScale = scale > 0 ? scale : 1.0;
                            double newX = room.x + (details.delta.dx / currentScale);
                            double newY = room.y + (details.delta.dy / currentScale);
                            
                            ref.read(roomZoneProvider.notifier).updateRoomZone(room.copyWith(
                              x: newX,
                              y: newY,
                            ));
                          } : null,
                          onPanEnd: _currentMode == CanvasMode.room ? (details) {
                            final snappedX = (room.x / _gridSize).round() * _gridSize;
                            final snappedY = (room.y / _gridSize).round() * _gridSize;
                            ref.read(roomZoneProvider.notifier).updateRoomZone(
                              room.copyWith(x: snappedX, y: snappedY), immediate: true
                            );
                          } : null,
                          onDoubleTap: _currentMode == CanvasMode.room ? () {
                            ref.read(roomZoneProvider.notifier).removeRoomZone(room.id);
                          } : null,
                          child: Container(
                            decoration: BoxDecoration(
                              color: AppColors.primaryNeon.withOpacity(0.1),
                              border: Border.all(color: AppColors.primaryNeon.withOpacity(0.5), width: 2),
                              borderRadius: BorderRadius.circular(8),
                            ),
                            child: Center(
                              child: Text(
                                room.label,
                                style: const TextStyle(color: Colors.white, fontSize: 16, fontWeight: FontWeight.bold),
                              ),
                            ),
                          ),
                        ),
                      );
                    }),

                    // Trajectories
                    Positioned.fill(
                      child: CustomPaint(
                        painter: _TrajectoryPainter(trajectories),
                      ),
                    ),

                    // Speaker Nodes
                    ...nodes.map((node) {
                      final isDuplicate = nodes.where((n) => n.id != node.id && n.channel == node.channel).isNotEmpty;

                      return Positioned(
                        left: node.x,
                        top: node.y,
                        child: GestureDetector(
                          onPanUpdate: _currentMode == CanvasMode.speaker ? (details) {
                            final scale = _transformationController.value.getMaxScaleOnAxis();
                            final currentScale = scale > 0 ? scale : 1.0;

                            double newX = node.x + (details.delta.dx / currentScale);
                            double newY = node.y + (details.delta.dy / currentScale);
                            
                            final updated = node.copyWith(
                              x: newX.clamp(0.0, _canvasWidth - _speakerSize),
                              y: newY.clamp(0.0, _canvasHeight - _speakerSize),
                            );
                            ref.read(speakerLayoutProvider.notifier).updateSpeaker(updated);
                          } : null,
                          onPanEnd: _currentMode == CanvasMode.speaker ? (details) {
                            final snappedX = (node.x / _gridSize).round() * _gridSize;
                            final snappedY = (node.y / _gridSize).round() * _gridSize;
                            final updated = node.copyWith(
                              x: snappedX.clamp(0.0, _canvasWidth - _speakerSize),
                              y: snappedY.clamp(0.0, _canvasHeight - _speakerSize)
                            );
                            ref.read(speakerLayoutProvider.notifier).updateSpeaker(updated, immediate: true);
                          } : null,
                          child: AbsorbPointer(
                            absorbing: _currentMode != CanvasMode.speaker,
                            child: SpeakerNodeWidget(
                              node: node,
                              onChannelChanged: (ch) {
                                ref.read(speakerLayoutProvider.notifier).updateSpeaker(node.copyWith(channel: ch));
                              },
                              onDelete: () {
                                ref.read(speakerLayoutProvider.notifier).removeSpeaker(node.id);
                              },
                              isDuplicateChannel: isDuplicate,
                            ),
                          ),
                        ),
                      );
                    }),
                  ],
                ),
              ),
            ),
          );
        },
      ),
      floatingActionButton: _currentMode == CanvasMode.trajectory 
          ? null 
          : FloatingActionButton(
              onPressed: _currentMode == CanvasMode.speaker ? _addSpeaker : _addRoom,
              backgroundColor: AppColors.primaryNeon,
              child: const Icon(Icons.add, color: Colors.black),
            ),
    );
  }
}

class _GridPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.white12
      ..strokeWidth = 1.0;

    for (double i = 0; i <= size.width; i += _gridSize) {
      canvas.drawLine(Offset(i, 0), Offset(i, size.height), paint);
    }
    for (double i = 0; i <= size.height; i += _gridSize) {
      canvas.drawLine(Offset(0, i), Offset(size.width, i), paint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class _TrajectoryPainter extends CustomPainter {
  final List<Trajectory> trajectories;

  _TrajectoryPainter(this.trajectories);

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.cyanAccent
      ..strokeWidth = 3.0
      ..style = PaintingStyle.stroke;

    final dotPaint = Paint()
      ..color = Colors.cyan
      ..style = PaintingStyle.fill;

    for (var t in trajectories) {
      if (t.waypoints.isEmpty) continue;
      
      final path = Path();
      path.moveTo(t.waypoints.first.x, t.waypoints.first.y);
      canvas.drawCircle(Offset(t.waypoints.first.x, t.waypoints.first.y), 6, dotPaint);

      for (int i = 1; i < t.waypoints.length; i++) {
        path.lineTo(t.waypoints[i].x, t.waypoints[i].y);
        canvas.drawCircle(Offset(t.waypoints[i].x, t.waypoints[i].y), 6, dotPaint);
      }
      canvas.drawPath(path, paint);
    }
  }

  @override
  bool shouldRepaint(covariant _TrajectoryPainter oldDelegate) {
    return true; // Simple approach, always repaint on change
  }
}
