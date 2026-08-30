import re

def main():
    path = "lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart"
    with open(path, "r") as f:
        content = f.read()
    
    old_code = """    _speakerMovedSub = ref.read(threeJsEngineProvider).onSpeakerMoved.listen((data) {
      final String id = data['id'];
      final double x = data['x'];
      final double y = data['y'];"""
      
    new_code = """    _speakerMovedSub = ref.read(threeJsEngineProvider).onSpeakerMoved.listen((data) {
      final String id = data['id'];
      final double x = (data['x'] as num).toDouble();
      final double y = (data['y'] as num).toDouble();"""
      
    content = content.replace(old_code, new_code)
    
    with open(path, "w") as f:
        f.write(content)

main()
