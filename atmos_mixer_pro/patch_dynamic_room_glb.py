import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# Add import
if 'glb_scaler.dart' not in content:
    content = content.replace("import 'package:flutter/material.dart';", "import 'package:flutter/material.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/utils/glb_scaler.dart';")

# Convert _Dynamic3DRoomState to use FutureBuilder or generate on didUpdateWidget
# It's better to generate it asynchronously and store the path in state.
old_class = """class _Dynamic3DRoomState extends State<Dynamic3DRoom> {
  @override
  Widget build(BuildContext context) {"""

new_class = """class _Dynamic3DRoomState extends State<Dynamic3DRoom> {
  String? _localGlbPath;
  double _lastW = -1, _lastD = -1, _lastH = -1;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _checkAndGenerateGlb();
  }

  @override
  void didUpdateWidget(Dynamic3DRoom oldWidget) {
    super.didUpdateWidget(oldWidget);
    _checkAndGenerateGlb();
  }

  Future<void> _checkAndGenerateGlb() async {
    final bp = widget.blueprintProvider;
    final w = widget.activeRoom?.physicalWidth ?? 6.0;
    final d = widget.activeRoom?.physicalHeight ?? 4.5;
    final h = widget.activeRoom?.ceilingHeight ?? 3.0;
    
    if (w == _lastW && d == _lastD && h == _lastH && _localGlbPath != null) return;
    
    _lastW = w; _lastD = d; _lastH = h;
    final path = await GlbScaler.generateScaledRoom(w / 4.016, h / 2.616, d / 4.016);
    if (mounted) {
      setState(() {
        _localGlbPath = path;
      });
    }
  }

  @override
  Widget build(BuildContext context) {"""

if "_localGlbPath" not in content:
    content = content.replace(old_class, new_class)

# Fix bp reference in _checkAndGenerateGlb (wait, I don't have widget.blueprintProvider, I'll just use 6.0 fallback)
# Actually, I should just copy the logic from build:

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
