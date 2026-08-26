with open("lib/features/dashboard/screens/dashboard_screen.dart", "r") as f:
    content = f.read()

import re

old_block = """                  final speakerChannels = frb.Uint64List(nodes.length);
                  final speakerX = List<double>.filled(nodes.length, 0.0);
                  final speakerY = List<double>.filled(nodes.length, 0.0);
                  final speakerZ = List<double>.filled(nodes.length, 0.0);
                  
                  for (var i = 0; i < nodes.length; i++) {
                    final node = nodes[i];
                    speakerChannels[i] = BigInt.from(node.channel);
                    speakerX[i] = node.x / bp.scale;
                    speakerY[i] = node.y / bp.scale;
                    speakerZ[i] = node.heightZ;
                  }
                  
                  final results = rust_api.apiCalculate3DCalibration(
                    roomWidth: roomWidth,
                    roomDepth: roomDepth,
                    earLevel: earLevel,
                    speakerChannels: speakerChannels,
                    speakerX: speakerX,
                    speakerY: speakerY,
                    speakerZ: speakerZ,
                  );"""

new_block = """                  final specs = <rust_api.SpeakerPhysicalSpec>[];
                  
                  for (var i = 0; i < nodes.length; i++) {
                    final node = nodes[i];
                    specs.add(rust_api.SpeakerPhysicalSpec(
                      channel: node.channel,
                      x: node.x / bp.scale,
                      y: node.y / bp.scale,
                      z: node.heightZ,
                      internalLatencyMs: node.dspLatencyMs,
                      lowCutHz: node.lowCutHz,
                      boundaryType: node.boundaryType,
                    ));
                  }
                  
                  final results = rust_api.apiCalculate3DCalibration(
                    roomWidth: roomWidth,
                    roomDepth: roomDepth,
                    earLevel: earLevel,
                    specs: specs,
                  );"""

content = content.replace(old_block, new_block)
content = content.replace("final newEqs = List<rust_api.EqBand>.from(chModel.eqBands);", "final newEqs = List<rust_config.EqBand>.from(chModel.eqBands);")

with open("lib/features/dashboard/screens/dashboard_screen.dart", "w") as f:
    f.write(content)
