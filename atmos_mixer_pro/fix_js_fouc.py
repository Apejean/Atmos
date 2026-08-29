import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

old_js = """                      if (node.isMesh && node.geometry && node.geometry.attributes.position.count > 1000) {
                         node.scale.set(invX, invY, invZ);
                      }"""

new_js = """                      if (node.isMesh && node.geometry && node.geometry.attributes.position.count > 1000) {
                         node.scale.set(invX, invY, invZ);
                         node.updateMatrix();
                         node.updateMatrixWorld(true);
                      }"""

content = content.replace(old_js, new_js)

# Also force a re-render just in case
old_js2 = """                    });
                  }
                });"""
new_js2 = """                    });
                  }
                  // Force a re-render to apply the scale immediately
                  requestAnimationFrame(() => {
                    const oldOrbit = mv.getCameraOrbit();
                    mv.cameraOrbit = `${oldOrbit.theta}rad ${oldOrbit.phi}rad ${oldOrbit.radius}m`;
                  });
                });"""

content = content.replace(old_js2, new_js2)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
