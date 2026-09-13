import 'dart:async';
import 'dart:convert';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:flutter/foundation.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/trajectory_state.dart';
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

  void _notifyBackend() {
    final nodes = state;
    final rooms = ref.read(roomZoneProvider);
    final trajectories = ref.read(trajectoryProvider);
    
    final bp = ref.read(blueprintProvider);

    // 리스너(마네킹) 기준점. 3D 룸은 마네킹을 방 중심에 세우므로
    // (studio_engine.html의 listenerGroup이 원점, posX = sp.x - width/2),
    // 엔진도 같은 지점을 기준으로 방위각을 계산해야 한다.
    //
    // 좌표 단위는 channel_positions와 반드시 같은 공간이어야 한다. 아래
    // buildChannelPositionsPayload가 scale로 나눈 값을 보내므로 여기서도
    // 같은 방식으로 나눈다(방위각은 균일 배율에 불변이라 각도는 정확하다).
    //
    // 이 값을 안 보내면 엔진이 "배치된 스피커들의 무게중심"으로 폴백하는데,
    // 스피커를 일렬로 늘어놓으면 무게중심이 그 직선 위에 놓여 모든 스피커가
    // 정확히 ±90°(하드 좌우)가 되어버린다.
    final listenerX = (bp.canvasWidthMeters / 2.0) / bp.scale;
    final listenerY = (bp.canvasHeightMeters / 2.0) / bp.scale;

    final payload = {
      'listener_position': {'x': listenerX, 'y': listenerY, 'z': 1.2},
      'channel_positions': buildChannelPositionsPayload(
        nodes,
        ref.read(engineStateProvider).outputChannelCount,
        bp.scale,
      ),
      'room_zones': rooms.map((r) {
        return {
          'room_id': r.id.hashCode.abs(),
          'boundary_min': {
            'x': r.x / ref.read(blueprintProvider).scale,
            'y': r.y / ref.read(blueprintProvider).scale,
            'z': 0.0,
          },
          'boundary_max': {
            'x': (r.x + r.width) / ref.read(blueprintProvider).scale,
            'y': (r.y + r.height) / ref.read(blueprintProvider).scale,
            'z': 2.0,
          },
          'absorption_coeff': r.absorptionCoeff,
          'material_name': r.materialName,
          'transmission_loss': r.wallTransmissionLoss,
        };
      }).toList(),
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
