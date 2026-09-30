import 'dart:async';
import 'dart:convert';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/position_eq.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/bass_management_provider.dart';


const _kSpeakerLayoutPrefsKey = 'exhibition_speaker_layout';
const _kSpeakerLayoutPrefsBackupKey = 'exhibition_speaker_layout_backup';

class SpeakerLayoutState extends Notifier<List<SpeakerNode>> {
  Timer? _saveDebounceTimer;

  @override
  List<SpeakerNode> build() {
    _loadFromPrefs();
    // 방 크기나 귀높이가 바뀌면 청취 지점이 움직이므로 자동 조준을 다시 건다.
    // 앱 시작 직후에는 방 정보가 스피커보다 늦게 올라올 수 있는데, 이 리스너가
    // 그 순서까지 함께 처리한다.
    ref.listen(roomZoneProvider, (previous, next) {
      reaimAutoSpeakers();
    });
    ref.onDispose(() {
      _saveDebounceTimer?.cancel();
    });
    return [];
  }

  /// 프로젝트 파일을 불러온 뒤 저장소를 다시 읽는다(core/state/project_file.dart).
  /// 스피커가 없는 프로젝트로 바꾼 경우 이전 프로젝트의 스피커를 지워야 하므로
  /// resetWhenAbsent를 켠다.
  Future<void> reloadFromPrefs() => _loadFromPrefs(resetWhenAbsent: true);

  /// [resetWhenAbsent]는 앱 시작 경로에서 끈다. 이 함수는 비동기라, 저장값이 없을 때
  /// 상태를 비우면 그 사이에 화면이 추가한 스피커까지 지워버린다.
  Future<void> _loadFromPrefs({bool resetWhenAbsent = false}) async {
    final prefs = await SharedPreferences.getInstance();
    final jsonString = prefs.getString(_kSpeakerLayoutPrefsKey);
    bool useBackup = false;

    if (jsonString == null) {
      // 프로젝트 전환으로 다시 읽는 경우에만 비운다(이전 프로젝트의 스피커 제거).
      if (resetWhenAbsent) state = [];
      return;
    }
    // 예전 저장본의 서브 지정(베이스 매니지먼트의 전역 lfeChannel)은 그 채널 스피커로
    // 옮겨 적는다.
    final bassJson = prefs.getString(kBassManagementPrefsKey);
    final legacyLfe = legacyLfeChannelOf(bassJson);
    {
      try {
        final List<dynamic> decoded = jsonDecode(jsonString);
        final nodes = decoded.map((e) => SpeakerNode.fromJson(e)).toList();
        state = legacyLfe == null ? nodes : adoptLegacyLfeChannel(nodes, legacyLfe);
        // 저장본은 addSpeaker/updateSpeaker를 거치지 않으므로 여기서 조준한다.
        // 이게 없으면 앱을 다시 켤 때마다 기존 스피커가 조준 안 된 채로 뜬다.
        reaimAutoSpeakers();
      } catch (e) {
        useBackup = true;
      }
    }
    final stripped = withoutLegacyLfeKeys(bassJson);
    if (!useBackup && stripped != null) {
      // 스피커 쪽을 먼저 저장하고 예전 키를 지운다. 중간에 꺼지면 다음 실행에 다시
      // 옮긴다(같은 결과). 지우지 않으면 사용자가 해제한 서브가 다음 실행에 되살아난다.
      if (legacyLfe != null) await _saveToPrefsImmediate();
      await prefs.setString(kBassManagementPrefsKey, stripped);
    }

    if (useBackup) {
      final backupString = prefs.getString(_kSpeakerLayoutPrefsBackupKey);
      if (backupString != null) {
        try {
          final List<dynamic> decoded = jsonDecode(backupString);
          state = decoded.map((e) => SpeakerNode.fromJson(e)).toList();
        } catch (e) {
          state = [];
        }
      } else {
        state = [];
      }
    }
  }

  // 엔진 전송은 이 상태가 하지 않는다. 공간 설정 payload는 세 상태(스피커·방·궤적)를
  // 모두 필요로 해서, 각자 보내면 서로를 읽어야 하고 그게 순환 의존이 됐다.
  // 이제 spatial_sync_provider.dart가 이 상태의 변화를 듣고 한 곳에서 보낸다.

