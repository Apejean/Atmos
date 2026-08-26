import re

path = "lib/features/dashboard/screens/dashboard_screen.dart"
with open(path, "r") as f:
    content = f.read()

# Add window_manager import if not exists
if "package:window_manager/window_manager.dart" not in content:
    content = content.replace(
        "import 'package:file_picker/file_picker.dart';",
        "import 'package:file_picker/file_picker.dart';\nimport 'package:window_manager/window_manager.dart';"
    )

# 1. Modify the menu to add Quit option
menu_find = """              PlatformMenuItem(
                label: 'Export Log',"""
menu_replace = """              PlatformMenuItemGroup(
                members: [
                  PlatformMenuItem(
                    label: 'Quit Atmos Mixer Pro',
                    shortcut: const SingleActivator(LogicalKeyboardKey.keyQ, meta: true),
                    onSelected: () {
                      windowManager.close();
                    },
                  ),
                ],
              ),
              PlatformMenuItem(
                label: 'Export Log',"""
content = content.replace(menu_find, menu_replace)

# 2. Modify `_buildHeader` to add a Quit button
header_find = """          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: ["""
header_replace = """          Row(
            children: [
              Expanded(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: Row(
                    children: ["""
content = content.replace(header_find, header_replace)

header_close_find = """                }),
              ],
            ),
          ),
          const SizedBox(height: 16),"""
header_close_replace = """                }),
                    ],
                  ),
                ),
              ),
              // App Quit Button
              IconButton(
                onPressed: () {
                  windowManager.close();
                },
                icon: const Icon(Icons.close, color: Colors.redAccent),
                tooltip: 'Quit Application',
              ),
            ],
          ),
          const SizedBox(height: 16),"""
content = content.replace(header_close_find, header_close_replace)

with open(path, "w") as f:
    f.write(content)

