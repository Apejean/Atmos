import re

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

target = """                  for (final res in results) {
                    final chId = res.channel + 1;
                    final idx = currentRouting.indexWhere((ch) => ch.id == chId);
                    final chModel = idx != -1 ? currentRouting[idx] : OutputChannelModel(id: chId, name: 'Speaker $chId');"""

replacement = """                  for (final res in results) {
                    final chId = res.channel.toInt() + 1;
                    final idx = currentRouting.indexWhere((ch) => ch.id == chId);
                    final chModel = idx != -1 ? currentRouting[idx] : OutputChannelModel(id: chId, name: 'Speaker $chId');"""

if target in content:
    content = content.replace(target, replacement)
else:
    print("target not found")

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
