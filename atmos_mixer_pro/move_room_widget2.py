import re
import os

dest_file = 'lib/features/exhibition/widgets/room_zone_widget.dart'
src_file = 'lib/features/exhibition/screens/speaker_canvas_screen.dart'

with open(dest_file, 'r') as f:
    content = f.read()

# Fix imports
content = content.replace(
    "import 'package:atmos_mixer_pro/features/exhibition/providers/speaker_layout_provider.dart';",
    "import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';"
)
content = content.replace(
    "import 'package:atmos_mixer_pro/features/exhibition/providers/room_zone_provider.dart';",
    "import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';"
)
content = content.replace(
    "import 'package:atmos_mixer_pro/features/exhibition/providers/blueprint_provider.dart';",
    "import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';"
)

# Replace _canvasWidth with widget.canvasWidth, etc
content = content.replace('_canvasWidth', 'widget.canvasWidth')
content = content.replace('_canvasHeight', 'widget.canvasHeight')
content = content.replace('_speakerSize', 'widget.speakerSize')

# Add these properties to the widget class
class_def = """class RoomZoneWidget extends ConsumerStatefulWidget {
  final RoomZone room;
  final List<SpeakerNode> containedSpeakers;
  final VoidCallback? onEdit;
  final VoidCallback? onDragUpdate;
  final TransformationController transformationController;
"""

new_class_def = """class RoomZoneWidget extends ConsumerStatefulWidget {
  final RoomZone room;
  final List<SpeakerNode> containedSpeakers;
  final VoidCallback? onEdit;
  final VoidCallback? onDragUpdate;
  final TransformationController transformationController;
  final double canvasWidth;
  final double canvasHeight;
  final double speakerSize;
"""

content = content.replace(class_def, new_class_def)

constructor = """  const RoomZoneWidget({
    super.key,
    required this.room,
    required this.containedSpeakers,
    required this.transformationController,
    this.onEdit,
    this.onDragUpdate,
  });"""

new_constructor = """  const RoomZoneWidget({
    super.key,
    required this.room,
    required this.containedSpeakers,
    required this.transformationController,
    required this.canvasWidth,
    required this.canvasHeight,
    required this.speakerSize,
    this.onEdit,
    this.onDragUpdate,
  });"""

content = content.replace(constructor, new_constructor)

with open(dest_file, 'w') as f:
    f.write(content)

# Now we must update speaker_canvas_screen.dart to pass these parameters!
with open(src_file, 'r') as f:
    src_content = f.read()

src_content = src_content.replace(
    "RoomZoneWidget(\n                                  room: room,\n                                  containedSpeakers: containedSpeakers,\n                                  transformationController: _transformationController,\n                                  onEdit: () => _editRoom(room),\n                                  onDragUpdate: () => setState(() {}),\n                                )",
    "RoomZoneWidget(\n                                  room: room,\n                                  containedSpeakers: containedSpeakers,\n                                  transformationController: _transformationController,\n                                  canvasWidth: _canvasWidth,\n                                  canvasHeight: _canvasHeight,\n                                  speakerSize: _speakerSize,\n                                  onEdit: () => _editRoom(room),\n                                  onDragUpdate: () => setState(() {}),\n                                )"
)

# And fix any occurrences where it was passed on a single line or something
src_content = re.sub(
    r'RoomZoneWidget\(\s*room:\s*room,\s*containedSpeakers:\s*containedSpeakers,\s*transformationController:\s*_transformationController,\s*onEdit:\s*\(\)\s*=>\s*_editRoom\(room\),\s*onDragUpdate:\s*\(\)\s*=>\s*setState\(\(\)\s*\{\}\),\s*\)',
    r'RoomZoneWidget(\n                                  room: room,\n                                  containedSpeakers: containedSpeakers,\n                                  transformationController: _transformationController,\n                                  canvasWidth: _canvasWidth,\n                                  canvasHeight: _canvasHeight,\n                                  speakerSize: _speakerSize,\n                                  onEdit: () => _editRoom(room),\n                                  onDragUpdate: () => setState(() {}),\n                                )',
    src_content
)

with open(src_file, 'w') as f:
    f.write(src_content)

