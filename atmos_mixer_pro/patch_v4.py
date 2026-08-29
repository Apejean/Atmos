with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# remove unused handlers that my regex failed to remove
handlers = """  void _handlePointerSignal(PointerSignalEvent event) {
    if (event is PointerScrollEvent) {
      // Native macOS trackpad & mouse wheel
      final delta = event.scrollDelta.dy;
      setState(() {
        _cameraDistance = (_cameraDistance + delta * 0.01).clamp(1.5, 25.0);
      });
    }
  }

  double _baseDistance = 7.4;

  void _handleScaleStart(ScaleStartDetails details) {
    _baseDistance = _cameraDistance;
  }

  void _handleScaleUpdate(ScaleUpdateDetails details) {
    if (details.scale == 1.0) {
      // Panning (rotate camera)
      setState(() {
        _yaw = (_yaw - details.focalPointDelta.dx * 0.5) % 360;
        _pitch = (_pitch - details.focalPointDelta.dy * 0.5).clamp(5.0, 175.0);
      });
    } else {
      // Pinch to zoom
      setState(() {
        _cameraDistance = (_baseDistance / details.scale).clamp(1.5, 25.0);
      });
    }
  }

  void _resetCamera() {
    setState(() {
      _cameraDistance = 7.4;
      _yaw = 45.0;
      _pitch = 65.0;
    });
  }"""
content = content.replace(handlers, '')
content = content.replace('double _cameraDistance = 7.4;\n  double _yaw = 45.0;\n  double _pitch = 65.0;', '')
content = content.replace("Zoom: ${_cameraDistance.toStringAsFixed(1)}m", "Zoom: Auto")

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
