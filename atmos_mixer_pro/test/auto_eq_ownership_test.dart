import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/position_eq.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// 슬롯 1(경계면 저음)에 들어갈 자동 밴드 한 장.
List<AutoEqBand> autoBands({double gain = -4.0}) => [
      AutoEqBand(
        slot: kAutoEqSlotBoundary,
        enabled: true,
        type: EqType.lowShelf,
        freq: 150.0,
        gain: gain,
        q: 0.707,
      ),
    ];

void main() {
  setUp(() {
    TestWidgetsFlutterBinding.ensureInitialized();
    SharedPreferences.setMockInitialValues({});
  });

  test('잠그지 않았으면 사람이 고쳐도 자동 계산이 다시 반영된다', () {
    // 규칙: Lock Tuning으로 잠근 채널만 사람 값이 유지된다. 예전에는 값을 한 번
    // 건드리면 그 밴드가 영구히 수동으로 고정돼, EQ를 초기화한 뒤로는 스피커를
    // 옮겨도 다시 계산되지 않았다(실기 보고).
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(tuningStateProvider.notifier);

    List<AutoEqBand> bands(double gain) => [
          AutoEqBand(
            slot: kAutoEqSlotBoundary,
            enabled: true,
            type: EqType.lowShelf,
            freq: 150.0,
            gain: gain,
            q: 0.707,
          ),
        ];

    notifier.applyAutoTuning(1, ChannelTuningState.initial(), bands(-4.0));
    expect(notifier.getTuning(1).gains[kAutoEqSlotBoundary], -4.0);

    final edited = notifier.getTuning(1);
    final manual = List<double>.from(edited.gains)
      ..[kAutoEqSlotBoundary] = -1.5;
    notifier.saveTuning(1, edited.copyWith(gains: manual));
    expect(notifier.getTuning(1).gains[kAutoEqSlotBoundary], -1.5);

    notifier.applyAutoTuning(1, notifier.getTuning(1), bands(-9.0));
    expect(notifier.getTuning(1).gains[kAutoEqSlotBoundary], -9.0,
        reason: '잠그지 않은 채널인데 자동 계산이 반영되지 않았다');
  });

  test('게인·딜레이도 같은 규칙으로 자동 계산이 반영된다', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(tuningStateProvider.notifier);

    notifier.applyAutoTuning(
      1,
      ChannelTuningState.initial().copyWith(delay: 5.0, gainDb: -6.0),
      const [],
    );
    notifier.saveTuning(1, notifier.getTuning(1).copyWith(gainDb: -2.0));
    expect(notifier.getTuning(1).gainDb, -2.0);

    notifier.applyAutoTuning(
      1,
      notifier.getTuning(1).copyWith(delay: 9.0, gainDb: -6.0),
      const [],
    );
    expect(notifier.getTuning(1).gainDb, -6.0);
    expect(notifier.getTuning(1).delay, 9.0);
  });

  test('화면 반올림 때문에 자동 소유권이 풀리면 안 된다', () {
    // 실기 보고 3번: "Lock Tuning 했다가 풀면 스피커를 바꿔도 FX가 고정됨".
    //
    // EQ 화면은 값을 소수 1자리로 표시하고 저장할 때 그 문자열을 다시
    // 파싱한다. 자동 계산이 써넣은 114.04Hz는 저장 한 번에 114.0이 되는데,
    // 이걸 정확히 비교하면 "사람이 고쳤다"로 오판해서 모든 항목의 자동
    // 소유권이 풀리고 FX가 영구히 얼어붙었다.
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(tuningStateProvider.notifier);

    // 자동 계산이 소수점이 지저분한 값을 써넣는다.
    notifier.applyAutoTuning(
      1,
      ChannelTuningState.initial().copyWith(delay: 5.5432, gainDb: -6.0789),
      [
        const AutoEqBand(
          slot: kAutoEqSlotFloorBounce,
          enabled: true,
          type: EqType.bell,
          freq: 114.04,
          gain: -4.0,
          q: 4.0,
        ),
      ],
    );

    // 사용자가 EQ 화면을 열었다 닫는 것과 동등한 동작:
    // 표시 자릿수로 반올림된 값이 그대로 다시 저장된다.
    final shown = notifier.getTuning(1);
    notifier.saveTuning(
      1,
      shown.copyWith(
        delay: double.parse(shown.delay.toStringAsFixed(1)),
        gainDb: double.parse(shown.gainDb.toStringAsFixed(1)),
        freqs: shown.freqs
            .map((f) => double.parse(f.toStringAsFixed(1)))
            .toList(),
        gains: shown.gains
            .map((g) => double.parse(g.toStringAsFixed(1)))
            .toList(),
        qs: shown.qs.map((q) => double.parse(q.toStringAsFixed(3))).toList(),
      ),
    );

    final after = notifier.getTuning(1);
    expect(after.gainAuto, isTrue, reason: '반올림만으로 게인 자동이 풀렸다');
    expect(after.delayAuto, isTrue, reason: '반올림만으로 딜레이 자동이 풀렸다');
    expect(after.bandAuto.sublist(0, kAutoEqSlotCount),
        List.filled(kAutoEqSlotCount, true),
        reason: '반올림만으로 EQ 밴드 자동이 풀렸다');

    // 사람이 실제로 값을 바꿔도 자동 소유권은 그대로다. 잠그지 않은 채널은
    // 다음 계산에서 자동값이 다시 반영되는 게 새 규칙이다.
    notifier.saveTuning(1, after.copyWith(gainDb: after.gainDb - 1.0));
    expect(notifier.getTuning(1).gainAuto, isTrue);
    notifier.applyAutoTuning(
      1,
      notifier.getTuning(1).copyWith(gainDb: -6.0789),
      const [],
    );
    expect(notifier.getTuning(1).gainDb, closeTo(-6.0789, 1e-6));
  });

  test('앱 시작 시 config 동기화가 EQ 자동 소유권을 꺼뜨리면 안 된다', () {
    // 실기 계측으로 잡은 버그: bandAuto=[false x5]인데 gainAuto/delayAuto는
    // true인 조합이 나왔고, 출처가 syncFromBackendConfig였다. 이 함수는
    // 앱을 켤 때마다 자동으로 불리는데(global_state.dart), 거기서 전 밴드를
    // 수동으로 돌려놔서 스피커를 움직여도 EQ가 영영 갱신되지 않았다.
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(tuningStateProvider.notifier);

    final config = AppConfig(
      oscPort: 8000,
      bufferSize: 256,
      themeStartOscAddress: '',
      systemResetOscAddress: '',
      monoConfigs: {
        1: ChannelSetting(
          enabled: true,
          customName: 'ch1',
          delayMs: 3.0,
          eqBands: [],
          phaseInvert: false,
          gainDb: -2.0,
        ),
      },
      stereoConfigs: {},
      multiConfigs: {},
      rooms: [],
      roomZones: [],
      isExhibitionMode: false,
      masterHeadroomDb: 0.0,
      peakLimiterEnabled: true,
      oscWhitelist: [],
      globalReverbMix: 0.0,
      globalReverbDecay: 1.0,
    );

    // 시작 시 자동 동기화: 자동 소유권이 유지돼야 한다.
    notifier.syncFromBackendConfig(config);
    expect(
      notifier.getTuning(1).bandAuto.sublist(0, kAutoEqSlotCount),
      List.filled(kAutoEqSlotCount, true),
      reason: '앱 시작 config 로드가 EQ 자동 소유권을 꺼뜨렸다',
    );
    expect(notifier.getTuning(1).gainAuto, isTrue);
    expect(notifier.getTuning(1).delayAuto, isTrue);

    // 자동 계산이 실제로 EQ를 쓸 수 있어야 한다.
    notifier.applyAutoTuning(1, notifier.getTuning(1), [
      const AutoEqBand(
        slot: kAutoEqSlotLowCut,
        enabled: true,
        type: EqType.lowCut,
        freq: 80.0,
        gain: 0.0,
        q: 0.707,
      ),
    ]);
    expect(notifier.getTuning(1).freqs[kAutoEqSlotLowCut], 80.0);

    // config에 eqBands가 비어 있어도 이미 계산된 EQ를 지우면 안 된다.
    // 이 함수는 엔진이 config를 브로드캐스트할 때마다 불리므로, 여기서
    // 덮어쓰면 스피커 배치로 계산한 EQ가 곧바로 사라진다.
    notifier.syncFromBackendConfig(config);
    expect(
      notifier.getTuning(1).freqs[kAutoEqSlotLowCut],
      80.0,
      reason: 'config 브로드캐스트가 계산된 EQ를 1000Hz 기본값으로 지웠다',
    );
    expect(notifier.getTuning(1).bandEnabled[kAutoEqSlotLowCut], isTrue);

    // 사용자가 직접 임포트한 config는 확정값으로 보고 수동으로 둔다.
    notifier.syncFromBackendConfig(config, treatAsManual: true);
    expect(
      notifier.getTuning(1).bandAuto.sublist(0, kAutoEqSlotCount),
      List.filled(kAutoEqSlotCount, false),
      reason: '명시적 임포트는 자동 계산이 덮어쓰지 않아야 한다',
    );
  });

  test('저장된 슬로프가 드롭다운에 없는 값이면 불러올 때 정리한다', () {
    // 이전 빌드가 코너 로우컷에 48dB/oct를 저장했고, EQ 화면 드롭다운(12/18/24)이
    // assertion으로 죽었다. 기존 저장본도 안전하게 열려야 한다.
    final json = ChannelTuningState.initial().toJson()
      ..['bandSlopes'] = [48, 12, 6, 18, 24, 30, 12, 12];
    final restored = ChannelTuningState.fromJson(json);
    for (final s in restored.bandSlopes) {
      expect([12, 18, 24], contains(s));
    }
    expect(restored.bandSlopes[0], 24);
    expect(restored.bandSlopes[2], 12);
  });

  test('밴드를 초기화해도 다음 계산에서 자동으로 다시 채워진다', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(tuningStateProvider.notifier);

    const auto = AutoEqBand(
      slot: kAutoEqSlotBoundary,
      enabled: true,
      type: EqType.lowShelf,
      freq: 150.0,
      gain: -4.0,
      q: 0.707,
    );
    notifier.applyAutoTuning(1, ChannelTuningState.initial(), [auto]);

    final reset = notifier.getTuning(1);
    final zeroed = List<double>.from(reset.gains)..[kAutoEqSlotBoundary] = 0.0;
    notifier.saveTuning(1, reset.copyWith(gains: zeroed));

    expect(notifier.applyAutoTuning(1, notifier.getTuning(1), [auto]), isTrue);
    expect(notifier.getTuning(1).gains[kAutoEqSlotBoundary], -4.0);
  });

  test('bandAuto는 저장/복원을 건너뛰지 않는다', () {
    final tuning = ChannelTuningState.initial();
    final flags = List<bool>.from(tuning.bandAuto)..[2] = false;
    final restored =
        ChannelTuningState.fromJson(tuning.copyWith(bandAuto: flags).toJson());
    expect(restored.bandAuto, flags);

    // 이 기능이 생기기 전 저장본(키 없음)은 자동 슬롯을 무장한 채로 올린다.
    final legacy = tuning.toJson()..remove('bandAuto');
    expect(
      ChannelTuningState.fromJson(legacy).bandAuto,
      List.generate(8, (i) => i < kAutoEqSlotCount),
    );
  });
}
