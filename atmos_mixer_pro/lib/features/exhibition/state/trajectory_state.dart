import 'dart:async';
import 'dart:convert';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/trajectory.dart';

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

  /// 프로젝트 파일을 불러온 뒤 저장소를 다시 읽는다(core/state/project_file.dart).
  Future<void> reloadFromPrefs() => _loadFromPrefs(resetWhenAbsent: true);

  /// [resetWhenAbsent]는 앱 시작 경로에서 끈다(스피커 배치 쪽 주석 참고).
  Future<void> _loadFromPrefs({bool resetWhenAbsent = false}) async {
    final prefs = await SharedPreferences.getInstance();
    final jsonString = prefs.getString(_kTrajectoryPrefsKey);
    if (jsonString == null) {
      // 프로젝트 전환으로 다시 읽는 경우에만 비운다.
      if (resetWhenAbsent) state = [];
      return;
    }
    {
      try {
        final List<dynamic> decoded = jsonDecode(jsonString);
        state = decoded.map((e) => TrajectoryModel.fromJson(e)).toList();
      } catch (e) {
        state = [];
      }
    }
  }

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
    final jsonString = jsonEncode(state.map((e) => e.toJson()).toList());
    await prefs.setString(_kTrajectoryPrefsKey, jsonString);
  }

  /// 엔진 동기화. 의존 provider나 FFI가 아직 준비되지 않아도 UI 상태까지
  /// 같이 죽지 않도록 방어한다. 다음 변경 때 다시 시도된다.


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

