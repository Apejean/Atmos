import re

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'r') as f:
    content = f.read()

# Make RoomSetupWindow stateful and accept a RoomZone and a callback
content = content.replace(
    'class RoomSetupWindow extends StatefulWidget {',
    "import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';\n\nclass RoomSetupWindow extends StatefulWidget {"
)

content = content.replace(
    'final VoidCallback? onClose;',
    'final RoomZone? room;\n  final Function(RoomZone)? onApply;\n  final VoidCallback? onClose;'
)

content = content.replace(
    'const RoomSetupWindow({super.key, this.onClose});',
    'const RoomSetupWindow({super.key, this.room, this.onApply, this.onClose});'
)

# Update state variables to initialize from widget.room
content = content.replace(
    '''  // Mock Data
  double width = 6.0;
  double depth = 4.5;
  double ceilingHeight = 3.0;
  double earLevel = 1.2;''',
    '''  double width = 6.0;
  double depth = 4.5;
  double ceilingHeight = 3.0;
  double earLevel = 1.2;

  @override
  void initState() {
    super.initState();
    if (widget.room != null) {
      width = widget.room!.physicalWidth;
      depth = widget.room!.physicalHeight;
      ceilingHeight = widget.room!.ceilingHeight;
      earLevel = widget.room!.earLevel;
    }
  }

  @override
  void didUpdateWidget(RoomSetupWindow oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.room != oldWidget.room && widget.room != null) {
      setState(() {
        width = widget.room!.physicalWidth;
        depth = widget.room!.physicalHeight;
        ceilingHeight = widget.room!.ceilingHeight;
        earLevel = widget.room!.earLevel;
      });
    }
  }'''
)

# Handle Apply button
content = content.replace(
    "_buildButton(\n                  label: 'Apply',\n                  isPrimary: true,\n                  onPressed: () {},",
    "_buildButton(\n                  label: 'Apply',\n                  isPrimary: true,\n                  onPressed: () {\n                    if (widget.room != null && widget.onApply != null) {\n                      final updated = widget.room!.copyWith(\n                        physicalWidth: width,\n                        physicalHeight: depth,\n                        ceilingHeight: ceilingHeight,\n                        earLevel: earLevel,\n                      );\n                      widget.onApply!(updated);\n                    }\n                  },"
)

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'w') as f:
    f.write(content)

