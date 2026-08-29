import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# Let's replace the Dummy Head with ModelViewer!
# But wait, ModelViewer needs `import 'package:model_viewer_plus/model_viewer_plus.dart';`
import_statement = "import 'package:model_viewer_plus/model_viewer_plus.dart';"
if import_statement not in content:
    content = content.replace("import 'package:vector_math/vector_math_64.dart' as vector;", 
                              "import 'package:vector_math/vector_math_64.dart' as vector;\nimport 'package:model_viewer_plus/model_viewer_plus.dart';")

# Find the head part
#     // 4. Dummy Head (Listener) at center, ear level 1.2m
#     final headY = roomH / 2 - (1.2 * ppm);
#     objects.add(SceneObject(
# ...
#       ));
#     }

head_pattern = r"// 4\. Dummy Head.*?\}\)\);"

head_replacement = """// 4. Head Model (Listener)
    final headY = roomH / 2 - (1.2 * ppm);
    objects.add(SceneObject(
      position: vector.Vector3(0, headY, 0),
      child: Transform.translate(
        offset: Offset(0, headY),
        child: SizedBox(
          width: 300,
          height: 300,
          child: IgnorePointer(
            child: ModelViewer(
              src: 'assets/models/listener_head.glb',
              alt: '3D Listener Head',
              autoRotate: false,
              cameraControls: false,
              disableZoom: true,
              disablePan: true,
              // We sync the ModelViewer orbit with our Flutter matrix rotation
              cameraOrbit: '${_yaw * 180 / math.pi}deg ${90 + _pitch * 180 / math.pi}deg 105%',
              interactionPrompt: InteractionPrompt.none,
            ),
          ),
        ),
      ),
    ));"""

content = re.sub(head_pattern, head_replacement, content, flags=re.DOTALL)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)

