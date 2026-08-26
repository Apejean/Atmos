import re

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

content = content.replace("import 'dart:typed_data';", "import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart' as frb;")

target = """                  final speakerChannels = Uint64List(nodes.length);
                  final speakerX = Float32List(nodes.length);
                  final speakerY = Float32List(nodes.length);
                  final speakerZ = Float32List(nodes.length);
                  
                  for (var i = 0; i < nodes.length; i++) {
                    final node = nodes[i];
                    speakerChannels[i] = BigInt.from(node.channel);
                    speakerX[i] = node.x / bp.scale;
                    speakerY[i] = node.y / bp.scale;
                    speakerZ[i] = node.heightZ;
                  }"""

replacement = """                  final speakerChannels = frb.Uint64List(nodes.length);
                  final speakerX = List<double>.filled(nodes.length, 0.0);
                  final speakerY = List<double>.filled(nodes.length, 0.0);
                  final speakerZ = List<double>.filled(nodes.length, 0.0);
                  
                  for (var i = 0; i < nodes.length; i++) {
                    final node = nodes[i];
                    speakerChannels[i] = BigInt.from(node.channel);
                    speakerX[i] = node.x / bp.scale;
                    speakerY[i] = node.y / bp.scale;
                    speakerZ[i] = node.heightZ;
                  }"""

if target in content:
    content = content.replace(target, replacement)
else:
    # try replacing without BigInt
    target2 = target.replace("BigInt.from(node.channel)", "node.channel")
    if target2 in content:
        content = content.replace(target2, replacement)
        
with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
