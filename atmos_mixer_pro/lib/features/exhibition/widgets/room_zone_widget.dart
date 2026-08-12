import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';

const double _canvasWidth = 2000.0;
const double _canvasHeight = 2000.0;
const double _speakerSize = 60.0;

class RoomZoneWidget extends ConsumerStatefulWidget {
  final RoomZone room;
  final List<SpeakerNode> containedSpeakers;
  final TransformationController transformationController;
  final VoidCallback onEdit;
  final VoidCallback? onDragUpdate;
  final bool isSelected;
  final VoidCallback? onInteractionStart;
  final VoidCallback? onInteractionEnd;

  const RoomZoneWidget({
    super.key,
    required this.room,
    required this.containedSpeakers,
    required this.transformationController,
    required this.onEdit,
    this.onDragUpdate,
    this.isSelected = false,
    this.onInteractionStart,
    this.onInteractionEnd,
  });

  @override
  ConsumerState<RoomZoneWidget> createState() => RoomZoneWidgetState();
}

class RoomZoneWidgetState extends ConsumerState<RoomZoneWidget> {
  late double _localX;
  late double _localY;
  late double _localW;
  late double _localH;
  bool _isInteracting = false;
  Map<String, Offset> _draggedSpeakersOffsets = {};

  double _initialRotation = 0.0;
  double _initialAngleToCenter = 0.0;

  @override
  void initState() {
    super.initState();
    _localX = widget.room.x;
    _localY = widget.room.y;
    _localW = widget.room.width;
    _localH = widget.room.height;
  }

  @override
  void didUpdateWidget(covariant RoomZoneWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Remove the _isInteracting guard completely because it prevents the widget
    // from adopting external state changes (like rotation, size from settings dialog)
    // while the user might not be actively "dragging" but the widget thinks it is.
    // Instead, only adopt parent state if we're not currently generating local pan delta.
    if (!_isInteracting) {
      _localX = widget.room.x;
      _localY = widget.room.y;
      _localW = widget.room.width;
      _localH = widget.room.height;
    }
  }

