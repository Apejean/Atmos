import re

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

# Import OutputCalibrationModal, output_routing_state, blueprint_state, speaker_layout_state, and dart:math
imports = """import 'package:atmos_mixer_pro/features/dashboard/widgets/output_calibration_modal.dart';
import 'package:atmos_mixer_pro/features/dashboard/state/output_routing_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'dart:math' as math;
"""

content = content.replace("import 'package:file_picker/file_picker.dart';", "import 'package:file_picker/file_picker.dart';\n" + imports)

# We want to add the button near `const TuningModal()` or Speaker Layout
target = """              IconButton(
                icon: const Icon(Icons.grid_on, color: Colors.white),"""

new_buttons = """              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.primaryNeon,
                  foregroundColor: Colors.black,
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(4)),
                ),
                onPressed: () {
                  final nodes = ref.read(speakerLayoutProvider);
                  final bp = ref.read(blueprintProvider);
                  if (nodes.isEmpty) return;
                  
                  final listenerX = (bp.canvasWidthMeters * bp.scale) / 2;
                  final listenerY = (bp.canvasHeightMeters * bp.scale) / 2;
                  final listenerZ = bp.listeningHeightMeters;

                  final distances = <String, double>{};
                  double maxDist = 0.0;

                  for (final node in nodes) {
                    final dx = (node.x - listenerX) / bp.scale;
                    final dy = (node.y - listenerY) / bp.scale;
                    final dz = node.heightZ - listenerZ;
                    final dist = math.sqrt(dx*dx + dy*dy + dz*dz);
                    distances[node.id] = dist;
                    if (dist > maxDist) maxDist = dist;
                  }

                  final routingNotifier = ref.read(outputRoutingProvider.notifier);
                  final currentRouting = ref.read(outputRoutingProvider);

                  for (final node in nodes) {
                    final dist = distances[node.id]!;
                    final delayMs = ((maxDist - dist) / 343.0) * 1000.0;
                    double gainDb = 0.0;
                    if (dist > 0.1) {
                      gainDb = 20.0 * (math.log(dist / maxDist) / math.ln10);
                    }

                    final idx = currentRouting.indexWhere((ch) => ch.channel == node.channel);
                    final chModel = idx != -1 ? currentRouting[idx] : OutputChannelModel(channel: node.channel);

                    routingNotifier.updateChannel(
                      chModel.copyWith(
                        delayMs: delayMs.clamp(0.0, 100.0),
                        gain: gainDb.clamp(-60.0, 12.0),
                      )
                    );
                  }
                  
                  if (context.mounted) {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('✅ 3D Calibration 적용 완료! Output Routing에서 결과를 확인하세요.')),
                    );
                  }
                },
                child: const Text('🎯 Apply 3D Calibration'),
              ),
              const SizedBox(width: 8),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.background,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(4),
                    side: const BorderSide(color: Colors.white24, width: 1),
                  ),
                ),
                onPressed: () {
                  if (context.mounted) {
                    showDialog(
                      context: context,
                      builder: (context) => const OutputCalibrationModal(),
                    );
                  }
                },
                child: const Text('🎛 Output Routing', style: TextStyle(color: Colors.white)),
              ),
              const SizedBox(width: 8),
"""

content = content.replace(target, new_buttons + target)

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
