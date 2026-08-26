import re

with open('lib/features/dashboard/widgets/object_panner_modal.dart', 'r') as f:
    content = f.read()

# Replace _formatVal and _formatZ logic
content = re.sub(r'String _formatVal\(double val\) \{[^\}]*\}', 
"""String _formatVal(double val) {
    final mapped = (val - 0.5) * 2.0;
    if (mapped == 0) return '0.000';
    return mapped > 0 ? '+${mapped.toStringAsFixed(3)}' : mapped.toStringAsFixed(3);
  }""", content)

content = re.sub(r'String _formatZ\(double val\) \{[^\}]*\}', 
"""String _formatZ(double val) {
    final mapped = (val - 0.5) * 2.0;
    if (mapped == 0) return '0.000';
    return mapped > 0 ? '+${mapped.toStringAsFixed(3)}' : mapped.toStringAsFixed(3);
  }""", content)

content = re.sub(r"_buildReadout\('Size', _size.toStringAsFixed\(3\)\),", 
"""_buildReadout('Size', (_size * 100).toInt().toString()),
                  _buildReadout('Spread', '+90°'),""", content)

# Add double tap to reset
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
                                      _y = 0.0; // Y=0 is Front in our mapping (0 is top/front visually? Wait, Top is usually Front)
                                    });
                                    _updateBackend();
                                  },
                                  child: CustomPaint""")

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
                                  _z = 0.5;
                                });
                                _updateBackend();
                              },
                              child: CustomPaint""")

with open('lib/features/dashboard/widgets/object_panner_modal.dart', 'w') as f:
    f.write(content)
