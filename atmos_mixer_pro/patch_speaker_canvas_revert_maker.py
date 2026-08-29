import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Revert the AppBar and 3D mode additions
appbar_target = """        appBar: AppBar(
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
        ),
        body: _is3DMode
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
            Positioned.fill("""

appbar_replacement = """        appBar: AppBar(
          title: const Text('Exhibition Canvas', style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold)),
          backgroundColor: Colors.black,
        ),
        body: Stack(
          children: [
            Positioned.fill("""

content = content.replace(appbar_target, appbar_replacement)

# Also remove the imports
imports_target = """import 'package:atmos_mixer_pro/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';"""
content = content.replace(imports_target, "import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';")

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

