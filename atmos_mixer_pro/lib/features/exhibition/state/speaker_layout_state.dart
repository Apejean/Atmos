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
  void addSpeaker(SpeakerNode node) {
    state = [...state, node];
    _saveToPrefsImmediate();
  }

  void updateSpeaker(SpeakerNode node, {bool immediate = false}) {
    state = [
      for (final n in state)
        if (n.id == node.id) node else n,
    ];
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