  Widget _buildResizeHandle(Alignment alignment) {
    MouseCursor cursor = SystemMouseCursors.basic;
    if (alignment == Alignment.topLeft || alignment == Alignment.bottomRight) {
      cursor = SystemMouseCursors.resizeUpLeftDownRight;
    } else if (alignment == Alignment.topRight ||
        alignment == Alignment.bottomLeft) {
      cursor = SystemMouseCursors.resizeUpRightDownLeft;
    } else if (alignment == Alignment.topCenter ||
        alignment == Alignment.bottomCenter) {
      cursor = SystemMouseCursors.resizeUpDown;
    } else if (alignment == Alignment.centerLeft ||
        alignment == Alignment.centerRight) {
      cursor = SystemMouseCursors.resizeLeftRight;
    }

    // Calculate center of handle based on the 40px margin
    double centerX = 40.0 + (alignment.x + 1.0) / 2.0 * _localW;
    double centerY = 40.0 + (alignment.y + 1.0) / 2.0 * _localH;

    return Positioned(
      left: centerX - 20.0,
      top: centerY - 20.0,
      child: MouseRegion(
        cursor: cursor,
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onPanStart: (details) {
            setState(() {
              _isInteracting = true;
              widget.onInteractionStart?.call();
            });
          },
          onPanUpdate: (details) {
            final scale = widget.transformationController.value
                .getMaxScaleOnAxis();
            final currentScale = scale > 0 ? scale : 1.0;
            final dxGlobal = details.delta.dx / currentScale;
            final dyGlobal = details.delta.dy / currentScale;

            // Convert global gesture delta to room's rotated local coordinate system
            final rad = widget.room.rotation * math.pi / 180.0;
            final cosA = math.cos(-rad);
            final sinA = math.sin(-rad);
            final dx = dxGlobal * cosA - dyGlobal * sinA;
            final dy = dxGlobal * sinA + dyGlobal * cosA;

            setState(() {
              double dxLocal = dx;
              double dyLocal = dy;

              double newLx = 0.0;
              double newLy = 0.0;
              double newW = _localW;
              double newH = _localH;

              if (alignment.x < 0) {
                newLx = dxLocal;
                newW -= dxLocal;
              } else if (alignment.x > 0) {
                newW += dxLocal;
              }

              if (alignment.y < 0) {
                newLy = dyLocal;
                newH -= dyLocal;
              } else if (alignment.y > 0) {
                newH += dyLocal;
              }

              if (newW < 80.0) {
                if (alignment.x < 0) newLx -= (80.0 - newW);
                newW = 80.0;
              }
              if (newH < 80.0) {
                if (alignment.y < 0) newLy -= (80.0 - newH);
                newH = 80.0;
              }

              double deltaCx = newLx + newW / 2 - _localW / 2;
              double deltaCy = newLy + newH / 2 - _localH / 2;

              double deltaCxGlobal =
                  deltaCx * math.cos(rad) - deltaCy * math.sin(rad);
              double deltaCyGlobal =
                  deltaCx * math.sin(rad) + deltaCy * math.cos(rad);

              double oldCx = _localX + _localW / 2;
              double oldCy = _localY + _localH / 2;

              _localW = newW;
              _localH = newH;
              _localX = (oldCx + deltaCxGlobal) - _localW / 2;
              _localY = (oldCy + deltaCyGlobal) - _localH / 2;
            });

            final blueprint = ref.read(blueprintProvider);
            final scaleM = blueprint.scale > 0 ? blueprint.scale : 40.0;
            ref
                .read(roomZoneProvider.notifier)
                .updateRoomZone(
                  widget.room.copyWith(
                    x: _localX,
                    y: _localY,
                    width: _localW,
                    height: _localH,
                    physicalWidth: _localW / scaleM,
                    physicalHeight: _localH / scaleM,
                  ),
                  immediate: false,
                );
            widget.onDragUpdate?.call();
          },
          onPanEnd: (_) {
            double snappedX =
                (_localX / ref.read(blueprintProvider).scale).round() *
                ref.read(blueprintProvider).scale;
            double snappedY =
                (_localY / ref.read(blueprintProvider).scale).round() *
                ref.read(blueprintProvider).scale;
            double snappedW =
                (_localW / ref.read(blueprintProvider).scale).round() *
                ref.read(blueprintProvider).scale;
            double snappedH =
                (_localH / ref.read(blueprintProvider).scale).round() *
                ref.read(blueprintProvider).scale;

            snappedX = snappedX.clamp(0.0, _canvasWidth - snappedW);
            snappedY = snappedY.clamp(0.0, _canvasHeight - snappedH);
            snappedW = snappedW.clamp(80.0, _canvasWidth - snappedX);
            snappedH = snappedH.clamp(80.0, _canvasHeight - snappedY);

            final blueprint = ref.read(blueprintProvider);
            final scaleM = blueprint.scale > 0 ? blueprint.scale : 40.0;

            setState(() {
              _localX = snappedX;
              _localY = snappedY;
              _localW = snappedW;
              _localH = snappedH;
              _isInteracting = false;
            });
            widget.onInteractionEnd?.call();

            ref
                .read(roomZoneProvider.notifier)
                .updateRoomZone(
                  widget.room.copyWith(
                    x: snappedX,
                    y: snappedY,
                    width: snappedW,
                    height: snappedH,
                    physicalWidth: snappedW / scaleM,
                    physicalHeight: snappedH / scaleM,
                  ),
                  immediate: true,
                );
            widget.onDragUpdate?.call();
          },
          child: Container(
            width: 40,
            height: 40,
            color: Colors.transparent,
            child: Center(
              child: Container(
                width: 8,
                height: 8,
                decoration: BoxDecoration(
                  color: Colors.white,
                  border: Border.all(color: AppColors.primaryNeon, width: 1.5),
                  shape: BoxShape.rectangle,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildDoorHandle() {
    if (!widget.room.hasDoor) return const SizedBox.shrink();

    double left = 0, top = 0;
    final double doorWidth = 44.0;

    if (widget.room.doorWall == 0) {
      left = 40.0 + (widget.room.doorOffset * _localW) - doorWidth / 2;
      top = 40.0 - doorWidth / 2;
    } else if (widget.room.doorWall == 1) {
      left = 40.0 + _localW - doorWidth / 2;
      top = 40.0 + (widget.room.doorOffset * _localH) - doorWidth / 2;
    } else if (widget.room.doorWall == 2) {
      left = 40.0 + (widget.room.doorOffset * _localW) - doorWidth / 2;
      top = 40.0 + _localH - doorWidth / 2;
    } else if (widget.room.doorWall == 3) {
      left = 40.0 - doorWidth / 2;
      top = 40.0 + (widget.room.doorOffset * _localH) - doorWidth / 2;
    }

    return Positioned(
      left: left,
      top: top,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onPanStart: (_) {
          setState(() => _isInteracting = true);
          widget.onInteractionStart?.call();
        },
        onPanUpdate: (details) {
          final scale = widget.transformationController.value
              .getMaxScaleOnAxis();
          final currentScale = scale > 0 ? scale : 1.0;
          final dx = details.delta.dx / currentScale;
          final dy = details.delta.dy / currentScale;

          double newOffset = widget.room.doorOffset;
          if (widget.room.doorWall == 0 || widget.room.doorWall == 2) {
            newOffset += dx / (_localW > 0 ? _localW : 1.0);
          } else {
            newOffset += dy / (_localH > 0 ? _localH : 1.0);
          }
          newOffset = newOffset.clamp(0.0, 1.0);

          ref
              .read(roomZoneProvider.notifier)
              .updateRoomZone(
                widget.room.copyWith(doorOffset: newOffset),
                immediate: true,
              );
        },
        onPanEnd: (_) {
          setState(() => _isInteracting = false);
          widget.onInteractionEnd?.call();
        },
        child: Container(
          width: doorWidth,
          height: doorWidth,
          color: Colors.transparent,
          child: CustomPaint(
            painter: CadDoorPainter(
              color: Colors.cyanAccent,
              wall: widget.room.doorWall,
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildRotateHandle() {
    return Positioned(
      top: 4.0,
      left: 40.0 + _localW / 2 - 12.0,
      child: MouseRegion(
        cursor: SystemMouseCursors.click,
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onPanStart: (details) {
            setState(() => _isInteracting = true);
            widget.onInteractionStart?.call();
            final renderBox = context.findRenderObject() as RenderBox?;
            if (renderBox != null) {
              final localTouch = renderBox.globalToLocal(
                details.globalPosition,
              );
              final roomCenterLocal = Offset(
                40.0 + _localW / 2,
                40.0 + _localH / 2,
              );
              _initialAngleToCenter = math.atan2(
                localTouch.dy - roomCenterLocal.dy,
                localTouch.dx - roomCenterLocal.dx,
              );
              _initialRotation = widget.room.rotation;
            }
          },
          onPanUpdate: (details) {
            final renderBox = context.findRenderObject() as RenderBox?;
            if (renderBox != null) {
              final localTouch = renderBox.globalToLocal(
                details.globalPosition,
              );
              final roomCenterLocal = Offset(
                40.0 + _localW / 2,
                40.0 + _localH / 2,
              );
              final currentAngleToCenter = math.atan2(
                localTouch.dy - roomCenterLocal.dy,
                localTouch.dx - roomCenterLocal.dx,
              );
              final deltaAngle =
                  (currentAngleToCenter - _initialAngleToCenter) *
                  180 /
                  math.pi;
              double newRotation = (_initialRotation + deltaAngle) % 360;
              if (newRotation < 0) newRotation += 360;

              ref
                  .read(roomZoneProvider.notifier)
                  .updateRoomZone(
                    widget.room.copyWith(rotation: newRotation),
                    immediate: false,
                  );
            }
          },
          onPanEnd: (_) {
            setState(() => _isInteracting = false);
            widget.onInteractionEnd?.call();
          },
          child: Container(
            width: 24,
            height: 24,
            decoration: BoxDecoration(
              color: AppColors.cardSurface,
              shape: BoxShape.circle,
              border: Border.all(color: AppColors.primaryNeon, width: 2),
              boxShadow: [
                BoxShadow(
                  color: AppColors.primaryNeon.withValues(alpha: 0.4),
                  blurRadius: 6,
                ),
              ],
            ),
            child: const Icon(
              Icons.rotate_right,
              size: 14,
              color: AppColors.primaryNeon,
            ),
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final roomColor = widget.isSelected
        ? Colors.blueAccent.withValues(alpha: 0.3)
        : Color(widget.room.color);
    final isHoveredOrActive = _isInteracting;

    return Positioned(
      left: _localX - 40,
      top: _localY - 40,
      width: _localW + 80,
      height: _localH + 80,
      child: SizedBox(
        width: _localW + 80,
        height: _localH + 80,
        child: Transform.rotate(
          angle: widget.room.rotation * math.pi / 180.0,
          child: Stack(
            clipBehavior: Clip.none,
            children: [
              // Room Main Container & Drag Move Gesture
              Positioned(
                left: 40.0,
                top: 40.0,
                right: 40.0,
                bottom: 40.0,
                child: MouseRegion(
                  cursor: SystemMouseCursors.move,
                  child: GestureDetector(
                    behavior: HitTestBehavior.opaque,
                    onDoubleTap: widget.onEdit,
                    onTap: () {
                      setState(() => _isInteracting = true);
                      widget.onInteractionStart?.call();
                      Future.delayed(const Duration(milliseconds: 150), () {
                        if (mounted) setState(() => _isInteracting = false);
                        widget.onInteractionEnd?.call();
                      });
                    },
                    onPanStart: (details) {
                      setState(() => _isInteracting = true);
                      widget.onInteractionStart?.call();
                      _draggedSpeakersOffsets = {
                        for (var s in widget.containedSpeakers)
                          s.id: Offset(s.x, s.y),
                      };
                    },
                    onPanUpdate: (details) {
                      final scale = widget.transformationController.value
                          .getMaxScaleOnAxis();
                      final currentScale = scale > 0 ? scale : 1.0;
                      final dx = details.delta.dx / currentScale;
                      final dy = details.delta.dy / currentScale;

                      setState(() {
                        _localX = (_localX + dx).clamp(
                          0.0,
                          _canvasWidth - _localW,
                        );
                        _localY = (_localY + dy).clamp(
                          0.0,
                          _canvasHeight - _localH,
                        );

                        final currentNodes = ref.read(speakerLayoutProvider);
                        for (final nodeId in _draggedSpeakersOffsets.keys) {
                          final prev = _draggedSpeakersOffsets[nodeId]!;
                          final nx = (prev.dx + dx).clamp(
                            0.0,
                            _canvasWidth - _speakerSize,
                          );
                          final ny = (prev.dy + dy).clamp(
                            0.0,
                            _canvasHeight - _speakerSize,
                          );
                          _draggedSpeakersOffsets[nodeId] = Offset(nx, ny);
                          try {
                            final node = currentNodes.firstWhere(
                              (n) => n.id == nodeId,
                            );
                            ref
                                .read(speakerLayoutProvider.notifier)
                                .updateSpeaker(node.copyWith(x: nx, y: ny));
                          } catch (_) {}
                        }
                      });
                      ref
                          .read(roomZoneProvider.notifier)
                          .updateRoomZone(
                            widget.room.copyWith(x: _localX, y: _localY),
                            immediate: false,
                          );
                      widget.onDragUpdate?.call();
                    },
                    onPanEnd: (details) {
                      double snappedX =
                          (_localX / ref.read(blueprintProvider).scale)
                              .round() *
                          ref.read(blueprintProvider).scale;
                      double snappedY =
                          (_localY / ref.read(blueprintProvider).scale)
                              .round() *
                          ref.read(blueprintProvider).scale;
                      snappedX = snappedX.clamp(0.0, _canvasWidth - _localW);
                      snappedY = snappedY.clamp(0.0, _canvasHeight - _localH);

                      setState(() {
                        _localX = snappedX;
                        _localY = snappedY;
                        _isInteracting = false;
                      });
                      widget.onInteractionEnd?.call();

                      ref
                          .read(roomZoneProvider.notifier)
                          .updateRoomZone(
                            widget.room.copyWith(x: snappedX, y: snappedY),
                            immediate: true,
                          );

                      final currentNodes = ref.read(speakerLayoutProvider);
                      for (final nodeId in _draggedSpeakersOffsets.keys) {
                        try {
                          final node = currentNodes.firstWhere(
                            (n) => n.id == nodeId,
                          );
                          final nX =
                              (node.x / ref.read(blueprintProvider).scale)
                                  .round() *
                              ref.read(blueprintProvider).scale;
                          final nY =
                              (node.y / ref.read(blueprintProvider).scale)
                                  .round() *
                              ref.read(blueprintProvider).scale;
                          ref
                              .read(speakerLayoutProvider.notifier)
                              .updateSpeaker(
                                node.copyWith(
                                  x: nX.clamp(0.0, _canvasWidth - _speakerSize),
                                  y: nY.clamp(
                                    0.0,
                                    _canvasHeight - _speakerSize,
                                  ),
                                ),
                                immediate: true,
                              );
                        } catch (_) {}
                      }
                      _draggedSpeakersOffsets.clear();
                      widget.onDragUpdate?.call();
                    },
                    child: ClipRRect(
                      borderRadius: BorderRadius.circular(8),
                      child: Container(
                        decoration: BoxDecoration(
                          color: isHoveredOrActive
                              ? roomColor.withValues(alpha: 0.35)
                              : roomColor.withValues(alpha: 0.20),
                          border: Border.all(
                            color: isHoveredOrActive
                                ? AppColors.primaryNeon
                                : roomColor,
                            width: isHoveredOrActive ? 3.0 : 2.5,
                          ),
                          borderRadius: BorderRadius.circular(8),
                          boxShadow: [
                            BoxShadow(
                              color: isHoveredOrActive
                                  ? AppColors.primaryNeon.withValues(alpha: 0.5)
                                  : roomColor.withValues(alpha: 0.25),
                              blurRadius: isHoveredOrActive ? 16 : 12,
                              spreadRadius: isHoveredOrActive ? 2 : 1,
                            ),
                          ],
                        ),
                        child: Center(
                          child: Column(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Row(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  Text(
                                    widget.room.label,
                                    style: const TextStyle(
                                      color: Colors.white,
                                      fontSize: 16,
                                      fontWeight: FontWeight.bold,
                                      letterSpacing: 0.5,
                                    ),
                                  ),
                                  const SizedBox(width: 8),
                                  MouseRegion(
                                    cursor: SystemMouseCursors.click,
                                    child: GestureDetector(
                                      onTap: widget.onEdit,
                                      child: Container(
                                        padding: const EdgeInsets.all(4),
                                        decoration: BoxDecoration(
                                          color: Colors.black45,
                                          borderRadius: BorderRadius.circular(
                                            4,
                                          ),
                                          border: Border.all(
                                            color: Colors.white38,
                                          ),
                                        ),
                                        child: const Icon(
                                          Icons.tune,
                                          size: 14,
                                          color: AppColors.primaryNeon,
                                        ),
                                      ),
                                    ),
                                  ),
                                ],
                              ),
                              const SizedBox(height: 4),
                              Text(
                                '${widget.room.physicalWidth.toStringAsFixed(1)}m × ${widget.room.physicalHeight.toStringAsFixed(1)}m (${widget.room.rotation.toInt()}°)',
                                style: TextStyle(
                                  color: Colors.white.withValues(alpha: 0.85),
                                  fontSize: 11,
                                  fontWeight: FontWeight.w600,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
              if (widget.room.hasDoor) _buildDoorHandle(),
              _buildRotateHandle(),
              Positioned(
                top: 12, // 40 margin - 28 = 12
                left: 40,
                child: Wrap(
                  spacing: 4,
                  children: widget.containedSpeakers
                      .map(
                        (s) => Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 6,
                            vertical: 2,
                          ),
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
                        ),
                      )
                      .toList(),
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
        ),
      ),
    );
  }
}

class CadDoorPainter extends CustomPainter {
  final Color color;
  final int wall; // 0: Top, 1: Right, 2: Bottom, 3: Left

  CadDoorPainter({required this.color, required this.wall});

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.0;

    canvas.save();
    canvas.translate(size.width / 2, size.height / 2);
    canvas.rotate(wall * math.pi / 2);
    canvas.translate(-size.width / 2, -size.height / 2);

    final center = Offset(size.width, size.height);
    canvas.drawLine(center, Offset(0, size.height), paint);
    canvas.drawArc(
      Rect.fromCircle(center: center, radius: size.width),
      math.pi,
      math.pi / 2,
      false,
      paint,
    );
    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant CadDoorPainter oldDelegate) {
    return oldDelegate.color != color || oldDelegate.wall != wall;
  }
}
