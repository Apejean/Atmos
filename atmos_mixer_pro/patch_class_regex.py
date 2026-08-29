import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

new_class = """class _Dynamic3DRoomState extends ConsumerState<Dynamic3DRoom> {
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

content = re.sub(r"class _Dynamic3DRoomState extends ConsumerState<Dynamic3DRoom> \{\s*@override\s*Widget build\(BuildContext context\) \{", new_class, content)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
