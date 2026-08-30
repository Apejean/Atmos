import re

def main():
    path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
    with open(path, "r") as f:
        content = f.read()

    old_apply = """                onApply: (updated) {
                  final bp = ref.read(blueprintProvider);
                  final pixelW = updated.physicalWidth * bp.scale;
                  final pixelH = updated.physicalHeight * bp.scale;
                  final finalUpdated = updated.copyWith(width: pixelW, height: pixelH);

                  ref.read(roomZoneProvider.notifier).updateRoomZone(finalUpdated, immediate: true);
                  ref.read(blueprintProvider.notifier).setCanvasDimensions(
                        finalUpdated.physicalWidth,
                        finalUpdated.physicalHeight,
                      );
                  setState(() {
                    _isRoomSetupOpen = false;
                  });"""

    new_apply = """                onApply: (updated) {
                  final bp = ref.read(blueprintProvider);
                  final pixelW = updated.physicalWidth * bp.scale;
                  final pixelH = updated.physicalHeight * bp.scale;
                  final finalUpdated = updated.copyWith(width: pixelW, height: pixelH);

                  // Scale speakers to maintain relative position
                  if (activeRoom != null && activeRoom.physicalWidth > 0 && activeRoom.physicalHeight > 0) {
                    final double scaleX = updated.physicalWidth / activeRoom.physicalWidth;
                    final double scaleY = updated.physicalHeight / activeRoom.physicalHeight;
                    
                    final nodes = ref.read(speakerLayoutProvider);
                    for (final node in nodes) {
                      if (node.roomId == activeRoom.id || node.roomId == null) {
                        final newX = (node.x * scaleX).clamp(0.0, updated.physicalWidth);
                        final newY = (node.y * scaleY).clamp(0.0, updated.physicalHeight);
                        ref.read(speakerLayoutProvider.notifier).updateSpeaker(node.copyWith(x: newX, y: newY), immediate: true);
                      }
                    }
                  }

                  ref.read(roomZoneProvider.notifier).updateRoomZone(finalUpdated, immediate: true);
                  ref.read(blueprintProvider.notifier).setCanvasDimensions(
                        finalUpdated.physicalWidth,
                        finalUpdated.physicalHeight,
                      );
                  setState(() {
                    _isRoomSetupOpen = false;
                  });"""

    content = content.replace(old_apply, new_apply)

    with open(path, "w") as f:
        f.write(content)

main()
