import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';

/// Resampler Active / Direct Status Badge Widget
class ResamplerStatusBadgeWidget extends ConsumerStatefulWidget {
  final int fileSampleRate; // e.g. 44100
  final int deviceSampleRate; // e.g. 48000
  final bool forceActive;

  const ResamplerStatusBadgeWidget({
    super.key,
    this.fileSampleRate = 44100,
    this.deviceSampleRate = 48000,
    this.forceActive = true,
  });

  @override
  ConsumerState<ResamplerStatusBadgeWidget> createState() =>
      _ResamplerStatusBadgeWidgetState();
}

class _ResamplerStatusBadgeWidgetState
    extends ConsumerState<ResamplerStatusBadgeWidget> {
  late bool _isResampling;

  @override
  void initState() {
    super.initState();
    _isResampling =
        widget.forceActive ||
        (widget.fileSampleRate != widget.deviceSampleRate);
  }

  void _toggleTestMode() {
    setState(() {
      _isResampling = !_isResampling;
    });
  }

  String _formatRate(int rate) {
    if (rate >= 1000) {
      final khz = rate / 1000.0;
      return khz % 1 == 0
          ? '${khz.toInt()}kHz'
          : '${khz.toStringAsFixed(1)}kHz';
    }
    return '${rate}Hz';
  }

  @override
  Widget build(BuildContext context) {
    final fileRateStr = _formatRate(widget.fileSampleRate);
    final deviceRateStr = _formatRate(widget.deviceSampleRate);

    return InkWell(
      onTap: _toggleTestMode,
      borderRadius: BorderRadius.circular(6),
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: _isResampling
              ? Colors.cyan.shade900.withValues(alpha: 0.8)
              : AppColors.cardSurface,
          borderRadius: BorderRadius.circular(6),
          border: Border.all(
            color: _isResampling ? Colors.cyanAccent : Colors.white12,
            width: 1,
          ),
          boxShadow: _isResampling
              ? [
                  BoxShadow(
                    color: Colors.cyanAccent.withValues(alpha: 0.2),
                    blurRadius: 4,
                    spreadRadius: 1,
                  ),
                ]
              : [],
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              _isResampling ? Icons.transform : Icons.sync_alt,
              size: 14,
              color: _isResampling ? Colors.cyanAccent : Colors.white54,
            ),
            const SizedBox(width: 6),
            Text(
              _isResampling
                  ? 'HQ Sinc Resampling: $fileRateStr ➔ $deviceRateStr'
                  : 'DIRECT: $deviceRateStr',
              style: TextStyle(
                color: _isResampling ? Colors.cyanAccent : Colors.white70,
                fontSize: 10,
                fontWeight: FontWeight.bold,
                letterSpacing: 0.5,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
