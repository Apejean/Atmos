def fix_room():
    with open('lib/features/exhibition/widgets/room_zone_widget.dart', 'r') as f:
        c = f.read()

    # 1. Properties
    c = c.replace('final VoidCallback? onDragUpdate;', 'final VoidCallback? onDragUpdate;\n  final bool isSelected;\n  final VoidCallback? onInteractionStart;\n  final VoidCallback? onInteractionEnd;')
    c = c.replace('this.onDragUpdate,', 'this.onDragUpdate,\n    this.isSelected = false,\n    this.onInteractionStart,\n    this.onInteractionEnd,')

    # 2. _isRotatingFromCorner removal
    c = c.replace('bool _isRotatingFromCorner = false;', '')
    
    # 3. _buildResizeHandle fix
    c = c.replace('''              if (isCorner) {
                // Center of the 40x40 box is (20,20)
                final dist = math.sqrt(
                  math.pow(details.localPosition.dx - 20, 2) +
                      math.pow(details.localPosition.dy - 20, 2),
                );
                _isRotatingFromCorner = dist > 10.0; // Outer part -> Rotate
              } else {
                _isRotatingFromCorner = false;
              }

              if (_isRotatingFromCorner) {
                final renderBox = context.findRenderObject() as RenderBox?;
                if (renderBox != null) {
                  final localTouch = renderBox.globalToLocal(
                    details.globalPosition,
                  );
                  final roomCenterLocal = Offset(_localW / 2, _localH / 2);
                  _initialAngleToCenter = math.atan2(
                    localTouch.dy - roomCenterLocal.dy,
                    localTouch.dx - roomCenterLocal.dx,
                  );
                  _initialRotation = widget.room.rotation;
                }
              }''', 'widget.onInteractionStart?.call();')
              
    c = c.replace('''            if (_isRotatingFromCorner) {
              final renderBox = context.findRenderObject() as RenderBox?;
              if (renderBox != null) {
                final localTouch = renderBox.globalToLocal(
                  details.globalPosition,
                );
                final roomCenterLocal = Offset(_localW / 2, _localH / 2);
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
                      immediate: true,
                    );
              }
              final scale = widget.transformationController.value
                  .getMaxScaleOnAxis();
              final currentScale = scale > 0 ? scale : 1.0;
              final dxGlobal = details.delta.dx / currentScale;
              final dyGlobal = details.delta.dy / currentScale;

              setState(() {
                _localX += dxGlobal;
                _localY += dyGlobal;

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
                      immediate: true,
                    );
              });
              widget.onDragUpdate?.call();
              return;
            }''', '')
            
    c = c.replace('''            if (_isRotatingFromCorner) {
              setState(() => _isInteracting = false);
              return;
            }''', '')

    c = c.replace('''            setState(() {
              _localX = snappedX;
              _localY = snappedY;
              _localW = snappedW;
              _localH = snappedH;
              _isInteracting = false;
            });''', '''            setState(() {
              _localX = snappedX;
              _localY = snappedY;
              _localW = snappedW;
              _localH = snappedH;
              _isInteracting = false;
            });
            widget.onInteractionEnd?.call();''')

    # 4. Rotate handle globalToLocal bug (use context instead of roomContext)
    c = c.replace('final renderBox = roomContext.findRenderObject() as RenderBox?;', 'final renderBox = context.findRenderObject() as RenderBox?;')

    # 5. Hit Test Clipping & Visual selection & main drag interaction
    c = c.replace('final roomColor = Color(widget.room.color);', 'final roomColor = widget.isSelected ? Colors.blueAccent.withValues(alpha: 0.3) : Color(widget.room.color);')
    c = c.replace('''    return Positioned(
      left: _localX,
      top: _localY,
      width: _localW,
      height: _localH,
      child: Builder(
        builder: (roomContext) {
          return Transform.rotate(
            angle: widget.room.rotation * math.pi / 180.0,
            child: Stack(
              clipBehavior: Clip.none,
              children: [''', '''    return Positioned(
      left: _localX - 40,
      top: _localY - 40,
      width: _localW + 80,
      height: _localH + 80,
      child: Builder(
        builder: (context) {
          return Stack(
            clipBehavior: Clip.none,
            children: [
              Positioned(
                left: 40,
                top: 40,
                width: _localW,
                height: _localH,
                child: Transform.rotate(
                  angle: widget.room.rotation * math.pi / 180.0,
                  child: Stack(
                    clipBehavior: Clip.none,
                    children: [''')
                    
    c = c.replace('''                _buildResizeHandle(Alignment.bottomCenter),
                _buildResizeHandle(Alignment.bottomRight),
              ],
            ),
          );
        },
      ),
    );''', '''                _buildResizeHandle(Alignment.bottomCenter),
                _buildResizeHandle(Alignment.bottomRight),
                    ],
                  ),
                ),
              ],
            ),
          );
        },
      ),
    );''')

    c = c.replace('''                      setState(() => _isInteracting = true);
                      _draggedSpeakersOffsets = {''', '''                      setState(() => _isInteracting = true);
                      widget.onInteractionStart?.call();
                      _draggedSpeakersOffsets = {''')

    c = c.replace('''                      setState(() {
                        _localX = snappedX;
                        _localY = snappedY;
                        _isInteracting = false;
                      });''', '''                      setState(() {
                        _localX = snappedX;
                        _localY = snappedY;
                        _isInteracting = false;
                      });
                      widget.onInteractionEnd?.call();''')

    c = c.replace('onPanEnd: (_) => setState(() => _isInteracting = false),', 'onPanEnd: (_) {\n            setState(() => _isInteracting = false);\n            widget.onInteractionEnd?.call();\n          },')
    
    # Door interaction end
    c = c.replace('onPanStart: (_) => setState(() => _isInteracting = true),', 'onPanStart: (_) {\n          setState(() => _isInteracting = true);\n          widget.onInteractionStart?.call();\n        },')
    c = c.replace('onPanEnd: (_) => setState(() => _isInteracting = false),', 'onPanEnd: (_) {\n          setState(() => _isInteracting = false);\n          widget.onInteractionEnd?.call();\n        },')

    with open('lib/features/exhibition/widgets/room_zone_widget.dart', 'w') as f:
        f.write(c)

