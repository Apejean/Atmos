import 'dart:async';
import 'dart:convert';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:flutter/foundation.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/trajectory.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';

const _kTrajectoryPrefsKey = 'exhibition_trajectory_layout';

class TrajectoryState extends Notifier<List<TrajectoryModel>> {
  Timer? _saveDebounceTimer;

  @override
  List<TrajectoryModel> build() {
    _loadFromPrefs();
    ref.onDispose(() {
      _saveDebounceTimer?.cancel();
    });
    return [];
  }

  Future<void> _loadFromPrefs() async {
    final prefs = await SharedPreferences.getInstance();
    final jsonString = prefs.getString(_kTrajectoryPrefsKey);
    if (jsonString != null) {
      try {
        final List<dynamic> decoded = jsonDecode(jsonString);
        state = decoded.map((e) => TrajectoryModel.fromJson(e)).toList();
      } catch (e) {
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
    final jsonString = jsonEncode(state.map((e) => e.toJson()).toList());
    await prefs.setString(_kTrajectoryPrefsKey, jsonString);
    _notifyBackend();
  }

  void _notifyBackend() {
    final nodes = ref.read(speakerLayoutProvider);
    final rooms = ref.read(roomZoneProvider);
    final trajectories = state;
    
    final payload = {
      // 리스너(마네킹)는 3D 룸에서 방 중심에 선다. 방위각 계산의 기준점이다.
      'listener_position': {
        'x': (rooms.isNotEmpty ? rooms.first.physicalWidth : 40.0) / 2.0,
        'y': (rooms.isNotEmpty ? rooms.first.physicalHeight : 40.0) / 2.0,
        'z': rooms.isNotEmpty ? rooms.first.earLevel : 1.2,
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
              'size': trajectories.first.size,
              'audio_file_path': trajectories.first.audioFilePath,
            }
          : null,
    };

    rust_api.apiUpdateSpatialConfigJson(jsonPayload: jsonEncode(payload)).catchError((e) {
      debugPrint('FFI sync error: $e');
    });
  }

  void addTrajectory(TrajectoryModel trajectory) {
    state = [...state, trajectory];
    _saveToPrefsImmediate();
  }

  void updateTrajectory(TrajectoryModel trajectory, {bool immediate = false}) {
    state = state.map((t) => t.id == trajectory.id ? trajectory : t).toList();
    if (immediate) {
      _saveToPrefsImmediate();
    } else {
      _saveToPrefsDebounced();
    }
  }

  void removeTrajectory(String id) {
    state = state.where((t) => t.id != id).toList();
    _saveToPrefsImmediate();
  }

  void clearAll() {
    state = [];
    _saveToPrefsImmediate();
  }
}

final trajectoryProvider = NotifierProvider<TrajectoryState, List<TrajectoryModel>>(
  TrajectoryState.new,
);

class ActiveTrajectoryIdNotifier extends Notifier<String?> {
  @override
  String? build() => null;
  void set(String? id) => state = id;
}

final activeTrajectoryIdProvider = NotifierProvider<ActiveTrajectoryIdNotifier, String?>(
  ActiveTrajectoryIdNotifier.new,
);

class IsDrawingModeNotifier extends Notifier<bool> {
  @override
  bool build() => false;
  void set(bool value) => state = value;
  void toggle() => state = !state;
}

final isDrawingModeProvider = NotifierProvider<IsDrawingModeNotifier, bool>(
  IsDrawingModeNotifier.new,
);

