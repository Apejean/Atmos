import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/resampler_status_badge.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

/// 리샘플링 배지는 엔진이 보낸 장치·음원 샘플레이트를 그대로 보여 준다(2026-10-24 기능 점검).
/// 예전에는 "HQ Sinc Resampling: 44.1kHz ➔ 48kHz"가 고정이었다.
class _FakeEngineState extends EngineStateNotifier {
  _FakeEngineState(this.initial);
  final EngineState initial;

  @override
  EngineState build() => initial;
}

void main() {
  test('바꾸는 음원이 없으면 장치 샘플레이트만 DIRECT로 보인다', () {
    expect(resamplerBadgeText(deviceRate: 48000, sourceRates: const [], trackCount: 0), 'DIRECT: 48kHz');
    expect(resamplerBadgeText(deviceRate: 44100, sourceRates: const [], trackCount: 0), 'DIRECT: 44.1kHz');
  });

  test('바꾸는 음원이 있으면 원래 샘플레이트와 트랙 수를 보인다', () {
    expect(
      resamplerBadgeText(deviceRate: 48000, sourceRates: const [44100], trackCount: 2),
      'HQ Sinc: 44.1kHz ➔ 48kHz (트랙 2개)',
    );
    expect(
      resamplerBadgeText(deviceRate: 48000, sourceRates: const [44100, 96000], trackCount: 3),
      'HQ Sinc: 44.1/96kHz ➔ 48kHz (트랙 3개)',
    );
  });

  test('장치 샘플레이트를 모르면 모른다고 보인다', () {
    expect(resamplerBadgeText(deviceRate: 0, sourceRates: const [], trackCount: 0), 'SR —');
  });

  testWidgets('배지는 엔진 상태의 값을 보이고 눌러도 바뀌지 않는다', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineStateProvider.overrideWith(
            () => _FakeEngineState(
              EngineState(deviceSampleRate: 48000, resampledSourceRates: const [44100], resampledTrackCount: 1),
            ),
          ),
        ],
        child: const MaterialApp(home: Scaffold(body: Center(child: ResamplerStatusBadgeWidget()))),
      ),
    );
    expect(find.text('HQ Sinc: 44.1kHz ➔ 48kHz (트랙 1개)'), findsOneWidget);
    await tester.tap(find.byType(ResamplerStatusBadgeWidget));
    await tester.pump();
    expect(find.text('HQ Sinc: 44.1kHz ➔ 48kHz (트랙 1개)'), findsOneWidget);
    expect(find.textContaining('DIRECT'), findsNothing);
  });
}
