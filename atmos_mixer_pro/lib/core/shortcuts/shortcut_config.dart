import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

final shortcutManagerInstance = ShortcutManagerNotifier();
final shortcutManagerProvider = Provider<ShortcutManagerNotifier>((ref) => shortcutManagerInstance);

/// Atmos Mixer Pro 지원 단축키 액션 목록
enum ShortcutAction {
  globalPlayPause,  // Space: 오토메이션 전체 재생/일시정지
  drawTrajectory,   // T: 3D 소리 궤도 드로잉 모드
  fitView,          // F: 선택 궤도 화면 중앙 피팅
  hideShowAll,      // H: 전체 궤도 눈 아이콘 숨김/표시
  deleteSelected,   // Delete: 선택 노드/궤도 삭제
  cancelMode,       // Esc: 포커스 해제 / 드로잉 취소
}

extension ShortcutActionExtension on ShortcutAction {
  String get label {
    switch (this) {
      case ShortcutAction.globalPlayPause:
        return '재생 / 일시정지 (Global Play/Pause)';
      case ShortcutAction.drawTrajectory:
        return '궤도 드로잉 모드 (Draw Trajectory)';
      case ShortcutAction.fitView:
        return '선택 궤도 중앙 맞춤 (Fit View)';
      case ShortcutAction.hideShowAll:
        return '전체 궤도 숨김/표시 (Hide/Show All)';
      case ShortcutAction.deleteSelected:
        return '선택 노드/트랙 삭제 (Delete Selected)';
      case ShortcutAction.cancelMode:
        return '포커스 해제 / 취소 (Cancel / Reset)';
    }
  }

  String get category {
    switch (this) {
      case ShortcutAction.globalPlayPause:
        return '재생 제어 (Transport)';
      case ShortcutAction.drawTrajectory:
      case ShortcutAction.fitView:
      case ShortcutAction.hideShowAll:
      case ShortcutAction.deleteSelected:
      case ShortcutAction.cancelMode:
        return '캔버스 편집 (Canvas Editing)';
    }
  }
}

/// 단축키 키 바인딩 모델
class CustomKeyBinding {
  final LogicalKeyboardKey key;
  final bool isControl;
  final bool isShift;
  final bool isAlt;
  final bool isMeta;

  const CustomKeyBinding({
    required this.key,
    this.isControl = false,
    this.isShift = false,
    this.isAlt = false,
    this.isMeta = false,
  });

  /// Flutter SingleActivator 변환
  SingleActivator toSingleActivator() {
    return SingleActivator(
      key,
      control: isControl,
      shift: isShift,
      alt: isAlt,
      meta: isMeta,
    );
  }

  /// UI 라벨 표시 (예: "Cmd + Shift + T" 또는 "Space")
  String get displayLabel {
    final List<String> parts = [];
    if (isMeta) parts.add('Cmd');
    if (isControl) parts.add('Ctrl');
    if (isAlt) parts.add('Option');
    if (isShift) parts.add('Shift');

    String keyName = key.keyLabel;
    if (key == LogicalKeyboardKey.space) keyName = 'Space';
    if (key == LogicalKeyboardKey.escape) keyName = 'Esc';
    if (key == LogicalKeyboardKey.delete) keyName = 'Delete';
    if (key == LogicalKeyboardKey.backspace) keyName = 'Backspace';

    parts.add(keyName);
    return parts.join(' + ');
  }

  Map<String, dynamic> toJson() {
    return {
      'keyId': key.keyId,
      'keyLabel': key.keyLabel,
      'isControl': isControl,
      'isShift': isShift,
      'isAlt': isAlt,
      'isMeta': isMeta,
    };
  }

  factory CustomKeyBinding.fromJson(Map<String, dynamic> json) {
    return CustomKeyBinding(
      key: LogicalKeyboardKey(json['keyId'] as int),
      isControl: json['isControl'] as bool? ?? false,
      isShift: json['isShift'] as bool? ?? false,
      isAlt: json['isAlt'] as bool? ?? false,
      isMeta: json['isMeta'] as bool? ?? false,
    );
  }
}

/// 프로 DAW 기본 단축키 기본값 정의
final Map<ShortcutAction, CustomKeyBinding> kDefaultShortcuts = {
  ShortcutAction.globalPlayPause: const CustomKeyBinding(key: LogicalKeyboardKey.space),
  ShortcutAction.drawTrajectory: const CustomKeyBinding(key: LogicalKeyboardKey.keyT),
  ShortcutAction.fitView: const CustomKeyBinding(key: LogicalKeyboardKey.keyF),
  ShortcutAction.hideShowAll: const CustomKeyBinding(key: LogicalKeyboardKey.keyH),
  ShortcutAction.deleteSelected: const CustomKeyBinding(key: LogicalKeyboardKey.delete),
  ShortcutAction.cancelMode: const CustomKeyBinding(key: LogicalKeyboardKey.escape),
};

/// 전역 단축키 매니저 (ChangeNotifier)
class ShortcutManagerNotifier extends ChangeNotifier {
  Map<ShortcutAction, CustomKeyBinding> _bindings = Map.from(kDefaultShortcuts);

  Map<ShortcutAction, CustomKeyBinding> get bindings => _bindings;

  ShortcutManagerNotifier() {
    _loadFromDisk();
  }

  /// 단축키 업데이트
  void updateBinding(ShortcutAction action, CustomKeyBinding newBinding) {
    _bindings[action] = newBinding;
    _saveToDisk();
    notifyListeners();
  }

  /// 단축키 충돌 검사
  ShortcutAction? checkConflict(CustomKeyBinding binding, {ShortcutAction? ignoreAction}) {
    for (var entry in _bindings.entries) {
      if (entry.key == ignoreAction) continue;
      if (entry.value.key == binding.key &&
          entry.value.isControl == binding.isControl &&
          entry.value.isShift == binding.isShift &&
          entry.value.isAlt == binding.isAlt &&
          entry.value.isMeta == binding.isMeta) {
        return entry.key;
      }
    }
    return null;
  }

  /// 초기 기본값 복원
  void resetToDefault() {
    _bindings = Map.from(kDefaultShortcuts);
    _saveToDisk();
    notifyListeners();
  }

  Future<void> _saveToDisk() async {
    try {
      final dir = await getApplicationSupportDirectory();
      final file = File('${dir.path}/shortcuts_config.json');
      final jsonMap = _bindings.map((k, v) => MapEntry(k.name, v.toJson()));
      await file.writeAsString(jsonEncode(jsonMap));
    } catch (e) {
      debugPrint('Failed to save shortcuts: $e');
    }
  }

  Future<void> _loadFromDisk() async {
    try {
      final dir = await getApplicationSupportDirectory();
      final file = File('${dir.path}/shortcuts_config.json');
      if (await file.exists()) {
        final content = await file.readAsString();
        final jsonMap = jsonDecode(content) as Map<String, dynamic>;
        for (var entry in jsonMap.entries) {
          final action = ShortcutAction.values.firstWhere(
            (e) => e.name == entry.key,
            orElse: () => ShortcutAction.globalPlayPause,
          );
          _bindings[action] = CustomKeyBinding.fromJson(entry.value as Map<String, dynamic>);
        }
        notifyListeners();
      }
    } catch (e) {
      debugPrint('Failed to load shortcuts: $e');
    }
  }
}
