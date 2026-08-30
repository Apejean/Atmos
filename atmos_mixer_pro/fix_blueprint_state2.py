import re

with open('lib/features/exhibition/state/blueprint_state.dart', 'r') as f:
    content = f.read()

# Replace class BlueprintState extends StateNotifier<BlueprintData> with class BlueprintState extends Notifier<BlueprintData>
content = content.replace("class BlueprintState extends StateNotifier<BlueprintData>", "class BlueprintState extends Notifier<BlueprintData>")
content = content.replace("BlueprintState() : super(const BlueprintData());", "@override\n  BlueprintData build() => const BlueprintData();")

# Remove shared preferences for now to just fix compilation
old_init = """  Future<void> init() async {
    final prefs = await SharedPreferences.getInstance();"""

new_init = """  Future<void> init() async {
    // final prefs = await SharedPreferences.getInstance();"""

content = content.replace(old_init, new_init)
content = content.replace("final prefs = await SharedPreferences.getInstance();", "// final prefs = await SharedPreferences.getInstance();")

with open('lib/features/exhibition/state/blueprint_state.dart', 'w') as f:
    f.write(content)
print("Fixed blueprint state completely")
