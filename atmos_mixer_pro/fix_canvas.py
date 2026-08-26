import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Instead of passing down, let's replace `_canvasWidth` with `_getCanvasWidth(ref)`
# and `_canvasHeight` with `_getCanvasHeight(ref)`

helper = """
double _getCanvasWidth(WidgetRef ref) {
  final bp = ref.read(blueprintProvider);
  return bp.canvasWidthMeters * bp.scale;
}

double _getCanvasHeight(WidgetRef ref) {
  final bp = ref.read(blueprintProvider);
  return bp.canvasHeightMeters * bp.scale;
}
"""

content = content.replace("class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {\n  double get _canvasWidth {\n    final bp = ref.read(blueprintProvider);\n    return bp.canvasWidthMeters * bp.scale;\n  }\n\n  double get _canvasHeight {\n    final bp = ref.read(blueprintProvider);\n    return bp.canvasHeightMeters * bp.scale;\n  }\n", helper + "\nclass _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {\n")

content = re.sub(r'\b_canvasWidth\b', '_getCanvasWidth(ref)', content)
content = re.sub(r'\b_canvasHeight\b', '_getCanvasHeight(ref)', content)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

