import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

target = """                                    // Speakers
                                    Consumer(
                                      builder: (context, ref, child) {
                                        final nodes = ref.watch(speakerLayoutProvider);
                                        return Stack(
                                          children: nodes.map((node) {
                                            return Positioned(
                                              left: node.x * blueprint.scale - 30, // Offset for center
                                              top: node.y * blueprint.scale - 30,
                                              child: GestureDetector(
                                                onTap: () => _onSpeakerTap(node.id),
                                                child: _SciFiSpeakerWidget(
                                                  node: node,
                                                  isSelected: _inspectorSpeakerId == node.id,
                                                  neonCyan: neonCyan,
                                                ),
                                              ),
                                            );
                                          }).toList(),
                                        );
                                      },
                                    ),"""

replacement = """                                    // Speakers
                                    Consumer(
                                      builder: (context, ref, child) {
                                        final nodes = ref.watch(speakerLayoutProvider);
                                        
                                        // Use same projection for speakers
                                        final projector = IsoProjector(
                                          scale: blueprint.scale,
                                          cx: 400, // Container is 800x600, so center is 400, 300+100
                                          cy: 400,
                                          roomW: blueprint.canvasWidthMeters,
                                          roomD: blueprint.canvasHeightMeters,
                                        );
                                        
                                        return Stack(
                                          children: nodes.map((node) {
                                            final pos = projector.project(node.x, node.y, node.heightZ);
                                            return Positioned(
                                              left: pos.dx - 35, // 70x70 widget
                                              top: pos.dy - 35,
                                              child: GestureDetector(
                                                onTap: () => _onSpeakerTap(node.id),
                                                child: _SciFiSpeakerWidget(
                                                  node: node,
                                                  isSelected: _inspectorSpeakerId == node.id,
                                                  neonCyan: neonCyan,
                                                ),
                                              ),
                                            );
                                          }).toList(),
                                        );
                                      },
                                    ),"""
content = content.replace(target, replacement)

# Center Listener pos
target_listener = """                                    // Center Listener
                                    Positioned(
                                      left: 400 - 24, // Assuming 800x600 center
                                      top: 300 - 24,
                                      child: Container("""

replacement_listener = """                                    // Center Listener
                                    Positioned(
                                      left: 400 - 24, 
                                      top: 400 - 24, // cy is 400 in projector
                                      child: Container("""
content = content.replace(target_listener, replacement_listener)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
