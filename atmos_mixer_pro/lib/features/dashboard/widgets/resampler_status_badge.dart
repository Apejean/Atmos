import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';

/// 리샘플링 상태 배지. 엔진이 보낸 실제 값을 보여 준다(2026-10-24 기능 점검). 예전에는 "44.1kHz ➔ 48kHz"를
/// 고정으로 보여 줬고, 누르면 화면만 바뀌는 시험용 토글이었다.
class ResamplerStatusBadgeWidget extends ConsumerWidget {
  const ResamplerStatusBadgeWidget({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final deviceRate = ref.watch(engineStateProvider.select((s) => s.deviceSampleRate));
    final sourceRates = ref.watch(engineStateProvider.select((s) => s.resampledSourceRates));
    final trackCount = ref.watch(engineStateProvider.select((s) => s.resampledTrackCount));
    final isResampling = trackCount > 0 && sourceRates.isNotEmpty;

    return Tooltip(
      message: '재생 중인 음원의 샘플레이트를 장치 샘플레이트로 바꾸고 있는지 보여 줍니다.\n'
          'HQ Sinc: 엔진이 고품질 sinc 필터로 바꾸는 중 · DIRECT: 바꾸지 않고 그대로 재생',
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: isResampling
              ? Colors.cyan.shade900.withValues(alpha: 0.8)
              : AppColors.cardSurface,
          borderRadius: BorderRadius.circular(6),
          border: Border.all(
            color: isResampling ? Colors.cyanAccent : Colors.white12,
            width: 1,
          ),
          boxShadow: isResampling
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
              isResampling ? Icons.transform : Icons.sync_alt,
              size: 14,
              color: isResampling ? Colors.cyanAccent : Colors.white54,
            ),
            const SizedBox(width: 6),
            Text(
              resamplerBadgeText(
                deviceRate: deviceRate,
                sourceRates: sourceRates,
                trackCount: trackCount,
              ),
              style: TextStyle(
                color: isResampling ? Colors.cyanAccent : Colors.white70,
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

/// 배지 문구.
/// - 장치 샘플레이트를 모르면 `SR —`
/// - 바꾸는 음원이 없으면(재생 중인 것이 없을 때 포함) `DIRECT: 48kHz`
/// - 있으면 `HQ Sinc: 44.1kHz ➔ 48kHz (트랙 2개)`, 원래 값이 여러 개면 `44.1/96kHz`
String resamplerBadgeText({
  required int deviceRate,
  required List<int> sourceRates,
  required int trackCount,
}) {
  if (deviceRate <= 0) return 'SR —';
  final device = '${_khz(deviceRate)}kHz';
  if (trackCount <= 0 || sourceRates.isEmpty) return 'DIRECT: $device';
  final sources = '${sourceRates.map(_khz).join('/')}kHz';
  return 'HQ Sinc: $sources ➔ $device (트랙 $trackCount개)';
}

String _khz(int rate) {
  final khz = rate / 1000.0;
  return khz % 1 == 0 ? '${khz.toInt()}' : khz.toStringAsFixed(1);
}
