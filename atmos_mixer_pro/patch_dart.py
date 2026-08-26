import re

with open("lib/features/dashboard/screens/dashboard_screen.dart", "r") as f:
    content = f.read()

old_block = """                  final speakerChannels = Uint64List(nodes.length);
                  final speakerX = Float32List(nodes.length);
                  final speakerY = Float32List(nodes.length);
                  final speakerZ = Float32List(nodes.length);
                  
                  for (var i = 0; i < nodes.length; i++) {
                    final node = nodes[i];
                    speakerChannels[i] = node.channel;
                    speakerX[i] = node.x / bp.scale;
                    speakerY[i] = node.y / bp.scale;
                    speakerZ[i] = node.heightZ;
                  }"""

new_block = """                  final speakerChannels = Uint64List(nodes.length);
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

content = content.replace(old_block, new_block)

with open("lib/features/dashboard/screens/dashboard_screen.dart", "w") as f:
    f.write(content)
