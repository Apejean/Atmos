import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Force the body to ONLY be Dynamic3DRoom, completely remove the 2D Stack and `_is3DMode`
# Find the AppBar and body section
pattern = r"appBar: AppBar\(.*?body: _is3DMode.*? \: Stack\(\n          children: \[\n            Positioned\.fill\("

replacement = """appBar: AppBar(
          leading: IconButton(
            icon: const Icon(Icons.arrow_back, color: Colors.white),
            onPressed: () => Navigator.of(context).pop(),
          ),
          title: const Text('Exhibition Canvas (3D Space)', style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold, color: Colors.white)),
          backgroundColor: Colors.black,
        ),
        body: Stack(
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
            if (_isRoomSetupOpen)
              Positioned(
                left: 16,
                bottom: 16,
                child: RoomSetupWindow(
                  onClose: () => setState(() => _isRoomSetupOpen = false),
                ),
              ),
            if (!_isRoomSetupOpen)
              Positioned(
                left: 16,
                bottom: 16,
                child: OutlinedButton.icon(
                  icon: const Icon(Icons.picture_in_picture_alt_outlined, size: 16),
                  label: const Text('ROOM SETUP'),
                  style: OutlinedButton.styleFrom(
                    foregroundColor: Colors.lightBlueAccent,
                    backgroundColor: const Color(0xFF2C394B).withValues(alpha: 0.8),
                    side: const BorderSide(color: Colors.lightBlueAccent),
                    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                  ),
                  onPressed: () => setState(() => _isRoomSetupOpen = true),
                ),
              ),
          ],
        ),
      ),
    );
  }
}"""

# The above replacement will replace everything down to Positioned.fill(. We need to drop the rest of the file which is the 2D stuff!
# Instead of regex, let's just build a clean SpeakerCanvasScreen.

clean_screen = """import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/room_setup_window.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';

class SpeakerCanvasScreen extends ConsumerStatefulWidget {
  const SpeakerCanvasScreen({super.key});

  @override
  ConsumerState<SpeakerCanvasScreen> createState() => _SpeakerCanvasScreenState();
}

class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {
  String? _selectedInspectorSpeakerId;
  bool _isRoomSetupOpen = false;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: AppColors.background,
      appBar: AppBar(
        leading: IconButton(
          icon: const Icon(Icons.arrow_back, color: Colors.white),
          onPressed: () => Navigator.of(context).pop(),
        ),
        title: const Text('Exhibition Canvas (3D Space)', style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold, color: Colors.white)),
        backgroundColor: Colors.black,
      ),
      body: Stack(
        children: [
          // 100% 3D Native Space
          Positioned.fill(
            child: Dynamic3DRoom(
              onSpeakerTapped: (id) {
                setState(() {
                  _selectedInspectorSpeakerId = id;
                });
              }
            ),
          ),
          
          // Inspector Panel (Glassmorphism right side)
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
            
          // Room Setup Panel (Bottom left)
          if (_isRoomSetupOpen)
            Positioned(
              left: 16,
              bottom: 16,
              child: RoomSetupWindow(
                onClose: () => setState(() => _isRoomSetupOpen = false),
              ),
            ),
          if (!_isRoomSetupOpen)
            Positioned(
              left: 16,
              bottom: 16,
              child: OutlinedButton.icon(
                icon: const Icon(Icons.picture_in_picture_alt_outlined, size: 16),
                label: const Text('ROOM SETUP'),
                style: OutlinedButton.styleFrom(
                  foregroundColor: Colors.lightBlueAccent,
                  backgroundColor: const Color(0xFF2C394B).withValues(alpha: 0.8),
                  side: const BorderSide(color: Colors.lightBlueAccent),
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                ),
                onPressed: () => setState(() => _isRoomSetupOpen = true),
              ),
            ),
        ],
      ),
    );
  }
}
"""

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(clean_screen)

