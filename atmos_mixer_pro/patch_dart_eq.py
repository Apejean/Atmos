import re

with open("lib/features/dashboard/screens/dashboard_screen.dart", "r") as f:
    content = f.read()

old_routing = """                    routingNotifier.updateChannel(
                      chModel.copyWith(
                        delayMs: res.delayMs.clamp(0.0, 500.0),
                        gainDb: res.gainDb.clamp(-60.0, 12.0),
                        isPhaseInverted: res.phaseInvert,
                        // Note: For now we don't automatically update eqBands in OutputChannelModel 
                        // unless it's supported by the UI model, but we can pass it if we have eqBands in it.
                      )
                    );"""

new_routing = """                    final newEqs = List<rust_api.EqBand>.from(chModel.eqBands);
                    if (res.eqBands.isNotEmpty) {
                      newEqs[0] = res.eqBands[0]; // Override Band 1 with Boundary EQ
                    }
                    
                    routingNotifier.updateChannel(
                      chModel.copyWith(
                        delayMs: res.delayMs.clamp(0.0, 500.0),
                        gainDb: res.gainDb.clamp(-60.0, 12.0),
                        isPhaseInverted: res.phaseInvert,
                        eqBands: newEqs,
                      )
                    );"""

content = content.replace(old_routing, new_routing)

with open("lib/features/dashboard/screens/dashboard_screen.dart", "w") as f:
    f.write(content)
