import 'dart:ui';
import 'dart:math' as math;
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

class SpeakerCanvasScreen extends ConsumerStatefulWidget {
  const SpeakerCanvasScreen({super.key});

  @override
  ConsumerState<SpeakerCanvasScreen> createState() =>
      _SpeakerCanvasScreenState();
}

class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {
  final TransformationController _transformationController =
      TransformationController();
  Size _viewportSize = const Size(800, 600);

  static int _roomColorIndex = 0;

  @override
  void initState() {
    super.initState();
    _transformationController.value = Matrix4.identity()
      ..setTranslationRaw(
        -_canvasWidth / 2 + 400,
        -_canvasHeight / 2 + 300,
        0.0,
      );
  }

  @override
  void dispose() {
    _transformationController.dispose();
    super.dispose();
  }

  Offset _getCanvasCenter() {
    final centerMatrix = _transformationController.value.clone()..invert();
    final viewportCenter = Offset(
      _viewportSize.width / 2,
      _viewportSize.height / 2,
    );
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
    cx = cx.clamp(0.0, _canvasWidth - 300.0);
    cy = cy.clamp(0.0, _canvasHeight - 200.0);

    final colorValue = AppColors
        .roomAccents[_roomColorIndex % AppColors.roomAccents.length]
        .toARGB32();
    _roomColorIndex++;

    final newRoom = RoomZone(
      id: const Uuid().v4(),
      x: cx,
      y: cy,
      width: 300.0,
      height: 200.0,
      color: colorValue,
    );
    ref.read(roomZoneProvider.notifier).addRoomZone(newRoom);
  }

  void _addTrajectory() {
    final canvasCenter = _getCanvasCenter();
    double cx = canvasCenter.dx.clamp(100.0, _canvasWidth - 100.0);
    double cy = canvasCenter.dy.clamp(100.0, _canvasHeight - 100.0);

    final t = Trajectory(
      id: const Uuid().v4(),
      waypoints: [
        TrajectoryWaypoint(cx - 100, cy),
        TrajectoryWaypoint(cx + 100, cy),
      ],
    );
    ref.read(trajectoryProvider.notifier).addTrajectory(t);
  }

