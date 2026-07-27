import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';

enum AppAction {
  togglePlayback,
  drawTrajectory,
  fitView,
  hideShowAll,
  soloHide,
  deleteSelection,
  escape,
}

class ShortcutBinding {
  final LogicalKeyboardKey key;
  final bool isMetaPressed;
  final bool isShiftPressed;
  final bool isAltPressed;

  const ShortcutBinding({
    required this.key,
    this.isMetaPressed = false,
    this.isShiftPressed = false,
    this.isAltPressed = false,
  });

  Map<String, dynamic> toJson() => {
        'keyId': key.keyId,
        'isMetaPressed': isMetaPressed,
        'isShiftPressed': isShiftPressed,
        'isAltPressed': isAltPressed,
      };

  factory ShortcutBinding.fromJson(Map<String, dynamic> json) {
    return ShortcutBinding(
      key: LogicalKeyboardKey(json['keyId'] as int),
      isMetaPressed: json['isMetaPressed'] as bool? ?? false,
      isShiftPressed: json['isShiftPressed'] as bool? ?? false,
      isAltPressed: json['isAltPressed'] as bool? ?? false,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ShortcutBinding &&
          runtimeType == other.runtimeType &&
          key == other.key &&
          isMetaPressed == other.isMetaPressed &&
          isShiftPressed == other.isShiftPressed &&
          isAltPressed == other.isAltPressed;

  @override
  int get hashCode => Object.hash(key, isMetaPressed, isShiftPressed, isAltPressed);
}

class ShortcutsManager extends Notifier<Map<AppAction, ShortcutBinding>> {
  static const Map<AppAction, ShortcutBinding> defaultBindings = {
    AppAction.togglePlayback: ShortcutBinding(key: LogicalKeyboardKey.space),
    AppAction.drawTrajectory: ShortcutBinding(key: LogicalKeyboardKey.keyT),
    AppAction.fitView: ShortcutBinding(key: LogicalKeyboardKey.keyF),
    AppAction.hideShowAll: ShortcutBinding(key: LogicalKeyboardKey.keyH),
    AppAction.soloHide: ShortcutBinding(key: LogicalKeyboardKey.shiftLeft), // Using shift for simplicity
    AppAction.deleteSelection: ShortcutBinding(key: LogicalKeyboardKey.delete),
    AppAction.escape: ShortcutBinding(key: LogicalKeyboardKey.escape),
  };

  @override
  Map<AppAction, ShortcutBinding> build() {
    _loadConfig();
    return defaultBindings;
  }

  Future<void> _loadConfig() async {
    try {
      final docDir = await getApplicationDocumentsDirectory();
      final file = File('${docDir.path}/shortcuts_config.json');
      if (await file.exists()) {
        final content = await file.readAsString();
        final Map<String, dynamic> json = jsonDecode(content);
        final newMap = <AppAction, ShortcutBinding>{};
        json.forEach((k, v) {
          final action = AppAction.values.firstWhere((e) => e.name == k);
          newMap[action] = ShortcutBinding.fromJson(v as Map<String, dynamic>);
        });
        state = newMap;
      }
    } catch (e) {
      debugPrint('Error loading shortcuts config: $e');
    }
  }

  Future<void> saveConfig() async {
    try {
      final docDir = await getApplicationDocumentsDirectory();
      final file = File('${docDir.path}/shortcuts_config.json');
      final jsonMap = state.map((key, value) => MapEntry(key.name, value.toJson()));
      await file.writeAsString(jsonEncode(jsonMap));
    } catch (e) {
      debugPrint('Error saving shortcuts config: $e');
    }
  }

  void resetToDefault() {
    state = defaultBindings;
    saveConfig();
  }

  bool setBinding(AppAction action, ShortcutBinding newBinding, {bool force = false}) {
    // Conflict Detection
    AppAction? conflictingAction;
    for (var entry in state.entries) {
      if (entry.key != action && entry.value == newBinding) {
        conflictingAction = entry.key;
        break;
      }
    }

    if (conflictingAction != null && !force) {
      // Return false indicating a conflict, UI should prompt user
      return false;
    }

    final newState = Map<AppAction, ShortcutBinding>.from(state);
    if (conflictingAction != null) {
      newState.remove(conflictingAction);
    }
    newState[action] = newBinding;
    state = newState;
    saveConfig();
    return true;
  }
}

final shortcutsProvider = NotifierProvider<ShortcutsManager, Map<AppAction, ShortcutBinding>>(
  ShortcutsManager.new,
);
