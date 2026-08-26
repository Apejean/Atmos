import re

path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

# We need to find the end of the build method for _SpeakerCanvasScreenState.
# It ends right before `class _HeatmapPainter extends CustomPainter {`
# Or `class SpeakerInspectorPanel extends ConsumerStatefulWidget {`
# Let's find the closing brace of `_SpeakerCanvasScreenState`.
parts = content.split("class _HeatmapPainter")
if len(parts) > 1:
    state_class_content = parts[0]
    # We replaced `child: Scaffold(` with `child: DefaultTabController(... Builder(... return Scaffold(`
    # So we need to find `    ); // End of GestureDetector` or similar at the end of `build()`
    # Let's search backwards in state_class_content for the last `    );` before `  }`
    
    # Actually, it's easier to just do it manually with sed or a known string replacement.
    last_lines = state_class_content[-200:]
    # print(last_lines) to debug
