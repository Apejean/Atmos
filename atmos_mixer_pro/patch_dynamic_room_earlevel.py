import re

def main():
    path = "lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart"
    with open(path, "r") as f:
        content = f.read()

    # Need to trigger setEarLevel when activeRoom.earLevel changes
    # Probably inside didUpdateWidget
    old_didUpdate = """  @override
  void didUpdateWidget(covariant Dynamic3DRoom oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.activeRoom?.id != oldWidget.activeRoom?.id) {
      _syncToWeb();
    } else if (widget.selectedSpeakerId != oldWidget.selectedSpeakerId) {
      _syncToWeb();
    } else if (widget.activeRoom?.physicalWidth != oldWidget.activeRoom?.physicalWidth ||
               widget.activeRoom?.physicalHeight != oldWidget.activeRoom?.physicalHeight ||
               widget.activeRoom?.ceilingHeight != oldWidget.activeRoom?.ceilingHeight) {
      _syncToWeb();
    }
  }"""

    new_didUpdate = """  @override
  void didUpdateWidget(covariant Dynamic3DRoom oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.activeRoom?.id != oldWidget.activeRoom?.id) {
      _syncToWeb();
    } else if (widget.selectedSpeakerId != oldWidget.selectedSpeakerId) {
      _syncToWeb();
    } else if (widget.activeRoom?.physicalWidth != oldWidget.activeRoom?.physicalWidth ||
               widget.activeRoom?.physicalHeight != oldWidget.activeRoom?.physicalHeight ||
               widget.activeRoom?.ceilingHeight != oldWidget.activeRoom?.ceilingHeight) {
      _syncToWeb();
    }
    
    if (widget.activeRoom?.earLevel != oldWidget.activeRoom?.earLevel) {
      if (widget.activeRoom != null) {
        ref.read(threeJsEngineProvider).setEarLevel(widget.activeRoom!.earLevel);
      }
    }
  }"""

    content = content.replace(old_didUpdate, new_didUpdate)

    # Also sync it initially in build or after loading
    old_sync = """  void _syncToWeb() {
    if (!ref.read(threeJsEngineProvider).isEngineReady) return;"""
    
    new_sync = """  void _syncToWeb() {
    if (!ref.read(threeJsEngineProvider).isEngineReady) return;
    
    if (widget.activeRoom != null) {
      ref.read(threeJsEngineProvider).setEarLevel(widget.activeRoom!.earLevel);
    }"""
    
    content = content.replace(old_sync, new_sync)

    with open(path, "w") as f:
        f.write(content)

main()
