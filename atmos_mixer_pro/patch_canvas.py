import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# remove const double _canvasWidth and _canvasHeight
content = re.sub(r'const double _canvasWidth = [0-9.]+;\n', '', content)
content = re.sub(r'const double _canvasHeight = [0-9.]+;\n', '', content)

# I can't just replace globally because they need to be evaluated per build/function.
# Or I could create getters in _SpeakerCanvasScreenState
# Let's add getters

getters = """  double get _canvasWidth {
    final bp = ref.read(blueprintProvider);
    return bp.canvasWidthMeters * bp.scale;
  }

  double get _canvasHeight {
    final bp = ref.read(blueprintProvider);
    return bp.canvasHeightMeters * bp.scale;
  }
"""

# Insert getters after class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {
content = content.replace("class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {\n", "class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {\n" + getters)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

