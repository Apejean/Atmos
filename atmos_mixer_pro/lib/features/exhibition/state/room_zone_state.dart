import 'dart:async';
import 'dart:convert';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';

const _kRoomZonePrefsKey = 'exhibition_room_zone_layout';

class RoomZoneState extends Notifier<List<RoomZone>> {
  Timer? _saveDebounceTimer;

  @override
  List<RoomZone> build() {
    _loadFromPrefs();
    ref.onDispose(() {
      _saveDebounceTimer?.cancel();
    });
    return [];
  }

  Future<void> _loadFromPrefs() async {
    final prefs = await SharedPreferences.getInstance();
    final jsonString = prefs.getString(_kRoomZonePrefsKey);
    if (jsonString != null) {
      try {
        final List<dynamic> decoded = jsonDecode(jsonString);
        state = decoded.map((e) => RoomZone.fromJson(e)).toList();
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
    await prefs.setString(_kRoomZonePrefsKey, jsonString);
  }

  void addRoomZone(RoomZone room) {
    state = [...state, room];
    _saveToPrefsImmediate();
  }

  void updateRoomZone(RoomZone room, {bool immediate = false}) {
    state = [
      for (final r in state)
        if (r.id == room.id) room else r
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
}

final roomZoneProvider =
    NotifierProvider<RoomZoneState, List<RoomZone>>(RoomZoneState.new);
