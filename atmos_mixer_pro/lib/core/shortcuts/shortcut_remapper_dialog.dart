import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/shortcuts/shortcut_config.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';

class ShortcutRemapperDialog extends StatelessWidget {
  final ShortcutManagerConfig manager;

  const ShortcutRemapperDialog({super.key, required this.manager});

  static Future<void> show(BuildContext context, ShortcutManagerConfig manager) async {
    await showDialog(
      context: context,
      builder: (context) => ShortcutRemapperDialog(manager: manager),
    );
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      backgroundColor: const Color(0xFF1E1E28),
      title: const Row(
        children: [
          Icon(Icons.keyboard, color: AppColors.primaryNeon),
          SizedBox(width: 8),
          Text('단축키 설정 (Keybindings)', style: TextStyle(color: Colors.white, fontSize: 16)),
        ],
      ),
      content: SizedBox(
        width: 450,
        child: ListenableBuilder(
          listenable: manager,
          builder: (context, _) {
            final items = manager.shortcuts.values.toList();
            return ListView.separated(
              shrinkWrap: true,
              itemCount: items.length,
              separatorBuilder: (context, index) => const Divider(color: Colors.white12),
              itemBuilder: (context, index) {
                final item = items[index];
                return Material(color: Colors.transparent, child: ListTile(
                  dense: true,
                  title: Text(item.name, style: const TextStyle(color: Colors.white, fontSize: 13)),
                  subtitle: Text(item.category, style: const TextStyle(color: Colors.white38, fontSize: 11)),
                  trailing: Container(
                    padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                    decoration: BoxDecoration(
                      color: AppColors.primaryNeon.withValues(alpha: 0.1),
                      border: Border.all(color: AppColors.primaryNeon),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      item.label,
                      style: const TextStyle(color: AppColors.primaryNeon, fontSize: 12, fontWeight: FontWeight.bold),
                    ),
                  ),
                ),);
              },
            );
          },
        ),
      ),
      actions: [
        TextButton(
          onPressed: () {
            manager.resetAll();
          },
          child: const Text('초기화', style: TextStyle(color: Colors.white54)),
        ),
        ElevatedButton(
          style: ElevatedButton.styleFrom(
            backgroundColor: AppColors.primaryNeon,
            foregroundColor: Colors.black,
          ),
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('확인', style: TextStyle(fontWeight: FontWeight.bold)),
        ),
      ],
    );
  }
}
