import 'dart:async';
import 'dart:convert';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';

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
    // Sync with backend if needed
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
}

final speakerLayoutProvider =
    NotifierProvider<SpeakerLayoutState, List<SpeakerNode>>(
      SpeakerLayoutState.new,
    );
