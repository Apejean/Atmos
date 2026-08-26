import re

path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
with open(path, "r") as f:
    content = f.read()

# We need to wrap InteractiveViewer with Transform to give 3D isometric perspective.
# Let's find InteractiveViewer
old_viewer = """                        child: InteractiveViewer(
                          transformationController: _transformationController,
                          minScale: 0.1,
                          maxScale: 10.0,
                          constrained: false,
                          boundaryMargin: const EdgeInsets.all(4000),
                          child: Stack(
                            clipBehavior: Clip.none,
                            children: [
                              // Grid Background
                              Positioned(
                                left: -4000,
                                top: -4000,
                                right: -4000,
                                bottom: -4000,
                                child: RepaintBoundary(
                                  child: CustomPaint(
                                    painter: _GridPainter(scale: 1.0),
                                  ),
                                ),
                              ),
                              // Origin Marker
                              const Positioned(
                                left: 0,
                                top: 0,
                                child: Icon(Icons.circle,
                                    size: 8, color: Colors.red),
                              ),"""

new_viewer = """                        child: InteractiveViewer(
                          transformationController: _transformationController,
                          minScale: 0.1,
                          maxScale: 10.0,
                          constrained: false,
                          boundaryMargin: const EdgeInsets.all(4000),
                          child: Transform(
                            alignment: Alignment.center,
                            transform: Matrix4.identity()
                              ..setEntry(3, 2, 0.001) // perspective
                              ..rotateX(-0.5)         // tilt
                              ..rotateZ(0.2),         // rotate
                            child: Stack(
                              clipBehavior: Clip.none,
                              children: [
                                // Grid Background
                                Positioned(
                                  left: -4000,
                                  top: -4000,
                                  right: -4000,
                                  bottom: -4000,
                                  child: RepaintBoundary(
                                    child: CustomPaint(
                                      painter: _GridPainter(scale: 1.0),
                                    ),
                                  ),
                                ),
                                // Origin Marker
                                const Positioned(
                                  left: 0,
                                  top: 0,
                                  child: Icon(Icons.circle,
                                      size: 8, color: Colors.red),
                                ),"""

content = content.replace(old_viewer, new_viewer)

# Also need to close the Transform widget. Where does InteractiveViewer end?
# It ends right before `if (_isSidebarOpen)`
old_viewer_end = """                              ),
                            ],
                          ),
                        ),
                      ),
                      if (_isSidebarOpen)"""

new_viewer_end = """                              ),
                            ],
                          ),
                          ), // Transform
                        ), // InteractiveViewer
                      ),
                      if (_isSidebarOpen)"""

content = content.replace(old_viewer_end, new_viewer_end)

with open(path, "w") as f:
    f.write(content)