def fix_screen():
    with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
        c = f.read()

    c = c.replace("String _selectedOctaveFilter =\n      'All';", "String _selectedOctaveFilter =\n      'All';\n  String? _selectedRoomId;\n  bool _isRoomInteracting = false;")

    c = c.replace('void _editRoom(RoomZone room) async {', 'void _editRoom(RoomZone room) async {\n    setState(() => _selectedRoomId = room.id);')

    c = c.replace('''                IconButton(
                  icon: const Icon(Icons.close, color: Colors.white54),
                  onPressed: () => Navigator.pop(context),
                ),''', '''                IconButton(
                  icon: const Icon(Icons.close, color: Colors.white54),
                  onPressed: () {
                    setState(() => _selectedRoomId = null);
                    Navigator.pop(context);
                  },
                ),''')

    c = c.replace('''                  ref.read(roomZoneProvider.notifier).removeRoomZone(room.id);
                  Navigator.pop(context);''', '''                  ref.read(roomZoneProvider.notifier).removeRoomZone(room.id);
                  setState(() => _selectedRoomId = null);
                  Navigator.pop(context);''')

    c = c.replace('''              TextButton(
                onPressed: () => Navigator.pop(context),''', '''              TextButton(
                onPressed: () {
                  setState(() => _selectedRoomId = null);
                  Navigator.pop(context);
                },''')

    c = c.replace('''                  _syncSpatialConfigRealtime();
                  Navigator.pop(context);''', '''                  _syncSpatialConfigRealtime();
                  setState(() => _selectedRoomId = null);
                  Navigator.pop(context);''')

    c = c.replace('''    return GestureDetector(
      onTap: () => _canvasFocusNode.requestFocus(),''', '''    return GestureDetector(
      onTap: () {
        _canvasFocusNode.requestFocus();
        if (!_isMeasuringScale) setState(() => _selectedRoomId = null);
      },''')

    c = c.replace('''                  return InteractiveViewer(
                    transformationController: _transformationController,
                    panEnabled: !_isMeasuringScale,
                    scaleEnabled: !_isMeasuringScale,''', '''                  return GestureDetector(
                    onTap: () {
                      if (!_isMeasuringScale) setState(() => _selectedRoomId = null);
                    },
                    child: InteractiveViewer(
                    transformationController: _transformationController,
                    panEnabled: !_isMeasuringScale && !_isRoomInteracting,
                    scaleEnabled: !_isMeasuringScale,''')

    # Replace the RepaintBoundary's closing Stack properly for Gesture detector
    # Wait, the InteractiveViewer wraps the whole canvas. So wrapping InteractiveViewer with GestureDetector is enough.
    # Since we opened a GestureDetector, we must close it. InteractiveViewer was just returned.
    c = c.replace('''                          if (_isMeasuringScale)
                            Positioned.fill(
                              child: Stack(''','''                          if (_isMeasuringScale)
                            Positioned.fill(
                              child: Stack(''') # no change here
    c = c.replace('''                                  ],
                                ),
                              ),
                            ),
                        ],
                      ),
                    ),
                  );
                },
              ),''', '''                                  ],
                                ),
                              ),
                            ),
                        ],
                      ),
                      ),
                    ),
                  );
                },
              ),''') # added closing for GestureDetector

    c = c.replace('''                                      return RoomZoneWidget(
                                        key: ValueKey(room.id),
                                        room: room,
                                        containedSpeakers: containedSpeakers,
                                        transformationController:
                                            _transformationController,
                                        onEdit: () => _editRoom(room),
                                        onDragUpdate: _syncSpatialConfigRealtime,
                                      );''', '''                                      return RoomZoneWidget(
                                        key: ValueKey(room.id),
                                        room: room,
                                        containedSpeakers: containedSpeakers,
                                        transformationController:
                                            _transformationController,
                                        isSelected: _selectedRoomId == room.id,
                                        onEdit: () => _editRoom(room),
                                        onDragUpdate: _syncSpatialConfigRealtime,
                                        onInteractionStart: () => setState(() {
                                          _isRoomInteracting = true;
                                          _selectedRoomId = room.id;
                                        }),
                                        onInteractionEnd: () => setState(() => _isRoomInteracting = false),
                                      );''')

    with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
        f.write(c)

if __name__ == '__main__':
    fix_room()
    fix_screen()
