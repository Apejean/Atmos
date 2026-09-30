import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:atmos_mixer_pro/features/exhibition/state/bass_management_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/spatial_reverb_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/spatial_sync_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';

/// 엔진이 (재)기동된 직후 현재 설정을 통째로 백엔드에 다시 밀어 넣는다.
///
/// 오디오 엔진을 다시 켜면 믹서가 **새로 만들어지고**, 그 믹서는 config.json에
/// 저장된 값만 복원한다. 화면에서만 들고 있던 설정(청취 지점, 리버브 파라미터,
/// 채널별 팬 트림·리버브 센드·초기반사 믹스, 베이스 매니지먼트)은 아무도 다시
/// 보내주지 않아 기본값으로 남는다.
///
/// 그래서 재스캔 뒤에는 사용자가 컨트롤을 한 번 건드려 명령이 새로 나갈 때까지
/// 소리가 설정과 달랐다(실기 보고: "재스캔 후 스피커를 한 번 더 움직여야 정상",
/// "리버브 노브를 눌러야 그때 반영됨").
///
/// 엔진을 (재)기동하는 **모든** 경로에서 이 함수를 불러야 한다.
void resyncEngineStateToBackend({
  required SpeakerLayoutState layout,
  required SpatialSyncNotifier spatial,
  required SpatialReverbNotifier reverb,
  required BassManagementNotifier bass,
  required TuningStateNotifier tuning,
}) {
  // 공간 설정(스피커 좌표·방·궤적·청취 지점)
  spatial.sendNow();
  // 채널별 팬 트림·리버브 센드·초기반사 믹스
  layout.resyncToBackend();
  // 채널 튜닝(딜레이·게인·위상·EQ)
  tuning.applyAllToBackend();
  // 공간 리버브
  reverb.resyncToBackend();
  // 베이스 매니지먼트
  bass.resyncToBackend();
}

/// Notifier 안에서 부르는 형태.
void resyncEngineStateFromRef(Ref ref) => resyncEngineStateToBackend(
      layout: ref.read(speakerLayoutProvider.notifier),
      spatial: ref.read(spatialSyncProvider.notifier),
      reverb: ref.read(spatialReverbProvider.notifier),
      bass: ref.read(bassManagementProvider.notifier),
      tuning: ref.read(tuningStateProvider.notifier),
    );

/// 위젯 안에서 부르는 형태.
void resyncEngineStateFromWidgetRef(WidgetRef ref) => resyncEngineStateToBackend(
      layout: ref.read(speakerLayoutProvider.notifier),
      spatial: ref.read(spatialSyncProvider.notifier),
      reverb: ref.read(spatialReverbProvider.notifier),
      bass: ref.read(bassManagementProvider.notifier),
      tuning: ref.read(tuningStateProvider.notifier),
    );
