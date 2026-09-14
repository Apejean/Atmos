import 'dart:async';
import 'dart:convert';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:flutter/foundation.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/trajectory_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';


const _kSpeakerLayoutPrefsKey = 'exhibition_speaker_layout';
const _kSpeakerLayoutPrefsBackupKey = 'exhibition_speaker_layout_backup';

class SpeakerLayoutState extends Notifier<List<SpeakerNode>> {
  Timer? _saveDebounceTimer;

  @override
  List<SpeakerNode> build() {
    _loadFromPrefs();
    ref.onDispose(() {
      _saveDebounceTimer?.cancel();
    });
    return [];
  }

  Future<void> _loadFromPrefs() async {
    final prefs = await SharedPreferences.getInstance();
    final jsonString = prefs.getString(_kSpeakerLayoutPrefsKey);
    bool useBackup = false;

    if (jsonString != null) {
      try {
        final List<dynamic> decoded = jsonDecode(jsonString);
        state = decoded.map((e) => SpeakerNode.fromJson(e)).toList();
        _notifyBackend();
      } catch (e) {
        useBackup = true;
      }
    }

    if (useBackup) {
      final backupString = prefs.getString(_kSpeakerLayoutPrefsBackupKey);
      if (backupString != null) {
        try {
          final List<dynamic> decoded = jsonDecode(backupString);
          state = decoded.map((e) => SpeakerNode.fromJson(e)).toList();
          _notifyBackend();
        } catch (e) {
          state = [];
        }
      } else {
        state = [];
      }
    }
  }

  Timer? _engineSyncThrottle;
  bool _engineSyncPending = false;

  /// 엔진으로 좌표를 보내는 것과 디스크에 저장하는 것을 분리한다.
  ///
  /// 저장은 무겁고(SharedPreferences 직렬화 + 디스크 I/O) 드래그 중 매 프레임
  /// 할 필요가 없어 300ms 디바운스를 쓴다. 그런데 예전에는 엔진 전송
  /// (`_notifyBackend`)이 그 저장 경로 안에 들어있어서, 드래그를 **놓은 뒤**
  /// 300ms가 지나야 비로소 소리가 새 위치를 따라왔다.
  ///
  /// 엔진 전송은 lock-free 커맨드 채널로 가는 가벼운 작업이므로, 드래그
  /// 중에도 약 30fps로 흘려보내 소리가 스피커를 따라 움직이게 한다.
  void _notifyBackendThrottled() {
    if (_engineSyncThrottle?.isActive ?? false) {
      _engineSyncPending = true;
      return;
    }
    _notifyBackend();
    _engineSyncThrottle = Timer(const Duration(milliseconds: 33), () {
      if (_engineSyncPending) {
        _engineSyncPending = false;
        _notifyBackend();
      }
    });
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
      _notifyBackend();
    } catch (e) {
      // Ignore save error to prevent crash
    }
  }

  /// 엔진에 공간 설정을 보낸다.
  ///
  /// 의존 provider가 아직 준비되지 않았거나(예: 엔진 스트림 미기동, 테스트
  /// 환경) FFI 호출이 실패해도 UI 상태 갱신까지 같이 죽으면 안 되므로 전체를
  /// 방어한다. 동기화는 다음 변경 때 다시 시도된다.
  void _notifyBackend() {
    try {
      _notifyBackendInner();
    } catch (e) {
      debugPrint('공간 설정 동기화 건너뜀: $e');
    }
  }

  void _notifyBackendInner() {
    final nodes = state;
    final rooms = ref.read(roomZoneProvider);
    final trajectories = ref.read(trajectoryProvider);
    
    final bp = ref.read(blueprintProvider);

    // 리스너(마네킹) 기준점. 3D 룸은 마네킹을 방 중심에 세우므로
    // (studio_engine.html의 listenerGroup이 원점, posX = sp.x - width/2),
    // 엔진도 같은 지점을 기준으로 방위각을 계산해야 한다.
    //
    // 좌표 단위는 channel_positions와 같은 미터 공간이다.
    //
    // 이 값을 안 보내면 엔진이 "배치된 스피커들의 무게중심"으로 폴백하는데,
    // 스피커를 일렬로 늘어놓으면 무게중심이 그 직선 위에 놓여 모든 스피커가
    // 정확히 ±90°(하드 좌우)가 되어버린다.
    // 방 크기는 3D 룸과 같은 출처를 쓴다: 방이 있으면 그 물리 치수, 없으면
    // 블루프린트 캔버스 크기(dynamic_3d_room.dart의 폴백과 동일).
    final roomW = rooms.isNotEmpty ? rooms.first.physicalWidth : bp.canvasWidthMeters;
    final roomD = rooms.isNotEmpty ? rooms.first.physicalHeight : bp.canvasHeightMeters;
    final earLevel = rooms.isNotEmpty ? rooms.first.earLevel : 1.2;

    final payload = {
      'listener_position': {
        'x': roomW / 2.0,
        'y': roomD / 2.0,
        'z': earLevel,
      },
      'channel_positions': buildChannelPositionsPayload(
        nodes,
        ref.read(engineStateProvider).outputChannelCount,
      ),
      'room_zones': buildRoomZonesPayload(rooms),
      'trajectory':
          trajectories.isNotEmpty && trajectories.first.waypoints.isNotEmpty
          ? {
              'waypoints': trajectories.first.waypoints
                  .map(
                    (w) => {
                      'x': w.position.dx,
                      'y': w.position.dy,
                      'z': w.heightZ,
                    },
                  )
                  .toList(),
              'current_position': {
                'x': trajectories.first.getCurrentPositionMeter().dx,
                'y': trajectories.first.getCurrentPositionMeter().dy,
                'z': trajectories.first.getCurrentHeightZ(),
              },
              'audio_file_path': trajectories.first.audioFilePath,
            }
          : null,
    };

    rust_api.apiUpdateSpatialConfigJson(jsonPayload: jsonEncode(payload)).catchError((e) {
      debugPrint('FFI sync error: $e');
    });
  }
  void addSpeaker(SpeakerNode node) {
    state = [...state, node];
    _saveToPrefsImmediate();
  }

  void updateSpeaker(SpeakerNode node, {bool immediate = false}) {
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
      final normalizedSend = (node.reverbSend > 1.0 ? (node.reverbSend / 100.0) : node.reverbSend).clamp(0.0, 1.0);
      rust_api.apiSetChannelReverbSend(channel: BigInt.from(chIdx), send: normalizedSend);
      rust_api.apiSetChannelPanDeg(channel: BigInt.from(chIdx), panDeg: node.panDeg);
      rust_api.apiSetChannelEarlyRefMix(channel: BigInt.from(chIdx), mix: node.earlyRefMix.clamp(0.0, 1.0));
    } catch (_) {}
    // 위치 변경을 엔진에 바로 흘린다(드래그 중에도 소리가 따라 움직이도록).
    // 디스크 저장은 아래에서 따로 디바운스한다.
    _notifyBackendThrottled();

    if (immediate) {
      _saveToPrefsImmediate();
    } else {
      _saveToPrefsDebounced();
    }
  }

  void saveImmediately() {
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

final speakerLayoutProvider =
    NotifierProvider<SpeakerLayoutState, List<SpeakerNode>>(
      SpeakerLayoutState.new,
    );
