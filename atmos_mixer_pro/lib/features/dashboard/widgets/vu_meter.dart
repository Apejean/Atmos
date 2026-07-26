import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';

class VUMeterPainter extends CustomPainter {
  final ValueNotifier<double> levelNotifier; // 0.0 to 1.0

  VUMeterPainter(this.levelNotifier) : super(repaint: levelNotifier);

  @override
  void paint(Canvas canvas, Size size) {
    double level = levelNotifier.value;
    final paint = Paint()..style = PaintingStyle.fill;

    // Draw background segments
    int totalSegments = 20;
    double segmentHeight =
        (size.height - (totalSegments - 1) * 2) / totalSegments;

    for (int i = 0; i < totalSegments; i++) {
      double y = size.height - (i * (segmentHeight + 2)) - segmentHeight;
      double threshold = (i + 1) / totalSegments;

      Color color;
      if (threshold > 0.85) {
        color = Colors.red;
      } else if (threshold > 0.6) {
        color = Colors.yellow;
      } else {
        color = AppColors.primaryNeon;
      }

      // Draw dimmed background
      paint.color = color.withOpacity(0.2);
      canvas.drawRect(Rect.fromLTWH(0, y, size.width, segmentHeight), paint);

      // Draw active foreground if level > threshold
      if (level >= (i / totalSegments)) {
        paint.color = color;
        canvas.drawRect(Rect.fromLTWH(0, y, size.width, segmentHeight), paint);
      }
    }
  }

  @override
  bool shouldRepaint(VUMeterPainter oldDelegate) {
    return oldDelegate.levelNotifier != levelNotifier;
  }
}

class NeonVUMeter extends ConsumerStatefulWidget {
  final int outputChannel;

  const NeonVUMeter({super.key, required this.outputChannel});

  @override
  ConsumerState<NeonVUMeter> createState() => _NeonVUMeterState();
}

class _NeonVUMeterState extends ConsumerState<NeonVUMeter> {
  final ValueNotifier<double> _levelNotifier = ValueNotifier<double>(0.0);

  @override
  void dispose() {
    _levelNotifier.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    ref.listen(vuStreamProvider, (previous, next) {
      final levels = next.value;
      if (levels != null && widget.outputChannel >= 0 && widget.outputChannel < levels.length) {
        final newLevel = levels[widget.outputChannel];
        double currentLevel = _levelNotifier.value;
        if (newLevel > currentLevel) {
          currentLevel = newLevel;
        } else {
          currentLevel -= 0.05; // Decay rate
          if (currentLevel < 0) currentLevel = 0;
        }
        _levelNotifier.value = currentLevel;
      }
    });
    return SizedBox(
      width: 12,
      height: double.infinity,
      child: RepaintBoundary(
        child: CustomPaint(painter: VUMeterPainter(_levelNotifier)),
      ),
    );
  }
}
