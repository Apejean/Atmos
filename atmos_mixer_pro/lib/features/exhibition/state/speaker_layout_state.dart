import 'dart:convert';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';

const _kSpeakerLayoutPrefsKey = 'exhibition_speaker_layout';

class SpeakerLayoutState extends Notifier<List<SpeakerNode>> {
  @override
  List<SpeakerNode> build() {
    _loadFromPrefs();
    return [];
  }

  Future<void> _loadFromPrefs() async {
    final prefs = await SharedPreferences.getInstance();
    final jsonString = prefs.getString(_kSpeakerLayoutPrefsKey);
    if (jsonString != null) {
      try {
        final List<dynamic> decoded = jsonDecode(jsonString);
        state = decoded.map((e) => SpeakerNode.fromJson(e)).toList();
        _notifyBackend();
      } catch (e) {
        // Fallback to empty if parse fails
        state = [];
      }
    }
  }

  Future<void> _saveToPrefs() async {
    final prefs = await SharedPreferences.getInstance();
    final jsonString = jsonEncode(state.map((e) => e.toJson()).toList());
    await prefs.setString(_kSpeakerLayoutPrefsKey, jsonString);
    _notifyBackend();
  }

  void _notifyBackend() {
    // final jsonString = jsonEncode(state.map((e) => e.toJson()).toList());
    try {
      // Assuming apiUpdateSpeakerLayout is implemented by @Back agent
      // rust_api.apiUpdateSpeakerLayout(layoutJson: jsonString);
    } catch (e) {
      // print('apiUpdateSpeakerLayout not yet available: $e');
    }
  }

  void addSpeaker(SpeakerNode node) {
    state = [...state, node];
    _saveToPrefs();
  }

  void updateSpeaker(SpeakerNode node) {
    state = [
      for (final n in state)
        if (n.id == node.id) node else n
    ];
    _saveToPrefs();
  }

  void removeSpeaker(String id) {
    state = state.where((n) => n.id != id).toList();
    _saveToPrefs();
  }
}

final speakerLayoutProvider =
    NotifierProvider<SpeakerLayoutState, List<SpeakerNode>>(
        SpeakerLayoutState.new);
