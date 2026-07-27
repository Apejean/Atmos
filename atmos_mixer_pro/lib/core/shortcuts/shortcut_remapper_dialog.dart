import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'shortcut_config.dart';

/// 단축키 리매핑 환경설정 다이얼로그 (Preferences -> Keybindings Remapper)
class ShortcutRemapperDialog extends StatefulWidget {
  final ShortcutManagerNotifier managerNotifier;

  const ShortcutRemapperDialog({
    super.key,
    required this.managerNotifier,
  });

  static Future<void> show(BuildContext context, ShortcutManagerNotifier manager) {
    return showDialog(
      context: context,
      builder: (context) => ShortcutRemapperDialog(managerNotifier: manager),
    );
  }

  @override
  State<ShortcutRemapperDialog> createState() => _ShortcutRemapperDialogState();
}

class _ShortcutRemapperDialogState extends State<ShortcutRemapperDialog> {
  ShortcutAction? _editingAction;

  @override
  Widget build(BuildContext context) {
    final notifier = widget.managerNotifier;

    return ListenableBuilder(
      listenable: notifier,
      builder: (context, _) {
        final bindings = notifier.bindings;

        return Dialog(
          backgroundColor: const Color(0xFF1A1C29), // Dark Slate Panel Surface
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
          child: Container(
            width: 620,
            height: 520,
            padding: const EdgeInsets.all(24),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // Header
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    const Row(
                      children: [
                        Icon(Icons.keyboard_alt_outlined, color: Color(0xFF00F2FE), size: 24),
                        SizedBox(width: 10),
                        Text(
                          '환경설정 - 단축키 리매핑 (Keybindings Remapper)',
                          style: TextStyle(
                            color: Colors.white,
                            fontSize: 16,
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                      ],
                    ),
                    IconButton(
                      icon: const Icon(Icons.close, color: Colors.grey),
                      onPressed: () => Navigator.of(context).pop(),
                    ),
                  ],
                ),
                const Divider(color: Colors.white12, height: 24),

                // Table Header
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                  decoration: BoxDecoration(
                    color: Colors.white.withValues(alpha: 0.05),
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: const Row(
                    children: [
                      Expanded(
                        flex: 3,
                        child: Text('기능명 (Action)', style: TextStyle(color: Colors.grey, fontSize: 12, fontWeight: FontWeight.bold)),
                      ),
                      Expanded(
                        flex: 2,
                        child: Text('단축키 (Key Binding)', style: TextStyle(color: Colors.grey, fontSize: 12, fontWeight: FontWeight.bold)),
                      ),
                      SizedBox(width: 60),
                    ],
                  ),
                ),
                const SizedBox(height: 8),

                // Shortcut Items List
                Expanded(
                  child: ListView.separated(
                    itemCount: ShortcutAction.values.length,
                    separatorBuilder: (context, index) => const Divider(color: Colors.white10, height: 1),
                    itemBuilder: (context, index) {
                      final action = ShortcutAction.values[index];
                      final binding = bindings[action]!;
                      final isEditing = _editingAction == action;

                      return Container(
                        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
                        child: Row(
                          children: [
                            Expanded(
                              flex: 3,
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text(
                                    action.label,
                                    style: const TextStyle(color: Colors.white, fontSize: 13, fontWeight: FontWeight.w500),
                                  ),
                                  Text(
                                    action.category,
                                    style: const TextStyle(color: Colors.white38, fontSize: 11),
                                  ),
                                ],
                              ),
                            ),
                            Expanded(
                              flex: 2,
                              child: InkWell(
                                onTap: () => _startCaptureKey(action),
                                borderRadius: BorderRadius.circular(6),
                                child: Container(
                                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                                  decoration: BoxDecoration(
                                    color: isEditing ? const Color(0xFF00F2FE).withValues(alpha: 0.2) : Colors.black26,
                                    border: Border.all(
                                      color: isEditing ? const Color(0xFF00F2FE) : Colors.white24,
                                      width: isEditing ? 1.5 : 1.0,
                                    ),
                                    borderRadius: BorderRadius.circular(6),
                                  ),
                                  child: Text(
                                    isEditing ? '키 입력 대기 중...' : binding.displayLabel,
                                    style: TextStyle(
                                      color: isEditing ? const Color(0xFF00F2FE) : Colors.white,
                                      fontSize: 12,
                                      fontWeight: isEditing ? FontWeight.bold : FontWeight.normal,
                                      fontFamily: 'monospace',
                                    ),
                                  ),
                                ),
                              ),
                            ),
                            const SizedBox(width: 10),
                            IconButton(
                              icon: const Icon(Icons.edit, size: 16, color: Colors.white54),
                              tooltip: '리매핑',
                              onPressed: () => _startCaptureKey(action),
                            ),
                          ],
                        ),
                      );
                    },
                  ),
                ),

                const Divider(color: Colors.white12, height: 24),

                // Footer Actions
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    OutlinedButton.icon(
                      style: OutlinedButton.styleFrom(
                        foregroundColor: Colors.white70,
                        side: const BorderSide(color: Colors.white24),
                      ),
                      icon: const Icon(Icons.refresh, size: 16),
                      label: const Text('기본값 초기화 (Reset Default)'),
                      onPressed: () {
                        notifier.resetToDefault();
                        ScaffoldMessenger.of(context).showSnackBar(
                          const SnackBar(content: Text('단축키가 프로 DAW 기본값으로 초기화되었습니다.')),
                        );
                      },
                    ),
                    ElevatedButton(
                      style: ElevatedButton.styleFrom(
                        backgroundColor: const Color(0xFF00F2FE),
                        foregroundColor: Colors.black,
                      ),
                      onPressed: () => Navigator.of(context).pop(),
                      child: const Text('완료 (Done)', style: TextStyle(fontWeight: FontWeight.bold)),
                    ),
                  ],
                ),
              ],
            ),
          ),
        );
      },
    );
  }

  void _startCaptureKey(ShortcutAction action) {
    setState(() {
      _editingAction = action;
    });

    showDialog(
      context: context,
      barrierDismissible: false,
      builder: (dialogContext) {
        return Focus(
          autofocus: true,
          onKeyEvent: (node, event) {
            if (event is KeyDownEvent) {
              final key = event.logicalKey;
              
              // Modifier keys alone are ignored
              if (key == LogicalKeyboardKey.controlLeft ||
                  key == LogicalKeyboardKey.controlRight ||
                  key == LogicalKeyboardKey.shiftLeft ||
                  key == LogicalKeyboardKey.shiftRight ||
                  key == LogicalKeyboardKey.altLeft ||
                  key == LogicalKeyboardKey.altRight ||
                  key == LogicalKeyboardKey.metaLeft ||
                  key == LogicalKeyboardKey.metaRight) {
                return KeyEventResult.handled;
              }

              final binding = CustomKeyBinding(
                key: key,
                isControl: HardwareKeyboard.instance.isControlPressed,
                isShift: HardwareKeyboard.instance.isShiftPressed,
                isAlt: HardwareKeyboard.instance.isAltPressed,
                isMeta: HardwareKeyboard.instance.isMetaPressed,
              );

              // Check conflict
              final conflict = widget.managerNotifier.checkConflict(binding, ignoreAction: action);
              Navigator.of(dialogContext).pop();

              if (conflict != null) {
                _showConflictDialog(action, conflict, binding);
              } else {
                widget.managerNotifier.updateBinding(action, binding);
                setState(() => _editingAction = null);
              }
              return KeyEventResult.handled;
            }
            return KeyEventResult.ignored;
          },
          child: AlertDialog(
            backgroundColor: const Color(0xFF12131C),
            title: const Text('새 단축키 입력', style: TextStyle(color: Colors.white, fontSize: 16)),
            content: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  '${action.label} 기능에 할당할 단축키를 눌러주세요.',
                  style: const TextStyle(color: Colors.white70, fontSize: 13),
                ),
                const SizedBox(height: 20),
                const CircularProgressIndicator(color: Color(0xFF00F2FE)),
              ],
            ),
            actions: [
              TextButton(
                onPressed: () {
                  Navigator.of(dialogContext).pop();
                  setState(() => _editingAction = null);
                },
                child: const Text('취소', style: TextStyle(color: Colors.grey)),
              ),
            ],
          ),
        );
      },
    );
  }

  void _showConflictDialog(ShortcutAction targetAction, ShortcutAction conflictAction, CustomKeyBinding binding) {
    showDialog(
      context: context,
      builder: (context) {
        return AlertDialog(
          backgroundColor: const Color(0xFF12131C),
          title: const Text('단축키 충돌 경고', style: TextStyle(color: Colors.amber, fontSize: 16)),
          content: Text(
            '입력하신 [${binding.displayLabel}] 단축키는 이미 "${conflictAction.label}" 기능에 바인딩되어 있습니다.\n\n해당 단축키로 변경하시겠습니까?',
            style: const TextStyle(color: Colors.white70, fontSize: 13),
          ),
          actions: [
            TextButton(
              onPressed: () {
                Navigator.of(context).pop();
                setState(() => _editingAction = null);
              },
              child: const Text('취소', style: TextStyle(color: Colors.grey)),
            ),
            ElevatedButton(
              style: ElevatedButton.styleFrom(backgroundColor: Colors.amber, foregroundColor: Colors.black),
              onPressed: () {
                widget.managerNotifier.updateBinding(targetAction, binding);
                Navigator.of(context).pop();
                setState(() => _editingAction = null);
              },
              child: const Text('강제 바인딩 (Overwrite)'),
            ),
          ],
        );
      },
    );
  }
}
