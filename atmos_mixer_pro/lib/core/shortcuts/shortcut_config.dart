import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

class ShortcutAction {
  final String id;
  final String name;
  final String category;
  final LogicalKeyboardKey defaultKey;
  final bool defaultCtrl;
  final bool defaultShift;
  final bool defaultAlt;

  LogicalKeyboardKey currentKey;
  bool currentCtrl;
  bool currentShift;
  bool currentAlt;

  ShortcutAction({
    required this.id,
    required this.name,
    required this.category,
    required this.defaultKey,
    this.defaultCtrl = false,
    this.defaultShift = false,
    this.defaultAlt = false,
  })  : currentKey = defaultKey,
        currentCtrl = defaultCtrl,
        currentShift = defaultShift,
        currentAlt = defaultAlt;

  String get label {
    final parts = <String>[];
    if (currentCtrl) parts.add('Ctrl');
    if (currentAlt) parts.add('Alt');
    if (currentShift) parts.add('Shift');
    parts.add(currentKey.keyLabel);
    return parts.join(' + ');
  }

  void reset() {
    currentKey = defaultKey;
    currentCtrl = defaultCtrl;
    currentShift = defaultShift;
    currentAlt = defaultAlt;
  }
}

class ShortcutManagerConfig extends ChangeNotifier {
  final Map<String, ShortcutAction> shortcuts = {
    'save_scene': ShortcutAction(
      id: 'save_scene',
      name: '현재 씬 저장',
      category: '전시/캔버스',
      defaultKey: LogicalKeyboardKey.keyS,
      defaultCtrl: true,
    ),
    'start_scheduler': ShortcutAction(
      id: 'start_scheduler',
      name: '스케줄러 시작',
      category: '전시/캔버스',
      defaultKey: LogicalKeyboardKey.space,
    ),
  };

  void updateShortcut(String id, LogicalKeyboardKey key, {bool ctrl = false, bool shift = false, bool alt = false}) {
    final action = shortcuts[id];
    if (action != null) {
      action.currentKey = key;
      action.currentCtrl = ctrl;
      action.currentShift = shift;
      action.currentAlt = alt;
      notifyListeners();
    }
  }

  void resetAll() {
    for (final s in shortcuts.values) {
      s.reset();
    }
    notifyListeners();
  }
}

final shortcutManagerInstance = ShortcutManagerConfig();
