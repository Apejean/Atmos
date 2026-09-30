import 'dart:async';
import 'dart:convert';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';

const _kRoomZonePrefsKey = 'exhibition_room_zone_layout';
const _kRoomZonePrefsBackupKey = 'exhibition_room_zone_layout_backup';

class RoomZoneState extends Notifier<List<RoomZone>> {
  bool _isLoaded = false;
  bool get isLoaded => _isLoaded;
  Timer? _saveDebounceTimer;

  @override
  List<RoomZone> build() {
    _loadFromPrefs();
    ref.onDispose(() {
      _saveDebounceTimer?.cancel();
    });
    return [];
  }

  /// 프로젝트 파일을 불러온 뒤 저장소를 다시 읽는다(core/state/project_file.dart).
  /// 방이 없는 프로젝트로 바꾼 경우 이전 프로젝트의 방을 지워야 하므로 켠다.
  Future<void> reloadFromPrefs() => _loadFromPrefs(resetWhenAbsent: true);

  /// [resetWhenAbsent]는 앱 시작 경로에서 끈다(스피커 배치 쪽 주석 참고 — 비동기라
  /// 그 사이 화면이 만든 기본 방을 지워버린다).
  Future<void> _loadFromPrefs({bool resetWhenAbsent = false}) async {
    final prefs = await SharedPreferences.getInstance();
    final jsonString = prefs.getString(_kRoomZonePrefsKey);
    bool useBackup = false;

    if (jsonString == null) {
      // 프로젝트 전환으로 다시 읽는 경우에만 비운다.
      if (resetWhenAbsent) state = [];
      _isLoaded = true;
      return;
    }
    {
      try {
        final List<dynamic> decoded = jsonDecode(jsonString);
        state = decoded.map((e) => RoomZone.fromJson(e)).toList();
      } catch (e) {
        useBackup = true;
      }
    }

    if (useBackup) {
      final backupString = prefs.getString(_kRoomZonePrefsBackupKey);
      if (backupString != null) {
        try {
          final List<dynamic> decoded = jsonDecode(backupString);
          state = decoded.map((e) => RoomZone.fromJson(e)).toList();
        } catch (e) {
          state = [];
        }
      } else {
        state = [];
      }
    }
    _isLoaded = true;
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
    
    final currentString = prefs.getString(_kRoomZonePrefsKey);
    if (currentString != null) {
      await prefs.setString(_kRoomZonePrefsBackupKey, currentString);
    }

    try {
      final jsonString = jsonEncode(state.map((e) => e.toJson()).toList());
      await prefs.setString(_kRoomZonePrefsKey, jsonString);
    } catch (e) {
      // Ignore save error to prevent crash
    }
  }



  void addRoomZone(RoomZone room) {
    state = [...state, room];
    _saveToPrefsImmediate();
  }

  void updateRoomZone(RoomZone room, {bool immediate = false}) {
    state = [
      for (final r in state)
        if (r.id == room.id) room else r,
    ];
    if (immediate) {
      _saveToPrefsImmediate();
    } else {
      _saveToPrefsDebounced();
    }
  }

  void removeRoomZone(String id) {
    state = state.where((r) => r.id != id).toList();
    _saveToPrefsImmediate();
  }

  void clearAll() {
    state = [];
    _saveToPrefsImmediate();
  }
}

final roomZoneProvider = NotifierProvider<RoomZoneState, List<RoomZone>>(
  RoomZoneState.new,
);

/// 지금 화면에서 보고 있는 방의 id.
///
/// 헤드폰 미리듣기(바이노럴)의 청취 지점과, 헤드폰에 들려줄 스피커를 고르는 기준이다.
/// 예전에는 화면 내부 상태로만 들고 있어서 엔진이 알 수 없었고, 그래서 항상 첫 번째
/// 방 기준으로 계산됐다.
class ActiveRoomIdNotifier extends Notifier<String?> {
  @override
  String? build() => null;

  void set(String? roomId) {
    if (state == roomId) return;
    state = roomId;
    // 엔진 전송은 RoomZoneState가 이 provider를 듣고 있다가 한다.
    // 여기서 roomZoneProvider를 읽으면 서로를 참조해(순환 의존) 전송이 조용히
    // 실패한다 — payload를 만들 때 RoomZoneState가 이미 이 provider를 읽는다.
  }
}

final activeRoomIdProvider =
    NotifierProvider<ActiveRoomIdNotifier, String?>(ActiveRoomIdNotifier.new);
