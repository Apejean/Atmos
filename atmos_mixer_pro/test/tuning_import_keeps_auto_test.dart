import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// 지연·게인이 0인 줄(보정값을 한 번도 넣지 않은 채널). 새로 켠 출력 줄과 같다.
const unsetRow = ChannelSetting(
  enabled: true,
  customName: '',
  delayMs: 0.0,
  eqBands: [],
  phaseInvert: false,
  gainDb: 0.0,
);

/// 손으로 보정해 둔 줄.
const calibratedRow = ChannelSetting(
  enabled: true,
  customName: '',
  delayMs: 5.0,
  eqBands: [],
  phaseInvert: false,
  gainDb: -6.0,
);

AppConfig configWith(Map<int, ChannelSetting> mono) => AppConfig(
      oscPort: 8000,
      bufferSize: 256,
      themeStartOscAddress: '',
      systemResetOscAddress: '',
      monoConfigs: mono,
      stereoConfigs: const {},
      multiConfigs: const {},
      rooms: [],
      roomZones: [],
      isExhibitionMode: false,
      masterHeadroomDb: 0.0,
      peakLimiterEnabled: true,
      globalReverbMix: 0.0,
      globalReverbDecay: 1.0,
    );

/// 프로젝트 파일을 불러올 때(dashboard_screen `_loadProject`)는 `treatAsManual: true`로 동기화한다. 손으로 맞춘
/// 지연·게인을 이 PC에 남아 있는 스피커 배치의 자동 계산이 덮지 않게 하려는 것이다. 그런데 보정값을 넣지 않은
/// (0) 채널까지 수동으로 굳히면, 스피커를 놓아도 그 채널의 자동 게인·지연이 영영 계산되지 않는다
/// (2026-11-24 Windows 실기: 스피커 배치 없는 프로젝트를 불러오자 CH1~3의 gainAuto/delayAuto가 false).
void main() {
  setUp(() {
    TestWidgetsFlutterBinding.ensureInitialized();
    SharedPreferences.setMockInitialValues({});
  });

  test('프로젝트에 보정값이 없는(0) 채널은 불러와도 자동 게인·지연이 그대로 자동이다', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(tuningStateProvider.notifier);
    await notifier.ensureLoaded();

    notifier.syncFromBackendConfig(
      configWith({1: unsetRow, 2: unsetRow, 3: unsetRow}),
      treatAsManual: true,
    );

    for (final ch in [1, 2, 3]) {
      expect(notifier.getTuning(ch).gainAuto, isTrue, reason: 'CH$ch 자동 게인이 수동으로 굳었다');
      expect(notifier.getTuning(ch).delayAuto, isTrue, reason: 'CH$ch 자동 지연이 수동으로 굳었다');
    }
  });

  test('손으로 보정한 값(0이 아님)은 예전처럼 확정값이라 수동으로 보호된다', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(tuningStateProvider.notifier);
    await notifier.ensureLoaded();

    notifier.syncFromBackendConfig(configWith({1: calibratedRow}), treatAsManual: true);

    expect(notifier.getTuning(1).delay, 5.0);
    expect(notifier.getTuning(1).gainDb, -6.0);
    expect(notifier.getTuning(1).delayAuto, isFalse, reason: '손 보정 지연이 자동 계산에 덮일 수 있다');
    expect(notifier.getTuning(1).gainAuto, isFalse, reason: '손 보정 게인이 자동 계산에 덮일 수 있다');
  });

  test('프로젝트에 EQ 밴드가 없는 채널은 불러와도 EQ 밴드 자동이 그대로다', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(tuningStateProvider.notifier);
    await notifier.ensureLoaded();

    notifier.syncFromBackendConfig(
      configWith({1: unsetRow, 2: unsetRow, 3: unsetRow}),
      treatAsManual: true,
    );

    for (final ch in [1, 2, 3]) {
      expect(notifier.getTuning(ch).bandAuto, ChannelTuningState.defaultBandAuto(),
          reason: 'CH$ch EQ 밴드 자동이 수동으로 굳었다(2026-11-25 Windows 실기: 8개 모두 false)');
    }
  });

  test('프로젝트에 든 EQ 밴드만 확정값으로 수동이 되고 나머지 밴드는 자동으로 남는다', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(tuningStateProvider.notifier);
    await notifier.ensureLoaded();

    const band = EqBand(enabled: true, freq: 300.0, gain: -3.0, qFactor: 0.707, filterType: EqType.bell, slopeDbPerOct: 12);
    notifier.syncFromBackendConfig(
      configWith({
        1: const ChannelSetting(
          enabled: true,
          customName: '',
          delayMs: 0.0,
          eqBands: [band, band],
          phaseInvert: false,
          gainDb: 0.0,
        ),
      }),
      treatAsManual: true,
    );

    final auto = notifier.getTuning(1).bandAuto;
    expect(auto.sublist(0, 2), [false, false], reason: '파일에 든 밴드는 확정값');
    expect(auto.sublist(2), ChannelTuningState.defaultBandAuto().sublist(2), reason: '파일에 없는 밴드는 자동 그대로');
  });

  test('지연만 보정한 채널은 게인만 자동으로 남는다', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(tuningStateProvider.notifier);
    await notifier.ensureLoaded();

    notifier.syncFromBackendConfig(
      configWith({
        1: const ChannelSetting(
          enabled: true,
          customName: '',
          delayMs: 5.0,
          eqBands: [],
          phaseInvert: false,
          gainDb: 0.0,
        ),
      }),
      treatAsManual: true,
    );

    expect(notifier.getTuning(1).delayAuto, isFalse);
    expect(notifier.getTuning(1).gainAuto, isTrue);
  });
}
