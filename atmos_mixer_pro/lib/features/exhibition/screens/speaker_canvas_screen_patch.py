import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add imports
content = re.sub(
    r"(import 'package:flutter/material\.dart';)",
    r"\1\nimport 'package:atmos_mixer_pro/features/exhibition/widgets/hud/room_setup_window.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/widgets/viewport_3d/room_3d_viewport.dart';",
    content
)

# Fix _SpeakerCanvasScreenState
content = re.sub(
    r'(class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> \{)',
    r'\1\n  double _targetSPL = 75.0;\n  bool _showHeatmap = false;\n',
    content
)

# Replace build method
build_pattern = r'  @override\n  Widget build\(BuildContext context\) \{.*'
new_build = r'''  @override
  Widget build(BuildContext context) {
    final blueprint = ref.watch(blueprintProvider);
    final speakers = ref.watch(speakerLayoutProvider);
    final selectedSpeakerId = ref.watch(selectedSpeakerIdProvider);
    final selectedSpeaker = speakers.firstWhere(
      (s) => s.id == selectedSpeakerId, 
      orElse: () => SpeakerNode(id: '', x: 0, y: 0, channel: 0) // Dummy if none selected
    );
    final hasSelection = selectedSpeakerId != null;

    const darkMatte = Color(0xFF121212);
    const neonCyan = Color(0xFF00E5FF);
    const borderColor = Color(0xFF333333);

    return Scaffold(
      backgroundColor: darkMatte,
      body: Column(
        children: [
          // 1. TOP HEADER BAR
          Container(
            height: 56,
            decoration: const BoxDecoration(
              color: Color(0xFF1A1A1A),
              border: Border(bottom: BorderSide(color: borderColor)),
            ),
            child: Row(
              children: [
                IconButton(
                  icon: const Icon(Icons.arrow_back, color: Colors.white70),
                  onPressed: () => Navigator.pop(context),
                ),
                const SizedBox(width: 16),
                const Text('ATMOS SPATIAL CANVAS', style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold, letterSpacing: 1.5)),
                const Spacer(),
                const Text('Target SPL:', style: TextStyle(color: Colors.white54, fontSize: 12)),
                SizedBox(
                  width: 150,
                  child: Slider(
                    value: _targetSPL,
                    min: 50, max: 95,
                    activeColor: neonCyan,
                    onChanged: (v) => setState(() => _targetSPL = v),
                  ),
                ),
                Text('${_targetSPL.toInt()} dBA', style: const TextStyle(color: Colors.white, fontSize: 12, fontFamily: 'monospace')),
                const SizedBox(width: 24),
                const Text('SPL HEATMAP', style: TextStyle(color: Colors.white54, fontSize: 12)),
                Switch(
                  value: _showHeatmap,
                  onChanged: (v) => setState(() => _showHeatmap = v),
                  activeColor: neonCyan,
                ),
                const SizedBox(width: 16),
              ],
            ),
          ),
          
          // MAIN CONTENT AREA
          Expanded(
            child: Row(
              children: [
                // LEFT: DUAL VIEWPORTS (2D Top + 3D Iso)
                Expanded(
                  child: Stack(
                    children: [
                      Column(
                        children: [
                          // Top 2D View
                          Expanded(
                            flex: 1,
                            child: Container(
                              decoration: const BoxDecoration(border: Border(bottom: BorderSide(color: borderColor))),
                              child: Center(
                                child: Text('2D Top View CAD Blueprint (Placeholder for old canvas)', style: TextStyle(color: Colors.white24)),
                              ),
                            ),
                          ),
                          // Bottom 3D View
                          Expanded(
                            flex: 1,
                            child: Room3DViewport(
                              showHeatmap: _showHeatmap,
                              targetSPL: _targetSPL,
                            ),
                          ),
                        ],
                      ),
                      // Floating Room Setup
                      const Positioned(
                        left: 16, bottom: 16,
                        child: RoomSetupWindow(),
                      ),
                    ],
                  ),
                ),
                
                // RIGHT: INSPECTOR PANEL
                if (hasSelection)
                  SpeakerInspectorPanel(selectedSpeaker: selectedSpeaker),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
'''
content = re.sub(build_pattern, new_build, content, flags=re.DOTALL)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