  void _clearCanvas() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        backgroundColor: AppColors.cardSurface,
        title: const Text('Clear Canvas', style: TextStyle(color: Colors.white)),
        content: const Text('Are you sure you want to delete all speakers, rooms, and trajectories?', style: TextStyle(color: Colors.white70)),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel', style: TextStyle(color: Colors.white70)),
          ),
          TextButton(
            onPressed: () {
              ref.read(speakerLayoutProvider.notifier).clearAll();
              ref.read(roomZoneProvider.notifier).clearAll();
              ref.read(trajectoryProvider.notifier).clearAll();
              Navigator.pop(context);
            },
            child: const Text('Clear All', style: TextStyle(color: Colors.redAccent)),
          ),
        ],
      ),
    );
  }

  Future<void> _editRoom(RoomZone room) async {
    final controller = TextEditingController(text: room.label);
    int selectedColor = room.color;

    await showDialog(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setState) => AlertDialog(
          backgroundColor: AppColors.cardSurface,
          title: const Text('Edit Room', style: TextStyle(color: Colors.white)),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: controller,
                style: const TextStyle(color: Colors.white),
                decoration: const InputDecoration(
                  labelText: 'Room Name',
                  labelStyle: TextStyle(color: Colors.white70),
                  enabledBorder: UnderlineInputBorder(
                    borderSide: BorderSide(color: Colors.white24),
                  ),
                  focusedBorder: UnderlineInputBorder(
                    borderSide: BorderSide(color: AppColors.primaryNeon),
                  ),
                ),
              ),
              const SizedBox(height: 24),
              const Text('Theme Color', style: TextStyle(color: Colors.white70)),
              const SizedBox(height: 12),
              Wrap(
                spacing: 12,
                runSpacing: 12,
                children: AppColors.roomAccents.map((color) {
                  final isSelected = color.toARGB32() == selectedColor;
                  return GestureDetector(
                    onTap: () => setState(() => selectedColor = color.toARGB32()),
                    child: Container(
                      width: 40,
                      height: 40,
                      decoration: BoxDecoration(
                        color: color,
                        shape: BoxShape.circle,
                        border: Border.all(
                          color: isSelected ? Colors.white : Colors.transparent,
                          width: 3,
                        ),
                      ),
                    ),
                  );
                }).toList(),
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () {
                ref.read(roomZoneProvider.notifier).removeRoomZone(room.id);
                Navigator.pop(context);
              },
              child: const Text('Delete Room', style: TextStyle(color: Colors.redAccent)),
            ),
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancel', style: TextStyle(color: Colors.white70)),
            ),
            TextButton(
              onPressed: () {
                ref.read(roomZoneProvider.notifier).updateRoomZone(
                      room.copyWith(
                        label: controller.text,
                        color: selectedColor,
                      ),
                      immediate: true,
                    );
                Navigator.pop(context);
              },
              child: const Text('Save', style: TextStyle(color: AppColors.primaryNeon)),
            ),
          ],
        ),
      ),
    );
  }

  List<SpeakerNode> _getSpeakersInRoom(RoomZone room, List<SpeakerNode> nodes) {
    return nodes
        .where((n) => room.containsPoint(n.x + _speakerSize / 2, n.y + _speakerSize / 2))
        .toList();
  }

  Color? _getRoomColorForSpeaker(SpeakerNode speaker, List<RoomZone> rooms) {
    for (final room in rooms) {
      if (room.containsPoint(speaker.x + _speakerSize / 2, speaker.y + _speakerSize / 2)) {
        return Color(room.color);
      }
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: AppColors.background,
      appBar: AppBar(
        title: Row(
          children: [
            const Text('Exhibition Canvas'),
            const Spacer(),
            Consumer(
              builder: (context, ref, child) {
                final isMasterMuted = ref.watch(
                  engineStateProvider.select((state) => state.masterMuteActive),
                );
                if (!isMasterMuted) return const SizedBox.shrink();
                return RepaintBoundary(
                  child: ClipRRect(
                    borderRadius: BorderRadius.circular(4),
                    child: BackdropFilter(
                      filter: ImageFilter.blur(sigmaX: 10, sigmaY: 10),
                      child: Container(
                        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
                        decoration: BoxDecoration(
                          color: Colors.orange.shade800.withValues(alpha: 0.8),
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
                      ),
                    ),
                  ),
                );
              },
            ),
            const SizedBox(width: 16),
            IconButton(
              icon: const Icon(Icons.delete_sweep, color: Colors.redAccent),
              onPressed: _clearCanvas,
              tooltip: 'Clear Canvas',
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
            child: SizedBox(
              width: _canvasWidth,
              height: _canvasHeight,
              child: Stack(
                clipBehavior: Clip.none,
                children: [
                  Positioned.fill(
                    child: RepaintBoundary(
                      child: CustomPaint(painter: _GridPainter()),
                    ),
                  ),
                  Consumer(
                    builder: (context, ref, _) {
                      final rooms = ref.watch(roomZoneProvider);
                      final nodes = ref.watch(speakerLayoutProvider);
                      return Stack(
                        clipBehavior: Clip.none,
                        children: rooms.map((room) {
                          final containedSpeakers = _getSpeakersInRoom(room, nodes);
                          return _DraggableRoomWidget(
                            key: ValueKey(room.id),
                            room: room,
                            containedSpeakers: containedSpeakers,
                            transformationController: _transformationController,
                            onEdit: () => _editRoom(room),
                          );
                        }).toList(),
                      );
                    },
                  ),
                  Consumer(
                    builder: (context, ref, _) {
                      final trajectories = ref.watch(trajectoryProvider);
                      return Stack(
                        children: [
                          Positioned.fill(
                            child: RepaintBoundary(
                              child: CustomPaint(
                                painter: _TrajectoryPainter(trajectories),
                              ),
                            ),
                          ),
                          ...trajectories.expand((t) {
                            return t.waypoints.asMap().entries.map((entry) {
                              final idx = entry.key;
                              final wp = entry.value;
                              return _DraggableWaypointWidget(
                                key: ValueKey('${t.id}_$idx'),
                                trajectory: t,
                                waypointIndex: idx,
                                waypoint: wp,
                                transformationController: _transformationController,
                              );
                            });
                          }),
                        ],
                      );
                    },
                  ),
                  Consumer(
                    builder: (context, ref, _) {
                      final nodes = ref.watch(speakerLayoutProvider);
                      final rooms = ref.watch(roomZoneProvider);
                      return Stack(
                        clipBehavior: Clip.none,
                        children: nodes.map((node) {
                          final isDuplicate = nodes
                              .where((n) => n.id != node.id && n.channel == node.channel)
                              .isNotEmpty;
                          final roomColor = _getRoomColorForSpeaker(node, rooms);
                          return _DraggableSpeakerWidget(
                            key: ValueKey(node.id),
                            node: node,
                            roomColor: roomColor,
                            isDuplicate: isDuplicate,
                            transformationController: _transformationController,
                          );
                        }).toList(),
                      );
                    },
                  ),
                ],
              ),
            ),
          );
        },
      ),
      floatingActionButton: PopupMenuButton<String>(
        onSelected: (value) {
          if (value == 'speaker') { _addSpeaker(); }
          else if (value == 'room') { _addRoom(); }
          else if (value == 'trajectory') { _addTrajectory(); }
        },
        itemBuilder: (BuildContext context) => [
          const PopupMenuItem(value: 'speaker', child: Text('Add Speaker')),
          const PopupMenuItem(value: 'room', child: Text('Add Room')),
          const PopupMenuItem(value: 'trajectory', child: Text('Add Trajectory')),
        ],
        child: FloatingActionButton(
          onPressed: null,
          backgroundColor: AppColors.primaryNeon,
          child: const Icon(Icons.add, color: Colors.black),
        ),
      ),
    );
  }
}

class _DraggableRoomWidget extends ConsumerStatefulWidget {
  final RoomZone room;
  final List<SpeakerNode> containedSpeakers;
  final TransformationController transformationController;
  final VoidCallback onEdit;

  const _DraggableRoomWidget({
    super.key,
    required this.room,
    required this.containedSpeakers,
    required this.transformationController,
    required this.onEdit,
  });

  @override
  ConsumerState<_DraggableRoomWidget> createState() => _DraggableRoomWidgetState();
}

class _DraggableRoomWidgetState extends ConsumerState<_DraggableRoomWidget> {
  late double _localX;
  late double _localY;
  late double _localW;
  late double _localH;
  Map<String, Offset> _draggedSpeakersOffsets = {};

  @override
  void initState() {
    super.initState();
    _localX = widget.room.x;
    _localY = widget.room.y;
    _localW = widget.room.width;
    _localH = widget.room.height;
  }

  @override
  void didUpdateWidget(covariant _DraggableRoomWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    _localX = widget.room.x;
    _localY = widget.room.y;
    _localW = widget.room.width;
    _localH = widget.room.height;
  }

  Widget _buildResizeHandle(Alignment alignment) {
    return Align(
      alignment: alignment,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onPanUpdate: (details) {
          final scale = widget.transformationController.value.getMaxScaleOnAxis();
          final currentScale = scale > 0 ? scale : 1.0;
          final dx = details.delta.dx / currentScale;
          final dy = details.delta.dy / currentScale;

          setState(() {
            if (alignment.x < 0) {
              _localX += dx;
              _localW -= dx;
              if (_localW < 100) {
                _localX -= (100 - _localW);
                _localW = 100;
              }
            } else if (alignment.x > 0) {
              _localW += dx;
              if (_localW < 100) _localW = 100;
            }

            if (alignment.y < 0) {
              _localY += dy;
              _localH -= dy;
              if (_localH < 100) {
                _localY -= (100 - _localH);
                _localH = 100;
              }
            } else if (alignment.y > 0) {
              _localH += dy;
              if (_localH < 100) _localH = 100;
            }

            _localX = _localX.clamp(0.0, _canvasWidth - _localW);
            _localY = _localY.clamp(0.0, _canvasHeight - _localH);
            _localW = _localW.clamp(100.0, _canvasWidth - _localX);
            _localH = _localH.clamp(100.0, _canvasHeight - _localY);
          });
        },
        onPanEnd: (details) {
          double snappedX = (_localX / _gridSize).round() * _gridSize;
          double snappedY = (_localY / _gridSize).round() * _gridSize;
          double snappedW = (_localW / _gridSize).round() * _gridSize;
          double snappedH = (_localH / _gridSize).round() * _gridSize;

          snappedX = snappedX.clamp(0.0, _canvasWidth - snappedW);
          snappedY = snappedY.clamp(0.0, _canvasHeight - snappedH);
          snappedW = snappedW.clamp(100.0, _canvasWidth - snappedX);
          snappedH = snappedH.clamp(100.0, _canvasHeight - snappedY);

          ref.read(roomZoneProvider.notifier).updateRoomZone(
                widget.room.copyWith(x: snappedX, y: snappedY, width: snappedW, height: snappedH),
                immediate: true,
              );
        },
        child: Container(
          width: 14,
          height: 14,
          decoration: BoxDecoration(
            color: Colors.white,
            border: Border.all(color: Colors.black, width: 2),
            shape: BoxShape.circle,
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final roomColor = Color(widget.room.color);
    return Positioned(
      left: _localX,
      top: _localY,
      width: _localW,
      height: _localH,
      child: Stack(
        clipBehavior: Clip.none,
        children: [
          RepaintBoundary(
            child: GestureDetector(
              onPanStart: (details) {
                _draggedSpeakersOffsets = {
                  for (var s in widget.containedSpeakers) s.id: Offset(s.x, s.y)
                };
              },
              onPanUpdate: (details) {
                final scale = widget.transformationController.value.getMaxScaleOnAxis();
                final currentScale = scale > 0 ? scale : 1.0;
                final dx = details.delta.dx / currentScale;
                final dy = details.delta.dy / currentScale;

                setState(() {
                  _localX = (_localX + dx).clamp(0.0, _canvasWidth - _localW);
                  _localY = (_localY + dy).clamp(0.0, _canvasHeight - _localH);

                  final currentNodes = ref.read(speakerLayoutProvider);
                  for (final nodeId in _draggedSpeakersOffsets.keys) {
                    final prev = _draggedSpeakersOffsets[nodeId]!;
                    final nx = (prev.dx + dx).clamp(0.0, _canvasWidth - _speakerSize);
                    final ny = (prev.dy + dy).clamp(0.0, _canvasHeight - _speakerSize);
                    _draggedSpeakersOffsets[nodeId] = Offset(nx, ny);
                    try {
                      final node = currentNodes.firstWhere((n) => n.id == nodeId);
                      ref.read(speakerLayoutProvider.notifier).updateSpeaker(node.copyWith(x: nx, y: ny));
                    } catch (_) {}
                  }
                });
              },
              onPanEnd: (details) {
                double snappedX = (_localX / _gridSize).round() * _gridSize;
                double snappedY = (_localY / _gridSize).round() * _gridSize;
                snappedX = snappedX.clamp(0.0, _canvasWidth - _localW);
                snappedY = snappedY.clamp(0.0, _canvasHeight - _localH);

                ref.read(roomZoneProvider.notifier).updateRoomZone(
                      widget.room.copyWith(x: snappedX, y: snappedY),
                      immediate: true,
                    );

                final currentNodes = ref.read(speakerLayoutProvider);
                for (final nodeId in _draggedSpeakersOffsets.keys) {
                  try {
                    final node = currentNodes.firstWhere((n) => n.id == nodeId);
                    final nX = (node.x / _gridSize).round() * _gridSize;
                    final nY = (node.y / _gridSize).round() * _gridSize;
                    ref.read(speakerLayoutProvider.notifier).updateSpeaker(
                          node.copyWith(
                            x: nX.clamp(0.0, _canvasWidth - _speakerSize),
                            y: nY.clamp(0.0, _canvasHeight - _speakerSize),
                          ),
                          immediate: true,
                        );
                  } catch (_) {}
                }
                _draggedSpeakersOffsets.clear();
              },
              onDoubleTap: widget.onEdit,
              child: ClipRRect(
                borderRadius: BorderRadius.circular(8),
                child: BackdropFilter(
                  filter: ImageFilter.blur(sigmaX: 10, sigmaY: 10),
                  child: Container(
                    decoration: BoxDecoration(
                      color: roomColor.withValues(alpha: 0.15),
                      border: Border.all(
                        color: roomColor.withValues(alpha: 0.8),
                        width: 2,
                      ),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Center(
                      child: Text(
                        widget.room.label,
                        style: const TextStyle(
                          color: Colors.white,
                          fontSize: 18,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
          // Door IN
          Positioned(
            left: -12,
            top: _localH / 2 - 20,
            child: const Column(
              children: [
                Icon(Icons.door_front_door, color: Colors.greenAccent, size: 24),
                Text('IN', style: TextStyle(color: Colors.greenAccent, fontSize: 10, fontWeight: FontWeight.bold)),
              ],
            ),
          ),
          // Door OUT
          Positioned(
            right: -12,
            top: _localH / 2 - 20,
            child: const Column(
              children: [
                Icon(Icons.door_front_door, color: Colors.redAccent, size: 24),
                Text('OUT', style: TextStyle(color: Colors.redAccent, fontSize: 10, fontWeight: FontWeight.bold)),
              ],
            ),
          ),
          Positioned(
            top: -28,
            left: 0,
            child: Wrap(
              spacing: 4,
              children: widget.containedSpeakers.map((s) => Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(
                      color: roomColor,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      '[Ch ${s.channel + 1}]',
                      style: const TextStyle(
                        color: Colors.white,
                        fontSize: 12,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  )).toList(),
            ),
          ),
          _buildResizeHandle(Alignment.topLeft),
          _buildResizeHandle(Alignment.topCenter),
          _buildResizeHandle(Alignment.topRight),
          _buildResizeHandle(Alignment.centerLeft),
          _buildResizeHandle(Alignment.centerRight),
          _buildResizeHandle(Alignment.bottomLeft),
          _buildResizeHandle(Alignment.bottomCenter),
          _buildResizeHandle(Alignment.bottomRight),
        ],
      ),
    );
  }
}

class _DraggableSpeakerWidget extends ConsumerStatefulWidget {
  final SpeakerNode node;
  final Color? roomColor;
  final bool isDuplicate;
  final TransformationController transformationController;

  const _DraggableSpeakerWidget({
    super.key,
    required this.node,
    this.roomColor,
    required this.isDuplicate,
    required this.transformationController,
  });

  @override
  ConsumerState<_DraggableSpeakerWidget> createState() => _DraggableSpeakerWidgetState();
}

class _DraggableSpeakerWidgetState extends ConsumerState<_DraggableSpeakerWidget> {
  late double _localX;
  late double _localY;

  @override
  void initState() {
    super.initState();
    _localX = widget.node.x;
    _localY = widget.node.y;
  }

  @override
  void didUpdateWidget(covariant _DraggableSpeakerWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    _localX = widget.node.x;
    _localY = widget.node.y;
  }

  @override
  Widget build(BuildContext context) {
    return Positioned(
      left: _localX,
      top: _localY,
      child: RepaintBoundary(
        child: GestureDetector(
          onPanUpdate: (details) {
            final scale = widget.transformationController.value.getMaxScaleOnAxis();
            final currentScale = scale > 0 ? scale : 1.0;
            setState(() {
              _localX = (_localX + details.delta.dx / currentScale).clamp(0.0, _canvasWidth - _speakerSize);
              _localY = (_localY + details.delta.dy / currentScale).clamp(0.0, _canvasHeight - _speakerSize);
            });
          },
          onPanEnd: (details) {
            final snappedX = (_localX / _gridSize).round() * _gridSize;
            final snappedY = (_localY / _gridSize).round() * _gridSize;
            final updated = widget.node.copyWith(
              x: snappedX.clamp(0.0, _canvasWidth - _speakerSize),
              y: snappedY.clamp(0.0, _canvasHeight - _speakerSize),
            );
            ref.read(speakerLayoutProvider.notifier).updateSpeaker(updated, immediate: true);
          },
          child: SpeakerNodeWidget(
            node: widget.node,
            roomColor: widget.roomColor,
            onChannelChanged: (ch) {
              ref.read(speakerLayoutProvider.notifier).updateSpeaker(widget.node.copyWith(channel: ch));
            },
            onDelete: () {
              ref.read(speakerLayoutProvider.notifier).removeSpeaker(widget.node.id);
            },
            isDuplicateChannel: widget.isDuplicate,
          ),
        ),
      ),
    );
  }
}

class _DraggableWaypointWidget extends ConsumerStatefulWidget {
  final Trajectory trajectory;
  final int waypointIndex;
  final TrajectoryWaypoint waypoint;
  final TransformationController transformationController;

  const _DraggableWaypointWidget({
    super.key,
    required this.trajectory,
    required this.waypointIndex,
    required this.waypoint,
    required this.transformationController,
  });

  @override
  ConsumerState<_DraggableWaypointWidget> createState() => _DraggableWaypointWidgetState();
}

class _DraggableWaypointWidgetState extends ConsumerState<_DraggableWaypointWidget> {
  late double _localX;
  late double _localY;

  @override
  void initState() {
    super.initState();
    _localX = widget.waypoint.x;
    _localY = widget.waypoint.y;
  }

  @override
  void didUpdateWidget(covariant _DraggableWaypointWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    _localX = widget.waypoint.x;
    _localY = widget.waypoint.y;
  }

  @override
  Widget build(BuildContext context) {
    return Positioned(
      left: _localX - 20,
      top: _localY - 20,
      child: GestureDetector(
        onPanUpdate: (details) {
          final scale = widget.transformationController.value.getMaxScaleOnAxis();
          final currentScale = scale > 0 ? scale : 1.0;
          setState(() {
            _localX = (_localX + details.delta.dx / currentScale).clamp(0.0, _canvasWidth);
            _localY = (_localY + details.delta.dy / currentScale).clamp(0.0, _canvasHeight);
          });
          
          // 실시간으로 선이 따라가도록 Riverpod 업데이트
          final wps = List<TrajectoryWaypoint>.from(widget.trajectory.waypoints);
          wps[widget.waypointIndex] = TrajectoryWaypoint(_localX, _localY);
          ref.read(trajectoryProvider.notifier).updateTrajectory(
            widget.trajectory.copyWith(waypoints: wps),
          );
        },
        onDoubleTap: () {
          // 더블클릭시 삭제 (최소 2개의 점은 유지해야 함)
          if (widget.trajectory.waypoints.length > 2) {
             final wps = List<TrajectoryWaypoint>.from(widget.trajectory.waypoints);
             wps.removeAt(widget.waypointIndex);
             ref.read(trajectoryProvider.notifier).updateTrajectory(
               widget.trajectory.copyWith(waypoints: wps),
             );
          } else {
             ref.read(trajectoryProvider.notifier).removeTrajectory(widget.trajectory.id);
          }
        },
        child: Container(
          width: 40,
          height: 40,
          color: Colors.transparent,
          child: Center(
            child: Container(
              width: 16,
              height: 16,
              decoration: BoxDecoration(
                color: Colors.white,
                shape: BoxShape.circle,
                border: Border.all(color: Colors.cyanAccent, width: 3),
              ),
            ),
          ),
        ),
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

    final headPaint = Paint()
      ..color = Colors.cyanAccent
      ..style = PaintingStyle.fill;

    for (var t in trajectories) {
      if (t.waypoints.length < 2) continue;

      final path = Path();
      path.moveTo(t.waypoints.first.x, t.waypoints.first.y);

      for (int i = 1; i < t.waypoints.length; i++) {
        path.lineTo(t.waypoints[i].x, t.waypoints[i].y);
      }
      canvas.drawPath(path, paint);

      // 화살표 머리 렌더링 (Arrowhead) - 끝에서 두번째 점과 마지막 점 사이의 각도를 이용
      final p1 = t.waypoints[t.waypoints.length - 2];
      final p2 = t.waypoints.last;
      final angle = math.atan2(p2.y - p1.y, p2.x - p1.x);
      final headLength = 15.0;
      final headAngle = math.pi / 6; // 30 degrees

      final h1x = p2.x - headLength * math.cos(angle - headAngle);
      final h1y = p2.y - headLength * math.sin(angle - headAngle);
      final h2x = p2.x - headLength * math.cos(angle + headAngle);
      final h2y = p2.y - headLength * math.sin(angle + headAngle);

      final headPath = Path()
        ..moveTo(p2.x, p2.y)
        ..lineTo(h1x, h1y)
        ..lineTo(h2x, h2y)
        ..close();

      canvas.drawPath(headPath, headPaint);
    }
  }

  @override
  bool shouldRepaint(covariant _TrajectoryPainter oldDelegate) {
    return true;
  }
}
