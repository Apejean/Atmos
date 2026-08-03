import re

src_file = 'lib/features/exhibition/screens/speaker_canvas_screen.dart'
dest_file = 'lib/features/exhibition/widgets/room_zone_widget.dart'

with open(src_file, 'r') as f:
    content = f.read()

# We need to extract _DraggableRoomWidget and _CadDoorPainter.
# Let's find the start of _DraggableRoomWidget.
room_start = content.find('class _DraggableRoomWidget extends ConsumerStatefulWidget {')
# It ends right before _DraggableSpeakerWidget
speaker_start = content.find('class _DraggableSpeakerWidget extends ConsumerStatefulWidget {')
room_code = content[room_start:speaker_start]

# Let's find _CadDoorPainter
door_painter_start = content.find('class _CadDoorPainter extends CustomPainter {')
measurement_painter_start = content.find('class _MeasurementPainter extends CustomPainter {')
door_painter_code = content[door_painter_start:measurement_painter_start]

# We need to replace them with imports in speaker_canvas_screen.dart
new_content = content[:room_start] + content[speaker_start:door_painter_start] + content[measurement_painter_start:]

# Change _DraggableRoomWidget to RoomZoneWidget
room_code = room_code.replace('_DraggableRoomWidget', 'RoomZoneWidget')
room_code = room_code.replace('_CadDoorPainter', 'CadDoorPainter')
door_painter_code = door_painter_code.replace('_CadDoorPainter', 'CadDoorPainter')

new_content = new_content.replace('_DraggableRoomWidget', 'RoomZoneWidget')

# Create room_zone_widget.dart
imports = """import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/features/exhibition/providers/speaker_layout_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/providers/room_zone_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/providers/blueprint_provider.dart';
import 'package:atmos_mixer_pro/src/rust/common/models.dart';

"""

with open(dest_file, 'w') as f:
    f.write(imports + room_code + '\n' + door_painter_code)

# Insert import in speaker_canvas_screen.dart
import_statement = "import 'package:atmos_mixer_pro/features/exhibition/widgets/room_zone_widget.dart';\n"
new_content = import_statement + new_content

with open(src_file, 'w') as f:
    f.write(new_content)

