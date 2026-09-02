import os

path = 'assets/3d_simulator/studio_engine.html'
with open(path, 'r') as f:
    content = f.read()

# Inject console logs into animate loop to debug targeting
debug_code = """
          if (isHit) {
"""

new_debug_code = """
          // Throttled logging for debugging
          if (!spk.userData.lastLog || Date.now() - spk.userData.lastLog > 1000) {
            console.log(`[Target Debug] SPK ${speakerData?.index} -> Ear:`, {
               spkWorldPos: spkWorldPos,
               targetEarPos: targetEarPos,
               angleDeg: angleDeg,
               distToEar: distToEar,
               spreadAtEar: spreadAtEar,
               isHit: isHit,
               yaw: spk.rotation.y * 180 / Math.PI,
               pitch: spk.rotation.x * 180 / Math.PI
            });
            spk.userData.lastLog = Date.now();
          }
          if (isHit) {
"""

content = content.replace("          if (isHit) {", new_debug_code)

with open(path, 'w') as f:
    f.write(content)

print("Debug patched.")
