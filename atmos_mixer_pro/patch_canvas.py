import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add import
import_str = """
import 'package:atmos_mixer_pro/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';
"""
content = content.replace("import 'package:flutter_riverpod/flutter_riverpod.dart';", import_str + "\nimport 'package:flutter_riverpod/flutter_riverpod.dart';")

# Add state variable
state_var = """  bool _isMeasuringScale = false;
  bool _is3DMode = false;
  String? _selectedInspectorSpeakerId;"""
content = content.replace("  bool _isMeasuringScale = false;", state_var)

# Add toggle to AppBar
appbar_find = """        appBar: AppBar(
          leading: IconButton(
            icon: const Icon(Icons.arrow_back, color: Colors.white),
            onPressed: () => Navigator.of(context).pop(),
          ),
          title: const Text('Exhibition Canvas', style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold, color: Colors.white)),
          backgroundColor: Colors.black,
        ),"""

appbar_replace = """        appBar: AppBar(
          leading: IconButton(
            icon: const Icon(Icons.arrow_back, color: Colors.white),
            onPressed: () => Navigator.of(context).pop(),
          ),
          title: const Text('Exhibition Canvas', style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold, color: Colors.white)),
          backgroundColor: Colors.black,
          actions: [
            Row(
              children: [
                const Text('3D Mode', style: TextStyle(color: Colors.white70)),
                Switch(
                  value: _is3DMode,
                  onChanged: (v) => setState(() {
                     _is3DMode = v;
                     if (!v) _selectedInspectorSpeakerId = null;
                  }),
                  activeColor: Colors.lightBlueAccent,
                ),
              ],
            ),
          ],
        ),"""
content = content.replace(appbar_find, appbar_replace)

# Modify body to render 3D Mode
body_find = """        body: Stack(
          children: [
            Positioned.fill(
              child: LayoutBuilder("""

body_replace = """        body: _is3DMode
            ? Stack(
                children: [
                  Positioned.fill(
                    child: Dynamic3DRoom(
                      onSpeakerTapped: (id) {
                        setState(() {
                          _selectedInspectorSpeakerId = id;
                        });
                      }
                    ),
                  ),
                  if (_selectedInspectorSpeakerId != null)
                    Positioned(
                      top: 0,
                      bottom: 0,
                      right: 0,
                      child: SpeakerInspectorPanel(
                        speakerId: _selectedInspectorSpeakerId!,
                        onClose: () => setState(() => _selectedInspectorSpeakerId = null),
                      ),
                    ),
                ],
              )
            : Stack(
          children: [
            Positioned.fill(
              child: LayoutBuilder("""
content = content.replace(body_find, body_replace)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
print("Patched successfully")
