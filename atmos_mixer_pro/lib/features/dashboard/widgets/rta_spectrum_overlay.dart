import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';

/// Real-time 24-Channel RTA (Real-Time Analyzer) Frequency Spectrum Overlay Widget
/// Optimized with [RepaintBoundary] for 60fps high performance rendering.
class RtaSpectrumOverlayWidget extends StatefulWidget {
  final bool isOverlayMode;
  final VoidCallback? onClose;

  const RtaSpectrumOverlayWidget({
    super.key,
    this.isOverlayMode = false,
    this.onClose,
  });

  @override
  State<RtaSpectrumOverlayWidget> createState() =>
      _RtaSpectrumOverlayWidgetState();
}

class _RtaSpectrumOverlayWidgetState extends State<RtaSpectrumOverlayWidget>
    with SingleTickerProviderStateMixin {
  late AnimationController _animationController;
  final List<List<double>> _channelBands = List.generate(
    24,
    (_) => List.generate(31, (_) => 0.0),
  );
  final math.Random _random = math.Random();

  @override
  void initState() {
    super.initState();
    _animationController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1000),
    )..repeat();

    _animationController.addListener(_updateSpectrumData);
  }

  void _updateSpectrumData() {
    if (!mounted) return;
    setState(() {
      final double time = DateTime.now().millisecondsSinceEpoch / 200.0;
      for (int ch = 0; ch < 24; ch++) {
        for (int b = 0; ch < 24 && b < 31; b++) {
          final double base = math.sin(time + ch * 0.3 + b * 0.2).abs();
          final double noise = _random.nextDouble() * 0.15;
          _channelBands[ch][b] = (base * 0.7 + noise).clamp(0.05, 1.0);
        }
      }
    });
  }

  @override
  void dispose() {
    _animationController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: AppColors.cardSurface.withValues(alpha: widget.isOverlayMode ? 0.85 : 1.0),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: AppColors.primaryNeon.withValues(alpha: 0.4)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Row(
                children: [
                  const Icon(Icons.graphic_eq, color: AppColors.primaryNeon, size: 18),
                  const SizedBox(width: 8),
                  const Text(
                    '24ch Real-Time RTA Spectrum Overlay',
                    style: TextStyle(
                      color: Colors.white,
                      fontSize: 13,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(
                      color: Colors.cyan.shade900,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: const Text(
                      '60 FPS Target',
                      style: TextStyle(color: Colors.cyanAccent, fontSize: 9, fontWeight: FontWeight.bold),
                    ),
                  ),
                ],
              ),
              if (widget.onClose != null)
                IconButton(
                  icon: const Icon(Icons.close, size: 16, color: Colors.white54),
                  onPressed: widget.onClose,
                ),
            ],
          ),
          const SizedBox(height: 8),
          Expanded(
            child: RepaintBoundary(
              child: CustomPaint(
                size: Size.infinite,
                painter: _RtaPainter(channelBands: _channelBands),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _RtaPainter extends CustomPainter {
  final List<List<double>> channelBands;

  _RtaPainter({required this.channelBands});

  @override
  void paint(Canvas canvas, Size size) {
    if (size.width <= 0 || size.height <= 0) return;

    final int numBands = 31;
    final double bandWidth = size.width / numBands;

    // Draw grid lines
    final Paint gridPaint = Paint()
      ..color = Colors.white10
      ..strokeWidth = 1.0;

    for (int i = 1; i < 4; i++) {
      final double y = size.height * (i / 4.0);
      canvas.drawLine(Offset(0, y), Offset(size.width, y), gridPaint);
    }

    // Draw 24-channel overlays
    final List<Color> channelColors = [
      Colors.cyanAccent,
      Colors.amberAccent,
      Colors.greenAccent,
      Colors.pinkAccent,
      Colors.lightBlueAccent,
      Colors.orangeAccent,
    ];

    for (int ch = 0; ch < math.min(6, channelBands.length); ch++) {
      final Color color = channelColors[ch % channelColors.length];
      final Path path = Path();
      final List<double> bands = channelBands[ch];

      path.moveTo(0, size.height);
      for (int b = 0; b < numBands; b++) {
        final double x = b * bandWidth + bandWidth / 2;
        final double val = bands[b];
        final double y = size.height * (1.0 - val);
        if (b == 0) {
          path.lineTo(x, y);
        } else {
          path.lineTo(x, y);
        }
      }
      path.lineTo(size.width, size.height);
      path.close();

      final Paint fillPaint = Paint()
        ..color = color.withValues(alpha: 0.12)
        ..style = PaintingStyle.fill;

      final Paint strokePaint = Paint()
        ..color = color.withValues(alpha: 0.8)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.5;

      canvas.drawPath(path, fillPaint);
      canvas.drawPath(path, strokePaint);
    }
  }

  @override
  bool shouldRepaint(covariant _RtaPainter oldDelegate) => true;
}
