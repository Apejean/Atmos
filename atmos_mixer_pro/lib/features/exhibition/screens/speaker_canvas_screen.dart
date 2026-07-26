import 'dart:ui';
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

enum CanvasMode { speaker, room, trajectory }

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
  CanvasMode _currentMode = CanvasMode.speaker;
  String? _drawingTrajectoryId;

  static int _roomColorIndex = 0;
  List<String> _draggedSpeakerIds = [];

  @override
  void initState() {
    super.initState();
    // Center the view initially
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

  void _addTrajectoryPoint(Offset localPosition) {
    // Clamp the points to prevent drawing out of canvas bounds
    final clampedX = localPosition.dx.clamp(0.0, _canvasWidth);
    final clampedY = localPosition.dy.clamp(0.0, _canvasHeight);

    if (_drawingTrajectoryId == null) {
      _drawingTrajectoryId = const Uuid().v4();
      final t = Trajectory(
        id: _drawingTrajectoryId!,
        waypoints: [TrajectoryWaypoint(clampedX, clampedY)],
      );
      ref.read(trajectoryProvider.notifier).addTrajectory(t);
    } else {
      final trajectories = ref.read(trajectoryProvider);
      final t = trajectories.firstWhere(
        (element) => element.id == _drawingTrajectoryId,
      );
      final updated = t.copyWith(
        waypoints: [...t.waypoints, TrajectoryWaypoint(clampedX, clampedY)],
      );
      ref.read(trajectoryProvider.notifier).updateTrajectory(updated);
    }
  }

  void _endTrajectory() {
    _drawingTrajectoryId = null;
    setState(() {});
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
              const Text(
                'Theme Color',
                style: TextStyle(color: Colors.white70),
              ),
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
              child: const Text(
                'Delete Room',
                style: TextStyle(color: Colors.redAccent),
              ),
            ),
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text(
                'Cancel',
                style: TextStyle(color: Colors.white70),
              ),
            ),
            TextButton(
              onPressed: () {
                ref
                    .read(roomZoneProvider.notifier)
                    .updateRoomZone(
                      room.copyWith(
                        label: controller.text,
                        color: selectedColor,
                      ),
                      immediate: true,
                    );
                Navigator.pop(context);
              },
              child: const Text(
                'Save',
                style: TextStyle(color: AppColors.primaryNeon),
              ),
            ),
          ],
        ),
      ),
    );
  }

  List<SpeakerNode> _getSpeakersInRoom(RoomZone room, List<SpeakerNode> nodes) {
    return nodes
        .where(
          (n) => room.containsPoint(
            n.x + _speakerSize / 2,
            n.y + _speakerSize / 2,
          ),
        )
        .toList();
  }

  Color? _getRoomColorForSpeaker(SpeakerNode speaker, List<RoomZone> rooms) {
    for (final room in rooms) {
      if (room.containsPoint(
        speaker.x + _speakerSize / 2,
        speaker.y + _speakerSize / 2,
      )) {
        return Color(room.color);
      }
    }
    return null;
  }

  Widget _buildResizeHandle(Alignment alignment, RoomZone room) {
    return Align(
      alignment: alignment,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onPanUpdate: (details) {
          final scale = _transformationController.value.getMaxScaleOnAxis();
          final currentScale = scale > 0 ? scale : 1.0;
          final dx = details.delta.dx / currentScale;
          final dy = details.delta.dy / currentScale;

          final currentRoom = ref
              .read(roomZoneProvider)
              .firstWhere((r) => r.id == room.id);
          double newX = currentRoom.x;
          double newY = currentRoom.y;
          double newW = currentRoom.width;
          double newH = currentRoom.height;

          if (alignment.x < 0) {
            newX += dx;
            newW -= dx;
            if (newW < 100) {
              newX -= (100 - newW);
              newW = 100;
            }
          } else if (alignment.x > 0) {
            newW += dx;
            if (newW < 100) newW = 100;
          }

          if (alignment.y < 0) {
            newY += dy;
            newH -= dy;
            if (newH < 100) {
              newY -= (100 - newH);
              newH = 100;
            }
          } else if (alignment.y > 0) {
            newH += dy;
            if (newH < 100) newH = 100;
          }

          newX = newX.clamp(0.0, _canvasWidth - newW);
          newY = newY.clamp(0.0, _canvasHeight - newH);
          newW = newW.clamp(100.0, _canvasWidth - newX);
          newH = newH.clamp(100.0, _canvasHeight - newY);

          ref
              .read(roomZoneProvider.notifier)
              .updateRoomZone(
                currentRoom.copyWith(
                  x: newX,
                  y: newY,
                  width: newW,
                  height: newH,
                ),
              );
        },
        onPanEnd: (details) {
          final currentRoom = ref
              .read(roomZoneProvider)
              .firstWhere((r) => r.id == room.id);
          double snappedX = (currentRoom.x / _gridSize).round() * _gridSize;
          double snappedY = (currentRoom.y / _gridSize).round() * _gridSize;
          double snappedW = (currentRoom.width / _gridSize).round() * _gridSize;
          double snappedH = (currentRoom.height / _gridSize).round() * _gridSize;

          snappedX = snappedX.clamp(0.0, _canvasWidth - snappedW);
          snappedY = snappedY.clamp(0.0, _canvasHeight - snappedH);
          snappedW = snappedW.clamp(100.0, _canvasWidth - snappedX);
          snappedH = snappedH.clamp(100.0, _canvasHeight - snappedY);

          ref
              .read(roomZoneProvider.notifier)
              .updateRoomZone(
                currentRoom.copyWith(
                  x: snappedX,
                  y: snappedY,
                  width: snappedW,
                  height: snappedH,
                ),
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
              fillColor: AppColors.primaryNeon.withValues(alpha: 0.1),
              borderColor: Colors.white24,
              selectedBorderColor: AppColors.primaryNeon,
              children: const [
                Padding(
                  padding: EdgeInsets.symmetric(horizontal: 12),
                  child: Text('Speakers'),
                ),
                Padding(
                  padding: EdgeInsets.symmetric(horizontal: 12),
                  child: Text('Rooms'),
                ),
                Padding(
                  padding: EdgeInsets.symmetric(horizontal: 12),
                  child: Text('Trajectories'),
                ),
              ],
            ),
            const SizedBox(width: 16),
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
                        padding: const EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 4,
                        ),
                        decoration: BoxDecoration(
                          color: Colors.orange.shade800.withValues(alpha: 0.8),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: const Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(
                              Icons.warning_amber_rounded,
                              color: Colors.white,
                              size: 16,
                            ),
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
                      child: RepaintBoundary(
                        child: CustomPaint(painter: _GridPainter()),
                      ),
                    ),

                    // Rooms
                    Consumer(
                      builder: (context, ref, _) {
                        final rooms = ref.watch(roomZoneProvider);
                        final nodes = ref.watch(speakerLayoutProvider);
                        return Stack(
                          clipBehavior: Clip.none,
                          children: rooms.map((room) {
                            final containedSpeakers = _getSpeakersInRoom(room, nodes);
                      final roomColor = Color(room.color);

                      return Positioned(
                        left: room.x,
                        top: room.y,
                        width: room.width,
                        height: room.height,
                        child: Stack(
                          clipBehavior: Clip.none,
                          children: [
                            // Room Body
                            RepaintBoundary(
                              child: GestureDetector(
                                onPanStart: _currentMode == CanvasMode.room
                                    ? (details) {
                                        _draggedSpeakerIds = containedSpeakers
                                            .map((e) => e.id)
                                            .toList();
                                      }
                                    : null,
                              onPanUpdate: _currentMode == CanvasMode.room
                                  ? (details) {
                                      final scale = _transformationController
                                          .value
                                          .getMaxScaleOnAxis();
                                      final currentScale = scale > 0
                                          ? scale
                                          : 1.0;
                                      double dx =
                                          details.delta.dx / currentScale;
                                      double dy =
                                          details.delta.dy / currentScale;

                                      final currentRoom = ref
                                          .read(roomZoneProvider)
                                          .firstWhere((r) => r.id == room.id);
                                      double newX = (currentRoom.x + dx).clamp(
                                        0.0,
                                        _canvasWidth - currentRoom.width,
                                      );
                                      double newY = (currentRoom.y + dy).clamp(
                                        0.0,
                                        _canvasHeight - currentRoom.height,
                                      );

                                      ref
                                          .read(roomZoneProvider.notifier)
                                          .updateRoomZone(
                                            currentRoom.copyWith(
                                              x: newX,
                                              y: newY,
                                            ),
                                          );

                                      // Group Drag: Move contained speakers
                                      final currentNodes = ref.read(
                                        speakerLayoutProvider,
                                      );
                                      for (final nodeId in _draggedSpeakerIds) {
                                        try {
                                          final node = currentNodes.firstWhere(
                                            (n) => n.id == nodeId,
                                          );
                                          final nX = (node.x + dx).clamp(
                                            0.0,
                                            _canvasWidth - _speakerSize,
                                          );
                                          final nY = (node.y + dy).clamp(
                                            0.0,
                                            _canvasHeight - _speakerSize,
                                          );
                                          ref
                                              .read(
                                                speakerLayoutProvider.notifier,
                                              )
                                              .updateSpeaker(
                                                node.copyWith(x: nX, y: nY),
                                              );
                                        } catch (_) {}
                                      }
                                    }
                                  : null,
                              onPanEnd: _currentMode == CanvasMode.room
                                  ? (details) {
                                      final currentRoom = ref
                                          .read(roomZoneProvider)
                                          .firstWhere((r) => r.id == room.id);
                                      double snappedX =
                                          ((currentRoom.x / _gridSize).round() *
                                                  _gridSize)
                                              .clamp(
                                                0.0,
                                                _canvasWidth -
                                                    currentRoom.width,
                                              );
                                      double snappedY =
                                          ((currentRoom.y / _gridSize).round() *
                                                  _gridSize)
                                              .clamp(
                                                0.0,
                                                _canvasHeight -
                                                    currentRoom.height,
                                              );
                                      ref
                                          .read(roomZoneProvider.notifier)
                                          .updateRoomZone(
                                            currentRoom.copyWith(
                                              x: snappedX,
                                              y: snappedY,
                                            ),
                                            immediate: true,
                                          );

                                      // Group Drag snap
                                      final currentNodes = ref.read(
                                        speakerLayoutProvider,
                                      );
                                      for (final nodeId in _draggedSpeakerIds) {
                                        try {
                                          final node = currentNodes.firstWhere(
                                            (n) => n.id == nodeId,
                                          );
                                          final nX =
                                              (node.x / _gridSize).round() *
                                              _gridSize;
                                          final nY =
                                              (node.y / _gridSize).round() *
                                              _gridSize;
                                          ref
                                              .read(
                                                speakerLayoutProvider.notifier,
                                              )
                                              .updateSpeaker(
                                                node.copyWith(
                                                  x: nX.clamp(
                                                    0.0,
                                                    _canvasWidth - _speakerSize,
                                                  ),
                                                  y: nY.clamp(
                                                    0.0,
                                                    _canvasHeight -
                                                        _speakerSize,
                                                  ),
                                                ),
                                                immediate: true,
                                              );
                                        } catch (_) {}
                                      }
                                      _draggedSpeakerIds = [];
                                    }
                                  : null,
                              onDoubleTap: _currentMode == CanvasMode.room
                                  ? () => _editRoom(room)
                                  : null,
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
                                        room.label,
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

                            // Channel Badges (above room)
                            Positioned(
                              top: -28,
                              left: 0,
                              child: Wrap(
                                spacing: 4,
                                children: containedSpeakers
                                    .map(
                                      (s) => Container(
                                        padding: const EdgeInsets.symmetric(
                                          horizontal: 6,
                                          vertical: 2,
                                        ),
                                        decoration: BoxDecoration(
                                          color: roomColor,
                                          borderRadius: BorderRadius.circular(
                                            4,
                                          ),
                                        ),
                                        child: Text(
                                          '[Ch ${s.channel + 1}]',
                                          style: const TextStyle(
                                            color: Colors.white,
                                            fontSize: 12,
                                            fontWeight: FontWeight.bold,
                                          ),
                                        ),
                                      ),
                                    )
                                    .toList(),
                              ),
                            ),

                            // 8-way Resize Handles
                            if (_currentMode == CanvasMode.room) ...[
                              _buildResizeHandle(Alignment.topLeft, room),
                              _buildResizeHandle(Alignment.topCenter, room),
                              _buildResizeHandle(Alignment.topRight, room),
                              _buildResizeHandle(Alignment.centerLeft, room),
                              _buildResizeHandle(Alignment.centerRight, room),
                              _buildResizeHandle(Alignment.bottomLeft, room),
                              _buildResizeHandle(Alignment.bottomCenter, room),
                              _buildResizeHandle(Alignment.bottomRight, room),
                            ],
                          ],
                        ),
                      );
                    }).toList(),
                  );
                }),

                    // Trajectories
                    Consumer(
                      builder: (context, ref, _) {
                        final trajectories = ref.watch(trajectoryProvider);
                        return Positioned.fill(
                          child: RepaintBoundary(
                            child: CustomPaint(
                              painter: _TrajectoryPainter(trajectories),
                            ),
                          ),
                        );
                      },
                    ),

                    // Speaker Nodes
                    Consumer(
                      builder: (context, ref, _) {
                        final nodes = ref.watch(speakerLayoutProvider);
                        final rooms = ref.watch(roomZoneProvider);
                        return Stack(
                          clipBehavior: Clip.none,
                          children: nodes.map((node) {
                            final isDuplicate = nodes
                          .where(
                            (n) => n.id != node.id && n.channel == node.channel,
                          )
                          .isNotEmpty;
                      final roomColor = _getRoomColorForSpeaker(node, rooms);

                      return Positioned(
                        left: node.x,
                        top: node.y,
                        child: RepaintBoundary(
                          child: GestureDetector(
                            onPanUpdate: _currentMode == CanvasMode.speaker
                                ? (details) {
                                    final scale = _transformationController
                                        .value
                                        .getMaxScaleOnAxis();
                                    final currentScale = scale > 0
                                        ? scale
                                        : 1.0;

                                    final currentNode = ref
                                        .read(speakerLayoutProvider)
                                        .firstWhere((n) => n.id == node.id);
                                    double newX =
                                        currentNode.x +
                                        (details.delta.dx / currentScale);
                                    double newY =
                                        currentNode.y +
                                        (details.delta.dy / currentScale);

                                    final updated = currentNode.copyWith(
                                      x: newX.clamp(
                                        0.0,
                                        _canvasWidth - _speakerSize,
                                      ),
                                      y: newY.clamp(
                                        0.0,
                                        _canvasHeight - _speakerSize,
                                      ),
                                    );
                                    ref
                                        .read(speakerLayoutProvider.notifier)
                                        .updateSpeaker(updated);
                                  }
                                : null,
                            onPanEnd: _currentMode == CanvasMode.speaker
                                ? (details) {
                                    final currentNode = ref
                                        .read(speakerLayoutProvider)
                                        .firstWhere((n) => n.id == node.id);
                                    final snappedX =
                                        (currentNode.x / _gridSize).round() *
                                        _gridSize;
                                    final snappedY =
                                        (currentNode.y / _gridSize).round() *
                                        _gridSize;
                                    final updated = currentNode.copyWith(
                                      x: snappedX.clamp(
                                        0.0,
                                        _canvasWidth - _speakerSize,
                                      ),
                                      y: snappedY.clamp(
                                        0.0,
                                        _canvasHeight - _speakerSize,
                                      ),
                                    );
                                    ref
                                        .read(speakerLayoutProvider.notifier)
                                        .updateSpeaker(
                                          updated,
                                          immediate: true,
                                        );
                                  }
                                : null,
                            child: AbsorbPointer(
                              absorbing: _currentMode != CanvasMode.speaker,
                              child: SpeakerNodeWidget(
                                node: node,
                                roomColor: roomColor,
                                onChannelChanged: (ch) {
                                  ref
                                      .read(speakerLayoutProvider.notifier)
                                      .updateSpeaker(
                                        node.copyWith(channel: ch),
                                      );
                                },
                                onDelete: () {
                                  ref
                                      .read(speakerLayoutProvider.notifier)
                                      .removeSpeaker(node.id);
                                },
                                isDuplicateChannel: isDuplicate,
                              ),
                            ),
                          ),
                        ),
                      );
                    }).toList(),
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
              onPressed: _currentMode == CanvasMode.speaker
                  ? _addSpeaker
                  : _addRoom,
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
      canvas.drawCircle(
        Offset(t.waypoints.first.x, t.waypoints.first.y),
        6,
        dotPaint,
      );

      for (int i = 1; i < t.waypoints.length; i++) {
        path.lineTo(t.waypoints[i].x, t.waypoints[i].y);
        canvas.drawCircle(
          Offset(t.waypoints[i].x, t.waypoints[i].y),
          6,
          dotPaint,
        );
      }
      canvas.drawPath(path, paint);
    }
  }

  @override
  bool shouldRepaint(covariant _TrajectoryPainter oldDelegate) {
    return true;
  }
}
