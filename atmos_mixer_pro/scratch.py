import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Replace `_gridSize` with `ref.read(blueprintProvider).scale` globally
# Wait, for builds we could use watch, but `ref.read` works everywhere in Riverpod if we just need it for math inside callbacks. Inside `build` or custom painters, maybe it doesn't rebuild?
# The `blueprintProvider` change triggers `ref.watch(blueprintProvider)` at the top of `build` in `SpeakerCanvasScreen`, `_DraggableRoomWidget`, etc.
# So they will rebuild! `ref.read(blueprintProvider).scale` inside the `build` method is technically not reactive on its own, but since the parent widgets already watch `blueprintProvider`, it's fine! 
# But to be safe, I'll use `ref.read(blueprintProvider).scale` for logic and snap variables.
content = re.sub(r'\b_gridSize\b', 'ref.read(blueprintProvider).scale', content)
content = re.sub(r'const double ref\.read\(blueprintProvider\)\.scale = 50\.0;', '', content)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
