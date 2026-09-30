import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:atmos_mixer_pro/core/state/engine_resync.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/bass_management_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/spatial_reverb_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/spatial_sync_provider.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';

/// 엔진을 다시 켜면 믹서가 새로 만들어지고 config.json에 없는 설정은 사라진다.
/// 재동기화가 그 설정들을 **빠짐없이** 다시 밀어 넣는지 확인한다.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('재동기화는 공간 설정·튜닝·리버브·베이스를 모두 다시 보낸다', () {
    SharedPreferences.setMockInitialValues({});
    final container = ProviderContainer();
    addTearDown(container.dispose);

    final layout = container.read(speakerLayoutProvider.notifier);
    final reverb = container.read(spatialReverbProvider.notifier);
    final bass = container.read(bassManagementProvider.notifier);
    final tuning = container.read(tuningStateProvider.notifier);
    final spatial = container.read(spatialSyncProvider.notifier);

    expect(layout.resyncCount, 0);
    expect(reverb.resyncCount, 0);
    expect(bass.resyncCount, 0);

    // 엔진이 없는 환경에서도 예외로 죽지 않아야 한다(기동 직후 호출되므로).
    resyncEngineStateToBackend(
      layout: layout,
      spatial: spatial,
      reverb: reverb,
      bass: bass,
      tuning: tuning,
    );

    expect(layout.resyncCount, 1, reason: '채널별 팬·센드·초기반사를 다시 보내지 않았다');
    expect(reverb.resyncCount, 1, reason: '리버브 설정을 다시 보내지 않았다');
    expect(bass.resyncCount, 1, reason: '베이스 매니지먼트를 다시 보내지 않았다');
  });
}
