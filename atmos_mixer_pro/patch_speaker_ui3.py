import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Add Inspector to Stack
stack_addition = """                  // Right Sidebar Inspector
                  AnimatedPositioned(
                    duration: const Duration(milliseconds: 300),
                    curve: Curves.easeOutCubic,
                    right: _inspectorSpeakerId != null ? 0 : -320,
                    top: 0,
                    bottom: 0,
                    width: 320,
                    child: _inspectorSpeakerId != null 
                      ? Consumer(builder: (context, ref, _) {
                          final node = ref.watch(speakerLayoutProvider).where((n) => n.id == _inspectorSpeakerId).firstOrNull;
                          if (node == null) return const SizedBox.shrink();
                          return SpeakerInspectorPanel(
                            node: node,
                            onClose: () => setState(() => _inspectorSpeakerId = null),
                            onSync: _syncSpatialConfigRealtime,
                          );
                        })
                      : const SizedBox.shrink(),
                  ),
                  // Add Speaker Button in Canvas
                  Positioned(
                    bottom: 16,
                    left: 16,
                    child: ElevatedButton.icon(
                      style: ElevatedButton.styleFrom(
                        backgroundColor: AppColors.primaryNeon,
                        foregroundColor: Colors.black,
                        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
                      ),
                      icon: const Icon(Icons.add_circle_outline),
                      label: const Text('Add Speaker', style: TextStyle(fontWeight: FontWeight.bold)),
                      onPressed: () {
                        _addSpeaker();
                        // Get the newly added speaker ID to open inspector
                        final nodes = ref.read(speakerLayoutProvider);
                        if (nodes.isNotEmpty) {
                          setState(() {
                            _inspectorSpeakerId = nodes.last.id;
                          });
                        }
                      },
                    ),
                  ),
                  const Positioned.fill(child: TrajectoryEditorToolbar()),"""

content = content.replace("                const Positioned.fill(child: TrajectoryEditorToolbar()),", stack_addition)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
