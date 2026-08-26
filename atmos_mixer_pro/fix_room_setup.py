import re

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'r') as f:
    content = f.read()

target = """    if (!_isVisible) return const SizedBox.shrink();
    final roomState = ref.watch(roomZoneProvider);
    if (roomState == null) return const SizedBox.shrink();

    return Positioned("""

replacement = """    if (!_isVisible) return const SizedBox.shrink();
    final blueprintState = ref.watch(blueprintProvider);
    
    return Positioned("""

if target in content:
    content = content.replace(target, replacement)
else:
    print("target1 not found")

target2 = """                    _buildRow('Width:', '${roomState.physicalWidth.toStringAsFixed(1)} m'),
                    const SizedBox(height: 12),
                    _buildRow('Depth:', '${roomState.physicalHeight.toStringAsFixed(1)} m'),"""

replacement2 = """                    _buildRow('Width:', '${blueprintState.canvasWidthMeters.toStringAsFixed(1)} m'),
                    const SizedBox(height: 12),
                    _buildRow('Depth:', '${blueprintState.canvasHeightMeters.toStringAsFixed(1)} m'),"""

if target2 in content:
    content = content.replace(target2, replacement2)
else:
    print("target2 not found")
    
with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'w') as f:
    f.write(content)