  /// 프로젝트 파일을 저장하기 전에 미뤄둔 저장을 즉시 끝낸다
  /// (core/state/project_file.dart). 이게 없으면 방금 옮긴 스피커나 방금 돌린
  /// 노브가 파일에 빠진다.
  Future<void> flushPendingSave() {
    _saveDebounceTimer?.cancel();
    return _saveToPrefsImmediate();
  }

  void _saveToPrefsDebounced() {
    _saveDebounceTimer?.cancel();
    _saveDebounceTimer = Timer(const Duration(milliseconds: 300), () {
      _saveToPrefsImmediate();
    });
  }

  Future<void> _saveToPrefsImmediate() async {
    final prefs = await SharedPreferences.getInstance();
    
    final currentString = prefs.getString(_kSpeakerLayoutPrefsKey);
    if (currentString != null) {
      await prefs.setString(_kSpeakerLayoutPrefsBackupKey, currentString);
    }
    
    try {
      final jsonString = jsonEncode(state.map((e) => e.toJson()).toList());
      await prefs.setString(_kSpeakerLayoutPrefsKey, jsonString);
    } catch (e) {
      // Ignore save error to prevent crash
    }
  }


  /// 지금 스피커를 드래그 중인가.
  ///
  /// 시간정렬 딜레이는 드래그가 끝난 뒤에만 반영한다. 딜레이 탭을 매 프레임
  /// 옮기면 테이프 스톱/스타트 같은 소리가 나기 때문이다(아래
  /// acoustic_sync_provider 주석 참고). 게인·EQ·팬은 드래그 중에도 계속
  /// 따라간다.
  bool isDragging = false;

  /// [node]가 자동 조준 상태면 청취 지점(방 정중앙)을 향하도록 yaw/pitch를
  /// 다시 계산해 넣는다.
  ///
  /// 값을 읽는 쪽마다 "자동이면 정면으로 친다"고 해석하는 대신 필드 자체를
  /// 갱신한다. 그래야 3D 뷰·인스펙터·DSP가 전부 같은 각도를 본다.
  SpeakerNode _applyAutoAim(SpeakerNode node) {
    if (!node.autoAim) return node;
    // 방이 아직 없으면 청사진 캔버스 치수로 대신한다. 예전 'Auto-Aim to
    // Listener' 버튼이 쓰던 폴백과 같다.
    final rooms = ref.read(roomZoneProvider);
    final room = rooms.where((r) => r.id == node.roomId).firstOrNull ??
        (rooms.isNotEmpty ? rooms.first : null);
    final bp = ref.read(blueprintProvider);
    final double roomW = room?.physicalWidth ?? bp.canvasWidthMeters;
    final double roomD = room?.physicalHeight ?? bp.canvasHeightMeters;
    final double earLevel = room?.earLevel ?? 1.2;

    final aim = aimAtListener(
      speakerX: node.x,
      speakerY: node.y,
      speakerZ: node.heightZ,
      listenerX: roomW / 2.0,
      listenerY: roomD / 2.0,
      listenerZ: earLevel,
    );
    return node.copyWith(rotation: aim.yawDeg, pitchTilt: aim.pitchDeg);
  }

  /// 자동 조준 스피커 전체를 다시 조준한다(방 크기/귀높이가 바뀐 경우).
  void reaimAutoSpeakers() {
    bool changed = false;
    final next = <SpeakerNode>[];
    for (final n in state) {
      final aimed = _applyAutoAim(n);
      if (aimed.rotation != n.rotation || aimed.pitchTilt != n.pitchTilt) {
        changed = true;
      }
      next.add(aimed);
    }
    if (!changed) return;
    state = next;
    _saveToPrefsDebounced();
  }

  /// 재동기화 호출 횟수(테스트 검증용).
  int resyncCount = 0;

