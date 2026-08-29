with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# Add showHeatmap
content = content.replace(
    'final RoomZone? activeRoom;',
    'final RoomZone? activeRoom;\n  final bool showHeatmap;'
)
content = content.replace(
    'this.activeRoom,\n  });',
    'this.activeRoom,\n    this.showHeatmap = false,\n  });'
)

# Filter speakers
content = content.replace(
    'final speakers = ref.watch(speakerLayoutProvider);',
    '''final allSpeakers = ref.watch(speakerLayoutProvider);
    final speakers = allSpeakers.where((s) => s.roomId == null || s.roomId == widget.activeRoom?.id).toList();'''
)

# Set roomId on Add Speaker
content = content.replace(
    'id: newId,\n                  x: roomWidth / 2,',
    'id: newId,\n                  roomId: widget.activeRoom?.id,\n                  x: roomWidth / 2,'
)

# Heatmap Overlay class
heatmap_painter = """
class HeatmapPainter extends CustomPainter {
  final List<SpeakerNode> speakers;
  final double roomWidth;
  final double roomDepth;

  HeatmapPainter(this.speakers, this.roomWidth, this.roomDepth);

  @override
  void paint(Canvas canvas, Size size) {
    if (speakers.isEmpty || roomWidth == 0 || roomDepth == 0) return;

    final double scaleX = size.width / roomWidth;
    final double scaleY = size.height / roomDepth;

    for (final spk in speakers) {
      // Very basic isometric projection approximation
      final double projX = (spk.x * scaleX);
      final double projY = (spk.y * scaleY);
      
      final rect = Rect.fromCircle(center: Offset(projX, projY), radius: size.width * 0.4);
      final paint = Paint()
        ..shader = RadialGradient(
          colors: [
            Colors.red.withValues(alpha: 0.6),
            Colors.orange.withValues(alpha: 0.4),
            Colors.green.withValues(alpha: 0.2),
            Colors.blue.withValues(alpha: 0.05),
            Colors.transparent,
          ],
          stops: const [0.0, 0.3, 0.6, 0.8, 1.0],
        ).createShader(rect)
        ..blendMode = BlendMode.screen;
      
      canvas.drawRect(rect, paint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => true;
}
"""

content = content.replace('class _Dynamic3DRoomState', heatmap_painter + '\nclass _Dynamic3DRoomState')

# Add Heatmap layer to Stack
layer = """
          // Heatmap Overlay
          if (widget.showHeatmap)
            Positioned.fill(
              child: IgnorePointer(
                child: CustomPaint(
                  painter: HeatmapPainter(speakers, roomWidth, roomDepth),
                ),
              ),
            ),

          // 2. Top-Left Room & Viewport Info Badge
"""
content = content.replace('// 2. Top-Left Room & Viewport Info Badge', layer)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)

