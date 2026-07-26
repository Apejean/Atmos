import 'dart:async';
import 'dart:convert';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/trajectory.dart';

const _kTrajectoryPrefsKey = 'exhibition_trajectory_layout';

class TrajectoryState extends Notifier<List<Trajectory>> {
  Timer? _saveDebounceTimer;

  @override
  List<Trajectory> build() {
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
        state = decoded.map((e) => Trajectory.fromJson(e)).toList();
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
  }

  void addTrajectory(Trajectory trajectory) {
    state = [...state, trajectory];
    _saveToPrefsImmediate();
  }

  void updateTrajectory(Trajectory trajectory, {bool immediate = false}) {
    state = [
      for (final t in state)
        if (t.id == trajectory.id) trajectory else t,
    ];
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
}

final trajectoryProvider = NotifierProvider<TrajectoryState, List<Trajectory>>(
  TrajectoryState.new,
);
