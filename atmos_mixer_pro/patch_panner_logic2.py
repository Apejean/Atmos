with open('lib/features/dashboard/widgets/object_panner_modal.dart', 'r') as f:
    content = f.read()

# Replace _formatVal
content = content.replace(
"""  String _formatVal(double val) {
    // 0.0 to 1.0 -> -1.0 to +1.0 for display
    final mapped = (val - 0.5) * 2.0;
    return mapped > 0 ? '+${mapped.toStringAsFixed(3)}' : mapped.toStringAsFixed(3);
  }""",
"""  String _formatVal(double val) {
    final mapped = (val - 0.5) * 2.0;
    if (mapped == 0) return '0.000';
    return mapped > 0 ? '+${mapped.toStringAsFixed(3)}' : mapped.toStringAsFixed(3);
  }"""
)

# Replace _formatZ
content = content.replace(
"""  String _formatZ(double val) {
    return '+${val.toStringAsFixed(3)}';
  }""",
"""  String _formatZ(double val) {
    final mapped = (val - 0.5) * 2.0;
    if (mapped == 0) return '0.000';
    return mapped > 0 ? '+${mapped.toStringAsFixed(3)}' : mapped.toStringAsFixed(3);
  }"""
)

# Fix Size & Spread
content = content.replace(
"""_buildReadout('Size', _size.toStringAsFixed(3)),""",
"""_buildReadout('Size', (_size * 100).toInt().toString()),
                  _buildReadout('Spread', '+90°'),"""
)

# Replace Y logic to map Back/Front directly (1.0 - _y -> 1.0 - _y)
content = content.replace(
"""_buildReadout('Back/Front', _formatVal(1.0 - _y)), // Inverse Y for Back/Front (0 is front, 1 is back)""",
"""_buildReadout('Back/Front', _formatVal(1.0 - _y)),"""
)

# Double Tap logic for Top-down map (around line 200)
content = content.replace(
"""                                  onPanDown: (details) {
                                    setState(() {
                                      _x = (details.localPosition.dx / constraints.maxWidth).clamp(0.0, 1.0);
                                      _y = (details.localPosition.dy / constraints.maxHeight).clamp(0.0, 1.0);
                                    });
                                    _updateBackend();
                                  },
                                  child: CustomPaint""",
"""                                  onPanDown: (details) {
                                    setState(() {
                                      _x = (details.localPosition.dx / constraints.maxWidth).clamp(0.0, 1.0);
                                      _y = (details.localPosition.dy / constraints.maxHeight).clamp(0.0, 1.0);
                                    });
                                    _updateBackend();
                                  },
                                  onDoubleTap: () {
                                    setState(() {
                                      _x = 0.5;
                                      _y = 0.0;
                                    });
                                    _updateBackend();
                                  },
                                  child: CustomPaint"""
)

# Double Tap logic for Side-view map
content = content.replace(
"""                              onPanDown: (details) {
                                setState(() {
                                  _z = (1.0 - details.localPosition.dy / constraints.maxHeight).clamp(0.0, 1.0);
                                });
                                _updateBackend();
                              },
                              child: CustomPaint""",
"""                              onPanDown: (details) {
                                setState(() {
                                  _z = (1.0 - details.localPosition.dy / constraints.maxHeight).clamp(0.0, 1.0);
                                });
                                _updateBackend();
                              },
                              onDoubleTap: () {
                                setState(() {
                                  _z = 0.0; // 0.0 is Ear Level in our mapping
                                });
                                _updateBackend();
                              },
                              child: CustomPaint"""
)

with open('lib/features/dashboard/widgets/object_panner_modal.dart', 'w') as f:
    f.write(content)
