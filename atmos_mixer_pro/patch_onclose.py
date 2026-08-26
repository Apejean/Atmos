import re

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'r') as f:
    content = f.read()

target = """class SpeakerInspectorPanel extends ConsumerStatefulWidget {
  final SpeakerNode? selectedSpeaker;

  const SpeakerInspectorPanel({super.key, required this.selectedSpeaker});"""
  
replacement = """class SpeakerInspectorPanel extends ConsumerStatefulWidget {
  final SpeakerNode? selectedSpeaker;
  final VoidCallback? onClose;

  const SpeakerInspectorPanel({super.key, required this.selectedSpeaker, this.onClose});"""
  
content = content.replace(target, replacement)
with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'w') as f:
    f.write(content)
