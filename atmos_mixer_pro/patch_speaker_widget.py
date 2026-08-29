import re

content = """class _DraggableSpeakerWidget extends ConsumerStatefulWidget {
  final SpeakerNode node;
  final Color? roomColor;
  final bool isDuplicate;
  final TransformationController transformationController;
  final VoidCallback? onEdit;
  final Function(String)? onSpeakerSelected;

  const _DraggableSpeakerWidget({
    super.key,
    required this.node,
    this.roomColor,
    required this.isDuplicate,
    required this.transformationController,
    this.onEdit,
    this.onSpeakerSelected,
  });

  @override
  ConsumerState<_DraggableSpeakerWidget> createState() =>
      _DraggableSpeakerWidgetState();
}

class _DraggableSpeakerWidgetState
    extends ConsumerState<_DraggableSpeakerWidget> {
  late double _localX;
  late double _localY;
  bool _isDragging = false;
  double _initialTouchAngle = 0.0;
  double _initialSpeakerRotation = 0.0;

  @override
  void initState() {
    super.initState();
    _localX = widget.node.x;
    _localY = widget.node.y;
  }

  @override
  void didUpdateWidget(covariant _DraggableSpeakerWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!_isDragging) {
      _localX = widget.node.x;
      _localY = widget.node.y;
    }
  }

  @override
  Widget build(BuildContext context) {
    const originOffset = Offset(_speakerSize / 2, _speakerSize / 2);
    final String label = 'S${(widget.node.channel + 1).toString().padLeft(2, '0')}';
    
    return Positioned(
      left: _localX,
      top: _localY,
      child: Transform.rotate(
        angle: widget.node.rotation * math.pi / 180.0,
        alignment: Alignment.topLeft,
        origin: originOffset,
        child: SizedBox(
          width: 120,
          height: 120,
          child: Stack(
            clipBehavior: Clip.none,
            alignment: Alignment.center,
            children: [
              // Wave rings behind speaker
              CustomPaint(
                size: const Size(120, 120),
                painter: _SpeakerWavesPainter(),
              ),
              
              // Direction Arrow
              Positioned(
                top: 0,
                child: Icon(Icons.arrow_upward, color: Colors.lightBlueAccent, size: 24),
              ),
              
              // Draggable Speaker Box Body
              Positioned.fill(
                child: GestureDetector(
                  onTap: () {
                    if (widget.onSpeakerSelected != null) {
                      widget.onSpeakerSelected!(widget.node.id);
                    }
                  },
                  onPanStart: (_) => setState(() => _isDragging = true),
                  onPanUpdate: (details) {
                    final scale = widget.transformationController.value
                        .getMaxScaleOnAxis();
                    final currentScale = scale > 0 ? scale : 1.0;
                    final rad = widget.node.rotation * math.pi / 180.0;
                    final localDx = details.delta.dx / currentScale;
                    final localDy = details.delta.dy / currentScale;

                    final globalDx =
                        localDx * math.cos(rad) - localDy * math.sin(rad);
                    final globalDy =
                        localDx * math.sin(rad) + localDy * math.cos(rad);

                    setState(() {
                      _localX = (_localX + globalDx).clamp(
                        0.0,
                        _getCanvasWidth(ref) - _speakerSize,
                      );
                      _localY = (_localY + globalDy).clamp(
                        0.0,
                        _getCanvasHeight(ref) - _speakerSize,
                      );
                    });
                    ref
                        .read(speakerLayoutProvider.notifier)
                        .updateSpeaker(
                          widget.node.copyWith(x: _localX, y: _localY),
                          immediate: false,
                        );
                  },
                  onPanEnd: (details) {
                    final snappedX =
                        (_localX / ref.read(blueprintProvider).scale).round() *
                        ref.read(blueprintProvider).scale;
                    final snappedY =
                        (_localY / ref.read(blueprintProvider).scale).round() *
                        ref.read(blueprintProvider).scale;
                    ref
                        .read(speakerLayoutProvider.notifier)
                        .updateSpeaker(
                          widget.node.copyWith(x: snappedX, y: snappedY),
                        );
                    setState(() {
                      _isDragging = false;
                      _localX = snappedX;
                      _localY = snappedY;
                    });
                  },
                  child: Container(
                    width: _speakerSize,
                    height: _speakerSize,
                    margin: const EdgeInsets.all(30), // Shrink actual touch target to center
                    decoration: BoxDecoration(
                      color: const Color(0xFF1B232D),
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(
                        color: Colors.lightBlueAccent,
                        width: 2,
                      ),
                      boxShadow: [
                        BoxShadow(
                          color: Colors.lightBlueAccent.withValues(alpha: 0.3),
                          blurRadius: 10,
                          spreadRadius: 2,
                        ),
                      ],
                    ),
                    child: Center(
                      child: Container(
                        width: 30,
                        height: 30,
                        decoration: BoxDecoration(
                          shape: BoxShape.circle,
                          border: Border.all(color: Colors.lightBlueAccent, width: 2),
                        ),
                        child: Center(
                          child: Container(
                            width: 10,
                            height: 10,
                            decoration: const BoxDecoration(
                              shape: BoxShape.circle,
                              color: Colors.lightBlueAccent,
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
              
              // Label
              Positioned(
                right: 0,
                top: 20,
                child: Row(
                  children: [
                    Text(label, style: const TextStyle(color: Colors.white, fontSize: 12, fontWeight: FontWeight.bold)),
                    const SizedBox(width: 4),
                    Container(width: 4, height: 4, decoration: const BoxDecoration(shape: BoxShape.circle, color: Colors.lightBlueAccent)),
                  ],
                ),
              ),
              
              // Rotation Handle (Optional, keep it simple for now)
            ],
          ),
        ),
      ),
    );
  }
}

class _SpeakerWavesPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.lightBlueAccent.withValues(alpha: 0.8)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.0
      ..strokeCap = StrokeCap.round;
      
    final center = Offset(size.width / 2, size.height / 2);
    
    // Draw 3 arcs on each side
    for (int i = 0; i < 3; i++) {
      final radius = 35.0 + (i * 10.0);
      
      // Left arcs
      canvas.drawArc(
        Rect.fromCircle(center: center, radius: radius),
        math.pi * 0.75,
        math.pi * 0.5,
        false,
        paint,
      );
      
      // Right arcs
      canvas.drawArc(
        Rect.fromCircle(center: center, radius: radius),
        -math.pi * 0.25,
        math.pi * 0.5,
        false,
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
"""

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    file_content = f.read()

# regex replace
import re
pattern = r"class _DraggableSpeakerWidget extends ConsumerStatefulWidget \{.*?\n\}\nclass _GridPainter"
# This might be tricky because we need to replace up to _GridPainter
pattern = r"class _DraggableSpeakerWidget extends ConsumerStatefulWidget \{.*?(?=class _GridPainter)"
file_content = re.sub(pattern, content + "\n\n", file_content, flags=re.DOTALL)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(file_content)

