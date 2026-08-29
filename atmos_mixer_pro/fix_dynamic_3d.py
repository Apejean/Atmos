import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# Add dart:math import if not present
if "import 'dart:math'" not in content:
    content = content.replace("import 'package:flutter/material.dart';", "import 'package:flutter/material.dart';\nimport 'dart:math' as math;")

# Find the build method and modify cameraOrbit calculation
build_start = content.find('  Widget build(BuildContext context) {')
# We need to insert the maxDim and orbitDist logic
injection = """
    final roomWidth = widget.activeRoom?.physicalWidth ?? bp.canvasWidthMeters;
    final roomDepth = widget.activeRoom?.physicalHeight ?? bp.canvasHeightMeters;
    final roomHeight = widget.activeRoom?.ceilingHeight ?? 3.0;
    final roomLabel = widget.activeRoom?.label ?? 'Room 1';

    final maxDim = math.max(roomWidth, roomDepth);
    final orbitDist = (maxDim * 1.5).toStringAsFixed(1);
"""

# Replace the existing variable declarations with the new one
old_vars = """    final roomWidth = widget.activeRoom?.physicalWidth ?? bp.canvasWidthMeters;
    final roomDepth = widget.activeRoom?.physicalHeight ?? bp.canvasHeightMeters;
    final roomHeight = widget.activeRoom?.ceilingHeight ?? 3.0;
    final roomLabel = widget.activeRoom?.label ?? 'Room 1';"""

content = content.replace(old_vars, injection)

# Update the cameraOrbit property
content = content.replace("cameraOrbit: '45deg 65deg 6.5m',", "cameraOrbit: '45deg 65deg ${orbitDist}m',")

# Update relatedJs to apply inverse scale to mannequin
related_js = """
              relatedJs: '''
                document.querySelector('model-viewer').addEventListener('load', function(e) {
                  const mv = e.target;
                  const sceneSymbol = Object.getOwnPropertySymbols(mv).find(s => s.description === 'scene');
                  if(sceneSymbol) {
                    const scene = mv[sceneSymbol];
                    const scaleStr = mv.getAttribute('scale') || '1 1 1';
                    const scaleParts = scaleStr.split(' ').map(parseFloat);
                    const invX = 1.0 / scaleParts[0];
                    const invY = 1.0 / scaleParts[1];
                    const invZ = 1.0 / scaleParts[2];
                    scene.traverse((node) => {
                      if (node.isMesh && node.geometry && node.geometry.attributes.position.count > 1000) {
                         node.scale.set(invX, invY, invZ);
                      }
                    });
                  }
                });
              ''',
"""
# Inject relatedJs after interactionPrompt: InteractionPrompt.none,
content = content.replace("interactionPrompt: InteractionPrompt.none,", "interactionPrompt: InteractionPrompt.none,\n" + related_js)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