  /// 엔진이 (재)기동된 뒤 현재 배치를 통째로 다시 밀어 넣는다.
  ///
  /// 엔진을 다시 켜면 믹서가 새로 만들어지는데, 그 믹서는 config.json에 있는
  /// 값만 복원한다. 청취 지점(listener_position)과 채널별 팬 트림·리버브
  /// 센드·초기반사 믹스는 config에 없어서 기본값으로 남았다. 청취 지점이
  /// 비면 바이노럴이 "스피커 무게중심"으로 폴백하므로 소리가 엉뚱한 방향에서
  /// 난다(실기 보고: "재스캔 후 ch1이 가운데 뒤에서 들린다").
  ///
  /// 채널별 값들은 `updateSpeaker`에서만 전송되기 때문에, 사용자가 스피커를
  /// 한 번 더 움직이기 전까지 복구되지 않았다.
  void resyncToBackend() {
    resyncCount++;
    for (final node in state) {
      try {
        final chIdx = node.channel;
        rust_api.apiSetChannelReverbSend(
            channel: BigInt.from(chIdx), send: node.reverbSendNormalized);
        rust_api.apiSetChannelPanDeg(
            channel: BigInt.from(chIdx), panDeg: node.panDeg);
        rust_api.apiSetChannelEarlyRefMix(
            channel: BigInt.from(chIdx), mix: node.earlyRefMix.clamp(0.0, 1.0));
      } catch (_) {
        // 엔진이 아직 준비 전이면 조용히 건너뛴다(다음 재동기화에서 다시 시도).
      }
    }
  }

  void addSpeaker(SpeakerNode node) {
    state = [...state, _applyAutoAim(node)];
    _saveToPrefsImmediate();
  }

  void updateSpeaker(SpeakerNode node, {bool immediate = false, bool dragging = false}) {
    isDragging = dragging;
    node = _applyAutoAim(node);
    state = [
      for (final n in state)
        if (n.id == node.id) node else n,
    ];
    
    // Sync to Rust Backend (0-indexed channel, normalized 0.0 ~ 1.0 send)
    try {
      // SpeakerNode.channel은 이미 0-based다(첫 스피커가 channel 0으로 생성되고,
      // UI는 'Output CH ${channel + 1}'로 표시한다). 엔진의 channel_dsp /
      // channel_pan_deg 인덱스와 그대로 1:1 대응하므로 변환하면 안 된다.
      final chIdx = node.channel;
      rust_api.apiSetChannelReverbSend(channel: BigInt.from(chIdx), send: node.reverbSendNormalized);
      rust_api.apiSetChannelPanDeg(channel: BigInt.from(chIdx), panDeg: node.panDeg);
      rust_api.apiSetChannelEarlyRefMix(channel: BigInt.from(chIdx), mix: node.earlyRefMix.clamp(0.0, 1.0));
    } catch (_) {}

    if (immediate) {
      _saveToPrefsImmediate();
    } else {
      _saveToPrefsDebounced();
    }
  }

  void saveImmediately() {
    _saveToPrefsImmediate();
  }

  /// [speakerId] 스피커를 자기 방의 서브우퍼로 지정하거나 해제한다.
  ///
  /// 방마다 서브는 하나다. 지정하면 같은 방의 다른 서브는 해제한다 — 방의 저역을
  /// 모을 곳이 하나여야 화면과 엔진 라우팅(rust bass_route.rs)이 같은 채널을 가리킨다.
  /// 다른 방의 서브는 그대로 둔다.
  void setSubwoofer(String speakerId, bool isSubwoofer) {
    final target = state.where((n) => n.id == speakerId).firstOrNull;
    if (target == null) return;
    state = [
      for (final n in state)
        if (n.id == speakerId)
          n.copyWith(isSubwoofer: isSubwoofer)
        else if (isSubwoofer && n.isSubwoofer && n.roomId == target.roomId)
          n.copyWith(isSubwoofer: false)
        else
          n,
    ];
    _saveToPrefsImmediate();
  }

  void removeSpeaker(String id) {
    state = state.where((n) => n.id != id).toList();
    _saveToPrefsImmediate();
  }

  void clearAll() {
    state = [];
    _saveToPrefsImmediate();
  }
}

/// 예전 전역 서브 채널([lfeChannel], 내부 0-based)을 그 채널 스피커의 서브 지정으로
/// 옮긴다. 방마다 서브는 하나라, 이미 서브가 있는 방의 스피커는 건드리지 않는다.
List<SpeakerNode> adoptLegacyLfeChannel(List<SpeakerNode> nodes, int lfeChannel) {
  final roomsWithSub = {
    for (final n in nodes)
      if (n.isSubwoofer) n.roomId,
  };
  final result = <SpeakerNode>[];
  for (final n in nodes) {
    if (n.channel == lfeChannel && !n.isSubwoofer && roomsWithSub.add(n.roomId)) {
      result.add(n.copyWith(isSubwoofer: true));
    } else {
      result.add(n);
    }
  }
  return result;
}

final speakerLayoutProvider =
    NotifierProvider<SpeakerLayoutState, List<SpeakerNode>>(
      SpeakerLayoutState.new,
    );
