import re

with open("lib/features/exhibition/screens/speaker_canvas_screen.dart", "r") as f:
    content = f.read()

# Remove the entire _DraggableTrajectoryPathWidget class and its state
content = re.sub(r"class _DraggableTrajectoryPathWidget extends ConsumerStatefulWidget \{[\s\S]*?\}\n\}\n", "", content)

# Remove the entire _WaypointDraggableWidget class and its state
content = re.sub(r"class _WaypointDraggableWidget extends ConsumerStatefulWidget \{[\s\S]*?\}\n\}\n", "", content)

# Remove remaining trajectory providers and refs
content = re.sub(r"\s*final trajectories = ref\.read\(trajectoryProvider\);", "", content)
content = re.sub(r"\s*'trajectory':[\s\S]*?,", "", content)
content = re.sub(r"\s*ref\.read\(trajectoryProvider\.notifier\)\.clearAll\(\);", "", content)
content = re.sub(r"\s*\.read\(trajectoryProvider\.notifier\)[\s\S]*?;", "", content)
content = re.sub(r"\s*trajectoryProvider,", "", content)
content = re.sub(r"\s*trajectory: t,", "", content)

with open("lib/features/exhibition/screens/speaker_canvas_screen.dart", "w") as f:
    f.write(content)
