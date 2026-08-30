import re

def main():
    path = "lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart"
    with open(path, "r") as f:
        content = f.read()

    # The actual sync function is _syncSceneData
    old_sync = """  void _syncSceneData() {
    final engine = ref.read(threeJsEngineProvider);
    if (!engine.isEngineReady) return;"""

    new_sync = """  void _syncSceneData() {
    final engine = ref.read(threeJsEngineProvider);
    if (!engine.isEngineReady) return;
    
    if (widget.activeRoom != null) {
      engine.setEarLevel(widget.activeRoom!.earLevel);
    }"""

    content = content.replace(old_sync, new_sync)

    with open(path, "w") as f:
        f.write(content)

main()
