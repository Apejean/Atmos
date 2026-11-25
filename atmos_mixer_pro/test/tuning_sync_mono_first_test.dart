import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// 출력 설정 창이 새로 켠 줄(`setOutputRow`의 기본 항목)과 같다.
const freshRow = ChannelSetting(
  enabled: true,
  customName: '',
  delayMs: 0.0,
  eqBands: [],
  phaseInvert: false,
  gainDb: 0.0,
);

AppConfig configWith({
  required Map<int, ChannelSetting> mono,
  Map<int, ChannelSetting> stereo = const {},
  Map<int, ChannelSetting> multi = const {},
}) =>
    AppConfig(
      oscPort: 8000,
      bufferSize: 256,
      themeStartOscAddress: '',
      systemResetOscAddress: '',
      monoConfigs: mono,
      stereoConfigs: stereo,
      multiConfigs: multi,
      rooms: [],
      roomZones: [],
      isExhibitionMode: false,
      masterHeadroomDb: 0.0,
      peakLimiterEnabled: true,
      globalReverbMix: 0.0,
      globalReverbDecay: 1.0,
      outputRoutingMode: OutputRoutingMode.trackMapping,
    );

/// 채널 보정값은 그 채널의 모노 줄 값이 이긴다(2026-11-22). 출력 설정에서 스테레오·멀티 줄을 새로 켜면 기본값 항목이
/// 생기는데, 예전 동기화는 모노 → 스테레오 → 멀티 순서로 덮어 그 기본값이 모노 보정값을 지웠다.
void main() {
  setUp(() {
    TestWidgetsFlutterBinding.ensureInitialized();
    SharedPreferences.setMockInitialValues({});
  });

  test('새로 켠 스테레오·멀티 줄의 기본값이 모노 줄 보정값을 덮지 않는다', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(tuningStateProvider.notifier);
    await notifier.ensureLoaded();

    const calibrated = ChannelSetting(
      enabled: true,
      customName: 'ch1',
      delayMs: 5.0,
      eqBands: [],
      phaseInvert: true,
      gainDb: -6.0,
    );
    // 확정값으로 불러온다(가져오기와 같다) — 지연·게인을 자동 계산이 갖지 않는다.
    notifier.syncFromBackendConfig(
      configWith(mono: {1: calibrated}, stereo: {1: freshRow}, multi: {1: freshRow}),
      treatAsManual: true,
    );

    expect(notifier.getTuning(1).delay, 5.0, reason: 'CH1 지연이 새 줄의 기본값으로 덮였다');
    expect(notifier.getTuning(1).gainDb, -6.0, reason: 'CH1 게인이 새 줄의 기본값으로 덮였다');
    expect(notifier.getTuning(1).phaseInvert, isTrue);
  });

  test('모노 줄이 없는 채널은 예전처럼 스테레오 줄 값을 쓴다', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(tuningStateProvider.notifier);
    await notifier.ensureLoaded();

    notifier.syncFromBackendConfig(
      configWith(mono: {}, stereo: {
        3: const ChannelSetting(
          enabled: true,
          customName: '',
          delayMs: 7.0,
          eqBands: [],
          phaseInvert: false,
          gainDb: -9.0,
        ),
      }),
      treatAsManual: true,
    );

    expect(notifier.getTuning(3).delay, 7.0);
    expect(notifier.getTuning(3).gainDb, -9.0);
  });

  test('튜닝 창에서 끈 스테레오 링크를 설정 동기화가 되돌리지 않는다', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(tuningStateProvider.notifier);
    await notifier.ensureLoaded();

    final config = configWith(mono: {1: freshRow, 2: freshRow}, stereo: {1: freshRow});
    notifier.syncFromBackendConfig(config);
    expect(notifier.getTuning(1).isStereoLinked, isTrue,
        reason: '스테레오 줄이 있는 채널은 처음에 링크된 채로 시작한다(예전과 같다)');

    notifier.saveTuning(1, notifier.getTuning(1).copyWith(isStereoLinked: false));
    notifier.syncFromBackendConfig(config);
    expect(notifier.getTuning(1).isStereoLinked, isFalse, reason: '앱을 켤 때마다 링크 스위치가 다시 켜졌다');
  });
}
