import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

dialog_code = """
  void _showWorkspaceSettingsDialog() {
    final bp = ref.read(blueprintProvider);
    final wController = TextEditingController(text: bp.canvasWidthMeters.toString());
    final hController = TextEditingController(text: bp.canvasHeightMeters.toString());
    
    showDialog(
      context: context,
      builder: (context) {
        return AlertDialog(
          backgroundColor: AppColors.background,
          title: const Text('Workspace Dimensions (meters)', style: TextStyle(color: Colors.white)),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: wController,
                style: const TextStyle(color: Colors.white),
                decoration: const InputDecoration(labelText: 'Width (m)', labelStyle: TextStyle(color: Colors.white70)),
                keyboardType: TextInputType.number,
              ),
              TextField(
                controller: hController,
                style: const TextStyle(color: Colors.white),
                decoration: const InputDecoration(labelText: 'Depth (m)', labelStyle: TextStyle(color: Colors.white70)),
                keyboardType: TextInputType.number,
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('Cancel', style: TextStyle(color: Colors.white54)),
            ),
            TextButton(
              onPressed: () {
                final w = double.tryParse(wController.text) ?? 40.0;
                final h = double.tryParse(hController.text) ?? 40.0;
                ref.read(blueprintProvider.notifier).setCanvasDimensions(w, h);
                Navigator.of(context).pop();
              },
              child: const Text('Apply', style: TextStyle(color: AppColors.primaryNeon)),
            ),
          ],
        );
      }
    );
  }
"""

content = content.replace("void _addRoom() {", dialog_code + "\n  void _addRoom() {")

# add to popup menu
old_popup = """            } else if (value == 'measure') {"""
new_popup = """            } else if (value == 'workspace') {
              _showWorkspaceSettingsDialog();
            } else if (value == 'measure') {"""
content = content.replace(old_popup, new_popup)

# Add menu item
old_item = """                const PopupMenuItem(
                  value: 'measure',
                  child: Text('📏  눈금자 크기 보정'),
                ),"""
new_item = """                const PopupMenuItem(
                  value: 'workspace',
                  child: Text('📐  공간 크기 설정 (Meters)'),
                ),
                const PopupMenuItem(
                  value: 'measure',
                  child: Text('📏  도면 스케일 보정'),
                ),"""
content = content.replace(old_item, new_item)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
