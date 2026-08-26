import re

with open("lib/features/dashboard/screens/dashboard_screen.dart", "r") as f:
    content = f.read()

old_block = """                  final speakerChannels = <BigInt>[];
                  final speakerX = <double>[];
                  final speakerY = <double>[];
                  final speakerZ = <double>[];
                  
                  for (var i = 0; i < nodes.length; i++) {
                    final node = nodes[i];
                    speakerChannels.add(BigInt.from(node.channel));
                    speakerX.add(node.x / bp.scale);
                    speakerY.add(node.y / bp.scale);
                    speakerZ.add(node.heightZ);
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
                    // We assume that node has internalLatencyMs, lowCutHz, boundaryType added by the front team,
                    // but since they might not be added yet, let's provide defaults to prevent crashes.
                    // Wait, SpeakerNode might not have them. Let's use 0.0 and "FreeSpace" as fallback if they don't exist,
                    // or just check if we can read them. If the class doesn't have it, Dart will fail to compile.
                    // Let's use dummy defaults, and front team can replace them when they update SpeakerNode.
                    specs.add(rust_api.SpeakerPhysicalSpec(
                      channel: node.channel,
                      x: node.x / bp.scale,
                      y: node.y / bp.scale,
                      z: node.heightZ,
                      internalLatencyMs: 0.0, // Replace with node.internalLatencyMs when available
                      lowCutHz: 80.0, // Replace with node.lowCutHz when available
                      boundaryType: "FreeSpace", // Replace with node.boundaryType when available
                    ));
                  }
                  
                  final results = rust_api.apiCalculate3DCalibration(
                    roomWidth: roomWidth,
                    roomDepth: roomDepth,
                    earLevel: earLevel,
                    specs: specs,
                  );"""

content = content.replace(old_block, new_block)

# Also update the part that uses the results, to handle phase_invert and eq_bands if we want.
# Actually, the user says:
# "Front의 상태 변경 이벤트가 발생할 때마다 위 계산식이 적용되어 믹서의 Output Matrix에 0-allocation으로 꽂히도록 코어 로직을 구현 및 방어하십시오."
# Does the frontend already send Phase and EQ to output routing?
# We previously had `delayMs` and `gainDb`. Now we should also send `phaseInvert` and `eqBands`.
# Let's check `routingNotifier.updateChannel` in `dashboard_screen.dart`.

with open("lib/features/dashboard/screens/dashboard_screen.dart", "w") as f:
    f.write(content)
