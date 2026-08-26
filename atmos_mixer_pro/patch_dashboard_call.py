import re

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

target = """                      final results = await rust_api.apiCalculate3DCalibration(
                        roomWidth: blueprint.canvasWidthMeters,
                        roomDepth: blueprint.canvasHeightMeters,
                        earLevel: blueprint.listeningHeightMeters,
                        speakerChannels: layout.map((s) => s.channel).toList(),
                        speakerX: layout.map((s) => s.x).toList(),
                        speakerY: layout.map((s) => s.y).toList(),
                        speakerZ: layout.map((s) => s.heightZ).toList(),
                      );"""

replacement = """                      // Updated for rust FFI signature changes (commit 5ef4c92)
                      final specs = layout.map((s) => rust_api.SpeakerPhysicalSpec(
                        channel: s.channel,
                        x: s.x,
                        y: s.y,
                        z: s.heightZ,
                        internalLatencyMs: s.dspLatencyMs,
                        lowCutHz: s.lowCutHz,
                        boundaryType: s.boundaryType,
                      )).toList();
                      
                      final results = await rust_api.apiCalculate3DCalibration(
                        roomWidth: blueprint.canvasWidthMeters,
                        roomDepth: blueprint.canvasHeightMeters,
                        earLevel: blueprint.listeningHeightMeters,
                        specs: specs,
                      );"""
                      
content = content.replace(target, replacement)
with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
