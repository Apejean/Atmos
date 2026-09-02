import os
import re

# 1. Patch Speaker Inspector Panel for Auto Aim
path1 = 'lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart'
with open(path1, 'r') as f:
    content1 = f.read()

import_line = "import '../../state/blueprint_state.dart';"
if import_line not in content1:
    content1 = content1.replace("import '../../state/speaker_layout_state.dart';", "import '../../state/speaker_layout_state.dart';\n" + import_line)

old_footer = """          // Footer
          Padding(
            padding: const EdgeInsets.all(16),"""
new_footer = """          // Auto-Aim Button
          if (!speaker.isFixed)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: SizedBox(
                width: double.infinity,
                child: OutlinedButton.icon(
                  onPressed: () {
                    final bp = ref.read(blueprintProvider);
                    final currentRoom = ref.read(roomZoneProvider).where((r) => r.id == speaker.roomId).firstOrNull;
                    final roomW = currentRoom?.physicalWidth ?? bp.canvasWidthMeters;
                    final roomD = currentRoom?.physicalHeight ?? bp.canvasHeightMeters;
                    final earLevel = currentRoom?.earLevel ?? 1.2;

                    // Speaker coordinates relative to center (0,0)
                    final spkX = speaker.x - (roomW / 2);
                    final spkZ = speaker.y - (roomD / 2);
                    final spkY = speaker.heightZ;

                    // Target (Mannequin Ear)
                    final tarX = 0.0;
                    final tarY = earLevel;
                    final tarZ = 0.0;

                    // Calculate direction
                    final dx = tarX - spkX;
                    final dy = tarY - spkY;
                    final dz = tarZ - spkZ;

                    // Yaw = atan2(dx, dz)
                    final yawDeg = math.atan2(dx, dz) * 180 / math.pi;
                    
                    // Pitch = atan2(dy, distance_xz)
                    final distXZ = math.sqrt(dx * dx + dz * dz);
                    final pitchDeg = math.atan2(dy, distXZ) * 180 / math.pi;

                    _updateSpeaker(speaker, rot: yawDeg, tilt: pitchDeg);
                  },
                  icon: const Icon(Icons.my_location_rounded, size: 18),
                  label: const Text('Auto-Aim to Listener'),
                  style: OutlinedButton.styleFrom(
                    foregroundColor: const Color(0xFF22C55E),
                    side: const BorderSide(color: Color(0xFF22C55E)),
                    padding: const EdgeInsets.symmetric(vertical: 12),
                  ),
                ),
              ),
            ),

          // Footer
          Padding(
            padding: const EdgeInsets.all(16),"""
content1 = content1.replace(old_footer, new_footer)
with open(path1, 'w') as f:
    f.write(content1)

print("UI patches applied cleanly.")
