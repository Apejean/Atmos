import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/spatial_reverb_state.dart';

void main() {
  // 저장소 모의값을 깔지 않으면 provider의 비동기 로더가 플러그인 미등록 예외를
  // 던지고, 그 예외가 테스트 종료와 경합해 간헐적으로 실패한다.
  setUp(() => SharedPreferences.setMockInitialValues({}));
  TestWidgetsFlutterBinding.ensureInitialized();

  group('SpatialReverbNotifier - Per-Channel Independence Tests', () {
    test('Channel 1 and Channel 2 can have independent reverb types and parameters', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      final notifier = container.read(spatialReverbProvider.notifier);

      // 1. Select Channel 1 and set to Hall with Decay 4.5s
      notifier.selectChannel(1);
      notifier.setReverbType(ReverbType.hall);
      notifier.updateDecayTime(4.5);
      notifier.updateDryWet(90.0);

      // Verify Channel 1 settings
      var state = container.read(spatialReverbProvider);
      expect(state.selectedChannel, equals(1));
      expect(state.currentSettings.reverbType, equals(ReverbType.hall));
      expect(state.currentSettings.decayTime, equals(4.5));
      expect(state.currentSettings.dryWetPercent, equals(90.0));

      // 2. Select Channel 2 and set to Room with Decay 1.1s
      notifier.selectChannel(2);
      notifier.setReverbType(ReverbType.room);
      notifier.updateDecayTime(1.1);
      notifier.updateDryWet(35.0);

      // Verify Channel 2 settings
      state = container.read(spatialReverbProvider);
      expect(state.selectedChannel, equals(2));
      expect(state.currentSettings.reverbType, equals(ReverbType.room));
      expect(state.currentSettings.decayTime, equals(1.1));
      expect(state.currentSettings.dryWetPercent, equals(35.0));

      // 3. Verify Channel 1 was NOT altered by Channel 2 modifications
      final ch1Settings = state.getSettingsForChannel(1);
      expect(ch1Settings.reverbType, equals(ReverbType.hall));
      expect(ch1Settings.decayTime, equals(4.5));
      expect(ch1Settings.dryWetPercent, equals(90.0));

      // 4. Test Copy to All from Channel 2
      notifier.selectChannel(2);
      notifier.copyToAll();

      state = container.read(spatialReverbProvider);
      final ch1AfterCopy = state.getSettingsForChannel(1);
      expect(ch1AfterCopy.reverbType, equals(ReverbType.room));
      expect(ch1AfterCopy.decayTime, equals(1.1));
      expect(ch1AfterCopy.dryWetPercent, equals(35.0));
    });
  });

  // 사용자 결정(2026-10-09): 리버브는 사용자가 MIX를 올려야 걸린다. 예전 기본값 80%는 새 설치·새
  // 프로젝트에서 모든 출력에 홀 리버브를 걸었다.
  test('새 상태의 ALL OUTPUTS 리버브 MIX 기본값은 0%다', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);

    final state = container.read(spatialReverbProvider);
    expect(state.getSettingsForChannel(0).dryWetPercent, equals(0.0));
    expect(state.getSettingsForChannel(5).dryWetPercent, equals(0.0));
    expect(SpatialReverbSettings.fromJson(const {}).dryWetPercent, equals(0.0));
  });
}
