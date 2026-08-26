import re
path = "lib/features/dashboard/widgets/track_card.dart"
with open(path, "r") as f:
    content = f.read()

# Make sure ObjectPannerModal is imported
if "import 'package:atmos_mixer_pro/features/dashboard/widgets/object_panner_modal.dart';" not in content:
    content = content.replace("import 'package:flutter/material.dart';", "import 'package:flutter/material.dart';\nimport 'package:atmos_mixer_pro/features/dashboard/widgets/object_panner_modal.dart';")

# Find the Trajectory button and add the Panner button next to it
find_btn = """                        ElevatedButton(
                          style: ElevatedButton.styleFrom(backgroundColor: widget.accentColor, padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 0)),
                          onPressed: () {
                            showDialog(
                              context: context,
                              builder: (context) => TrajectorySettingsModal(trackId: widget.track.id),
                            );
                          },
                          child: const Text('3D Trajectory', style: TextStyle(color: Colors.black, fontSize: 10)),
                        )"""

replace_btn = """                        ElevatedButton(
                          style: ElevatedButton.styleFrom(backgroundColor: widget.accentColor, padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 0)),
                          onPressed: () {
                            showDialog(
                              context: context,
                              builder: (context) => ObjectPannerModal(trackId: widget.track.id),
                            );
                          },
                          child: const Text('3D Panner', style: TextStyle(color: Colors.black, fontSize: 10)),
                        ),
                        const SizedBox(width: 4),
                        ElevatedButton(
                          style: ElevatedButton.styleFrom(backgroundColor: Colors.white24, padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 0)),
                          onPressed: () {
                            showDialog(
                              context: context,
                              builder: (context) => TrajectorySettingsModal(trackId: widget.track.id),
                            );
                          },
                          child: const Text('Auto-Move', style: TextStyle(color: Colors.white, fontSize: 10)),
                        )"""

content = content.replace(find_btn, replace_btn)

with open(path, "w") as f:
    f.write(content)
