import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/environment_state_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/field_geometry.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/trajectory_state.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;

/// 공간 설정(스피커 좌표·방 형상·궤적·청취 지점)을 엔진에 보내는 **단일 소유자**.
///
/// 예전에는 스피커 배치·방·궤적 상태가 각자 payload를 만들어 보냈다. 그런데 payload
/// 하나를 만들려면 세 상태가 모두 필요해서 서로를 읽어야 했다(스피커 쪽은 방을 읽고,
/// 방 쪽은 스피커를 읽었다). Riverpod은 이걸 순환 의존으로 판단하고, 그 타이밍에
/// 걸리면 전송이 조용히 실패했다 — 화면은 정상인데 소리만 예전 설정으로 남는
/// 증상이다(실기 보고: "재스캔 후 스피커를 한 번 더 움직여야 정상").
///
/// 이제 이 provider만 세 상태를 읽고, 세 상태는 서로를 읽지 않는다. 상태가 바뀌면
/// 여기로 알림이 와서 payload를 다시 만들어 보낸다.
class SpatialSyncNotifier extends Notifier<void> {
  Timer? _throttle;

  /// 엔진에 보낸 횟수(테스트 검증용).
  int sentCount = 0;

  @override
  void build() {
    ref.listen(speakerLayoutProvider, (_, _) => _schedule());
    ref.listen(roomZoneProvider, (_, _) => _schedule());
    ref.listen(trajectoryProvider, (_, _) => _schedule());
    ref.listen(activeRoomIdProvider, (_, _) => _schedule());
    ref.listen(blueprintProvider, (_, _) => _schedule());
    // 온도가 바뀌면 음속이 바뀌어 현장 물리 밴드(경계면·룸 모드 주파수)가 달라진다.
    ref.listen(environmentStateProvider, (_, _) => _schedule());
    ref.onDispose(() => _throttle?.cancel());
  }

  /// 드래그처럼 초당 수십 번 바뀌는 경우를 위해 16ms(약 한 프레임)로 묶어 보낸다.
  void _schedule() {
    if (_throttle?.isActive ?? false) return;
    _throttle = Timer(const Duration(milliseconds: 16), sendNow);
  }

  /// 지금 상태를 즉시 보낸다. 엔진 재기동·프로젝트 불러오기 직후에 부른다
  /// (새 믹서는 config.json에 없는 값을 모른다 — core/state/engine_resync.dart).
  void sendNow() {
    _throttle?.cancel();
    try {
      _send();
    } catch (e) {
      // 엔진 미기동·테스트 환경 등. 다음 변경이나 재동기화에서 다시 시도된다.
      debugPrint('공간 설정 동기화 건너뜀: $e');
    }
  }

  void _send() {
    final nodes = ref.read(speakerLayoutProvider);
    final rooms = ref.read(roomZoneProvider);
    final trajectories = ref.read(trajectoryProvider);
    final bp = ref.read(blueprintProvider);
    final speedOfSound = ref.read(environmentStateProvider).speedOfSound;
    // 엔진이 아직 준비되지 않았으면 여기서 예외가 나고 전송을 건너뛴다.
    final channelCount = ref.read(engineStateProvider).outputChannelCount;

    final payload = {
      // 청취 지점(마네킹)과 지금 보고 있는 방. 3D 룸은 마네킹을 방 중심에 세우므로
      // 엔진도 같은 지점을 기준으로 방위각을 계산해야 한다. 방이 없으면 청사진
      // 캔버스 치수를 쓴다(dynamic_3d_room.dart의 폴백과 동일).
      ...buildListenerPayload(
        rooms: rooms,
        activeRoomId: ref.read(activeRoomIdProvider),
        fallbackWidth: bp.canvasWidthMeters,
        fallbackHeight: bp.canvasHeightMeters,
      ),
      'channel_positions': buildChannelPositionsPayload(
        nodes,
        channelCount,
        activeRoomId: activeRoomOf(rooms, ref.read(activeRoomIdProvider))?.id,
        // 헤드폰 미리듣기의 현장 물리 밴드(경계면 저음·룸 모드·근접면 반사·지향성). 자동 EQ와
        // 같은 모델·같은 청취 지점이라 헤드폰에서도 보정이 현장처럼 상쇄된다.
        simBands: (node) => fieldPhysicsPayloadFor(
          node,
          fieldGeometryFor(node, rooms, bp),
          speedOfSound,
        ),
      ),
      'room_zones': buildRoomZonesPayload(rooms),
      'trajectory':
          trajectories.isNotEmpty && trajectories.first.waypoints.isNotEmpty
              ? {
                  'waypoints': trajectories.first.waypoints
                      .map((w) => {
                            'x': w.position.dx,
                            'y': w.position.dy,
                            'z': w.heightZ,
                          })
                      .toList(),
                  'current_position': {
                    'x': trajectories.first.getCurrentPositionMeter().dx,
                    'y': trajectories.first.getCurrentPositionMeter().dy,
                    'z': trajectories.first.getCurrentHeightZ(),
                  },
                  'size': trajectories.first.size,
                  'audio_file_path': trajectories.first.audioFilePath,
                }
              : null,
    };

    // 여기까지 오면 payload 구성에 성공했다는 뜻이다(순환 의존 회귀 검증용).
    sentCount++;
    rust_api
        .apiUpdateSpatialConfigJson(jsonPayload: jsonEncode(payload))
        .catchError((e) {
      debugPrint('FFI sync error: $e');
    });
  }
}

final spatialSyncProvider =
    NotifierProvider<SpatialSyncNotifier, void>(SpatialSyncNotifier.new);
