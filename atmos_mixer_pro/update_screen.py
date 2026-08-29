import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add _showHeatmap variable
content = content.replace(
    'bool _isRoomSetupOpen = false;',
    'bool _isRoomSetupOpen = false;\n  bool _showHeatmap = false;'
)

# Pass showHeatmap to Dynamic3DRoom
content = content.replace(
    'activeRoom: activeRoom,',
    'activeRoom: activeRoom,\n              showHeatmap: _showHeatmap,'
)

# Update AppBar
app_bar_old = """
        title: const Text(
          'Exhibition Canvas (3D Space)',
          style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold, color: Colors.white),
        ),
"""
app_bar_new = """
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              'Exhibition Canvas (3D Space)',
              style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold, color: Colors.white),
            ),
            const SizedBox(height: 4),
            Row(
              children: [
                _buildAppBarButton(
                  icon: Icons.map_rounded,
                  label: 'SPL Heatmap',
                  isActive: _showHeatmap,
                  onTap: () => setState(() => _showHeatmap = !_showHeatmap),
                ),
                const SizedBox(width: 12),
                _buildAppBarButton(
                  icon: Icons.picture_as_pdf_rounded,
                  label: 'Export PDF Report',
                  onTap: () {},
                ),
                const SizedBox(width: 12),
                _buildAppBarButton(
                  icon: Icons.settings_input_antenna_rounded,
                  label: 'Apply 3D Calibration',
                  onTap: () {},
                ),
              ],
            ),
          ],
        ),
        toolbarHeight: 80,
"""
content = content.replace(app_bar_old, app_bar_new)

# Add _buildAppBarButton helper
button_code = """
  Widget _buildAppBarButton({required IconData icon, required String label, required VoidCallback onTap, bool isActive = false}) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(4),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        decoration: BoxDecoration(
          color: isActive ? Colors.lightBlueAccent.withOpacity(0.2) : Colors.transparent,
          borderRadius: BorderRadius.circular(4),
          border: Border.all(
            color: isActive ? Colors.lightBlueAccent : Colors.white24,
            width: 1,
          ),
        ),
        child: Row(
          children: [
            Icon(icon, size: 14, color: isActive ? Colors.lightBlueAccent : Colors.white70),
            const SizedBox(width: 6),
            Text(
              label,
              style: TextStyle(
                fontSize: 11,
                color: isActive ? Colors.lightBlueAccent : Colors.white70,
                fontWeight: isActive ? FontWeight.bold : FontWeight.normal,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
"""
content = content.replace('}\n\nclass _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {', '}\n\nclass _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {')
content = re.sub(r'}\n$', button_code, content)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
