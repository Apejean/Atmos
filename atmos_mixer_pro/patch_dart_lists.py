import re

with open("lib/features/dashboard/screens/dashboard_screen.dart", "r") as f:
    content = f.read()

content = content.replace("import 'dart:typed_data';\n", "")

old_block = """                  final speakerChannels = Uint64List(nodes.length);
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

new_block = """                  final speakerChannels = <BigInt>[];
                  final speakerX = <double>[];
                  final speakerY = <double>[];
                  final speakerZ = <double>[];
                  
                  for (var i = 0; i < nodes.length; i++) {
                    final node = nodes[i];
                    speakerChannels.add(BigInt.from(node.channel));
                    speakerX.add(node.x / bp.scale);
                    speakerY.add(node.y / bp.scale);
                    speakerZ.add(node.heightZ);
                  }"""

content = content.replace(old_block, new_block)

# FRB requires Uint64List and Float32List for the arguments?
# Let's check `apiCalculate3DCalibration` argument types.
# required Uint64List speakerChannels
# required List<double> speakerX

content = content.replace(
    "speakerChannels: speakerChannels,",
    "speakerChannels: Uint64List.fromList(speakerChannels.map((e) => e.toInt()).toList()),"
)

content = content.replace("final chId = res.channel + 1;", "final chId = res.channel.toInt() + 1;")

# Fix the syntax error at the end:
# I introduced a syntax error "Expected to find ']'"
