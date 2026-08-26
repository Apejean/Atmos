import re

with open("lib/features/dashboard/screens/dashboard_screen.dart", "r") as f:
    content = f.read()

content = content.replace("internalLatencyMs: 0.0, // Replace with node.internalLatencyMs when available", "internalLatencyMs: node.dspLatencyMs,")
content = content.replace("lowCutHz: 80.0, // Replace with node.lowCutHz when available", "lowCutHz: node.lowCutHz,")
content = content.replace('boundaryType: "FreeSpace", // Replace with node.boundaryType when available', 'boundaryType: node.boundaryType,')

# Now also update the `routingNotifier.updateChannel` to include phaseInvert and eqBands.
old_routing = """                    routingNotifier.updateChannel(
                      chModel.copyWith(
                        delayMs: res.delayMs.clamp(0.0, 500.0),
                        gainDb: res.gainDb.clamp(-60.0, 12.0),
                      )
                    );"""

new_routing = """                    routingNotifier.updateChannel(
                      chModel.copyWith(
                        delayMs: res.delayMs.clamp(0.0, 500.0),
                        gainDb: res.gainDb.clamp(-60.0, 12.0),
                        isPhaseInverted: res.phaseInvert,
                        // Note: For now we don't automatically update eqBands in OutputChannelModel 
                        // unless it's supported by the UI model, but we can pass it if we have eqBands in it.
                      )
                    );"""

content = content.replace(old_routing, new_routing)

with open("lib/features/dashboard/screens/dashboard_screen.dart", "w") as f:
    f.write(content)
