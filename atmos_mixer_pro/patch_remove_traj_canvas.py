import re

with open("lib/features/exhibition/screens/speaker_canvas_screen.dart", "r") as f:
    content = f.read()

# 1. Remove Trajectory Provider import
content = re.sub(r"import 'package:atmos_mixer_pro/features/exhibition/state/trajectory_state\.dart';\n", "", content)
content = re.sub(r"import 'package:atmos_mixer_pro/features/exhibition/models/trajectory\.dart';\n", "", content)
content = re.sub(r"import 'package:atmos_mixer_pro/features/exhibition/widgets/trajectory_layer_painter\.dart';\n", "", content)
content = re.sub(r"import 'package:atmos_mixer_pro/features/exhibition/widgets/trajectory_sidebar_widget\.dart';\n", "", content)
content = re.sub(r"import 'package:atmos_mixer_pro/features/exhibition/widgets/trajectory_editor_toolbar\.dart';\n", "", content)

# 2. Remove boolean variables related to trajectory
content = re.sub(r"\s*bool _showTrajectories = true;", "", content)
content = re.sub(r"\s*bool _isPlayingAutomation = false;", "", content)

# 3. Remove _toggleAutomation
content = re.sub(r"\s*void _toggleAutomation\(\) \{[\s\S]*?\}\n\n", "\n", content)

# 4. Remove _editTrajectory
content = re.sub(r"\s*Future<void> _editTrajectory\(TrajectoryModel trajectory\) async \{[\s\S]*?\}\n\n", "\n", content)

# 5. Remove _animationTimer
content = re.sub(r"\s*Timer\? _animationTimer;", "", content)

# 6. Inside build, remove trajectory layer rendering
layer_rendering = r"""\s*if \(_showTrajectories\) \.\.\.\[\n\s*Consumer\(\n\s*builder: \(context, ref, child\) \{\n\s*final trajectories = ref\.watch\(trajectoryProvider\);\n\s*return Stack\(\n\s*children: \[\n\s*Positioned\.fill\(\n\s*child: CustomPaint\(\n\s*painter: TrajectoryLayerPainter\(\n\s*trajectories: trajectories,\n\s*focusedTrajectoryId: ref\.watch\(activeTrajectoryIdProvider\),\n\s*\),\n\s*\),\n\s*\),\n\s*for \(var t in trajectories\)\n\s*_DraggableTrajectoryPathWidget\(\n\s*trajectory: t,\n\s*onLongPress: \(\) => _editTrajectory\(t\),\n\s*\),\n\s*\],\n\s*\);\n\s*\},\n\s*\),\n\s*\],"""
content = re.sub(layer_rendering, "", content)

# 7. Remove trajectory sidebar
sidebar = r"""\s*if \(_showTrajectories\)\n\s*const Positioned\(\n\s*right: 16,\n\s*top: 80,\n\s*bottom: 16,\n\s*child: TrajectorySidebarWidget\(\),\n\s*\),"""
content = re.sub(sidebar, "", content)

# 8. Remove trajectory toolbar
toolbar = r"""\s*if \(_showTrajectories\)\n\s*const Positioned\.fill\(child: TrajectoryEditorToolbar\(\)\),"""
content = re.sub(toolbar, "", content)


with open("lib/features/exhibition/screens/speaker_canvas_screen.dart", "w") as f:
    f.write(content)
