with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

zoom_logic = """
  double _cameraDistance = 6.5;
  String _cameraAngle = '45deg 65deg';
  bool _isTopView = false;

  void _zoomIn() {
    setState(() {
      _cameraDistance = (_cameraDistance - 1.0).clamp(2.0, 15.0);
    });
  }

  void _zoomOut() {
    setState(() {
      _cameraDistance = (_cameraDistance + 1.0).clamp(2.0, 15.0);
    });
  }

  void _resetCamera() {
    setState(() {
      _cameraDistance = 6.5;
      _cameraAngle = '45deg 65deg';
      _isTopView = false;
    });
  }

  void _toggleTopView() {
    setState(() {
      _isTopView = !_isTopView;
      _cameraAngle = _isTopView ? '0deg 5deg' : '45deg 65deg';
    });
  }

  @override
"""

content = content.replace('  @override\n  Widget build(BuildContext context) {', zoom_logic + '  Widget build(BuildContext context) {')

content = content.replace("cameraOrbit: '45deg 65deg 6.5m',", "cameraOrbit: '$_cameraAngle ${_cameraDistance.toStringAsFixed(1)}m',")

zoom_ui = """
          // Right-Side Zoom & Camera Navigation Control Pod
          Positioned(
            top: 68,
            right: widget.selectedSpeakerId != null ? 360 : 16,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 6),
              decoration: BoxDecoration(
                color: const Color(0xFF161E28).withOpacity(0.90),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.white.withOpacity(0.12)),
                boxShadow: [
                  BoxShadow(
                    color: Colors.black.withOpacity(0.4),
                    blurRadius: 10,
                    offset: const Offset(0, 4),
                  ),
                ],
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  // Zoom In Button (+)
                  _buildNavIconButton(
                    icon: Icons.add_rounded,
                    tooltip: 'Zoom In',
                    onTap: _zoomIn,
                  ),
                  const SizedBox(height: 4),

                  // Current Distance
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                    child: Text(
                      '${_cameraDistance.toStringAsFixed(1)}m',
                      style: const TextStyle(
                        color: Colors.white60,
                        fontSize: 10,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ),
                  const SizedBox(height: 4),

                  // Zoom Out Button (-)
                  _buildNavIconButton(
                    icon: Icons.remove_rounded,
                    tooltip: 'Zoom Out',
                    onTap: _zoomOut,
                  ),

                  const Divider(color: Colors.white12, height: 12, indent: 4, endIndent: 4),

                  // Reset Camera View (⟲)
                  _buildNavIconButton(
                    icon: Icons.restart_alt_rounded,
                    tooltip: 'Reset View',
                    onTap: _resetCamera,
                  ),
                  const SizedBox(height: 4),

                  // Top-Down / 3D Toggle
                  _buildNavIconButton(
                    icon: _isTopView ? Icons.view_in_ar_rounded : Icons.grid_view_rounded,
                    tooltip: _isTopView ? 'Switch to 3D Orbit' : 'Switch to Top View',
                    color: _isTopView ? Colors.lightBlueAccent : Colors.white70,
                    onTap: _toggleTopView,
                  ),
                ],
              ),
            ),
          ),

          // 3. Floating Action Button: Add Speaker (Bottom Right)
"""

content = content.replace('          // 3. Floating Action Button: Add Speaker (Bottom Right)', zoom_ui)


build_nav = """
  Widget _buildNavIconButton({
    required IconData icon,
    required String tooltip,
    required VoidCallback onTap,
    Color color = Colors.white70,
  }) {
    return Tooltip(
      message: tooltip,
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(6),
          child: Container(
            width: 34,
            height: 34,
            alignment: Alignment.center,
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(6),
              border: Border.all(color: Colors.white.withOpacity(0.08)),
            ),
            child: Icon(icon, color: color, size: 18),
          ),
        ),
      ),
    );
  }
}
"""

content = content.replace('}\n', build_nav)
content = content.replace('withOpacity', 'withValues(alpha: ')
content = content.replace('0.90)', '0.90))')
content = content.replace('0.12)', '0.12))')
content = content.replace('0.4)', '0.4))')
content = content.replace('0.08)', '0.08))')

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
