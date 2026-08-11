import 'dart:async';
import 'dart:convert';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:flutter/foundation.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/trajectory.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
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
      'channel_positions': List.generate(
        ref.read(engineStateProvider).outputChannelCount,
        (index) {
          final node = nodes.where((n) => n.channel == index).firstOrNull;
          if (node == null) return null;
          return {
            'x': node.x / ref.read(blueprintProvider).scale,
            'y': node.y / ref.read(blueprintProvider).scale,
            'z': 0.0,
          };
        },
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
