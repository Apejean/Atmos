import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:uuid/uuid.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/speaker_node_widget.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';

const double _gridSize = 50.0;
const double _canvasWidth = 2000.0;
const double _canvasHeight = 2000.0;

class SpeakerCanvasScreen extends ConsumerStatefulWidget {
  const SpeakerCanvasScreen({super.key});

  @override
  ConsumerState<SpeakerCanvasScreen> createState() => _SpeakerCanvasScreenState();
}

class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {
  final TransformationController _transformationController = TransformationController();

  @override
  void initState() {
    super.initState();
    // Center the view initially
    _transformationController.value = Matrix4.identity()
      ..translate(-_canvasWidth / 2 + 400, -_canvasHeight / 2 + 300, 0.0);
  }

  @override
  void dispose() {
    _transformationController.dispose();
    super.dispose();
  }

  void _addSpeaker() {
    final centerMatrix = _transformationController.value.clone()..invert();
    // Approximate center of screen mapped to canvas
    double cx = centerMatrix.getTranslation().x + 400;
    double cy = centerMatrix.getTranslation().y + 300;
    
    cx = (cx / _gridSize).round() * _gridSize;
    cy = (cy / _gridSize).round() * _gridSize;

    final newNode = SpeakerNode(
      id: const Uuid().v4(),
      x: cx,
      y: cy,
      channel: 0,
    );
    ref.read(speakerLayoutProvider.notifier).addSpeaker(newNode);
  }

  @override
  Widget build(BuildContext context) {
    final nodes = ref.watch(speakerLayoutProvider);

    return Scaffold(
      backgroundColor: AppColors.background,
      appBar: AppBar(
        title: Row(
          children: [
            const Text('Speaker Canvas (Exhibition)'),
            const SizedBox(width: 16),
            Consumer(
              builder: (context, ref, child) {
                final isMasterMuted = ref.watch(engineStateProvider.select((state) => state.masterMuteActive));
                if (!isMasterMuted) return const SizedBox.shrink();
                return Container(
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
                  decoration: BoxDecoration(
                    color: Colors.orange.shade800,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: const Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(Icons.warning_amber_rounded, color: Colors.white, size: 16),
                      SizedBox(width: 8),
                      Text(
                        'MASTER MUTE ACTIVE',
                        style: TextStyle(
                          color: Colors.white,
                          fontWeight: FontWeight.bold,
                          fontSize: 12,
                          letterSpacing: 1.0,
                        ),
                      ),
                    ],
                  ),
                );
              },
            ),
          ],
        ),
        backgroundColor: Colors.black,
      ),
      body: InteractiveViewer(
        transformationController: _transformationController,
        boundaryMargin: const EdgeInsets.all(double.infinity),
        minScale: 0.1,
        maxScale: 2.0,
        constrained: false,
        child: SizedBox(
          width: _canvasWidth,
          height: _canvasHeight,
          child: Stack(
            clipBehavior: Clip.none,
            children: [
              // Grid Background
              Positioned.fill(
                child: CustomPaint(
                  painter: _GridPainter(),
                ),
              ),
              
              // Speaker Nodes
              ...nodes.map((node) {
                // Check if this speaker's channel is duplicated among other nodes
                final isDuplicate = nodes.where((n) => n.id != node.id && n.channel == node.channel).isNotEmpty;

                return Positioned(
                  left: node.x,
                  top: node.y,
                  child: GestureDetector(
                    onPanUpdate: (details) {
                      // Get current zoom scale from InteractiveViewer transformation matrix
                      final scale = _transformationController.value.getMaxScaleOnAxis();
                      final currentScale = scale > 0 ? scale : 1.0;

                      // Normalize drag delta by current scale so movement tracks pointer accurately
                      final updated = node.copyWith(
                        x: node.x + (details.delta.dx / currentScale),
                        y: node.y + (details.delta.dy / currentScale),
                      );
                      ref.read(speakerLayoutProvider.notifier).updateSpeaker(updated);
                    },
                    onPanEnd: (details) {
                      // Snap to grid on release and save immediately
                      final snappedX = (node.x / _gridSize).round() * _gridSize;
                      final snappedY = (node.y / _gridSize).round() * _gridSize;
                      final updated = node.copyWith(x: snappedX, y: snappedY);
                      ref.read(speakerLayoutProvider.notifier).updateSpeaker(updated, immediate: true);
                    },
                    child: SpeakerNodeWidget(
                      node: node,
                      isDuplicateChannel: isDuplicate,
                      onChannelChanged: (ch) {
                        ref.read(speakerLayoutProvider.notifier).updateSpeaker(node.copyWith(channel: ch));
                      },
                      onDelete: () {
                        ref.read(speakerLayoutProvider.notifier).removeSpeaker(node.id);
                      },
                    ),
                  ),
                );
              }),
            ],
          ),
        ),
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: _addSpeaker,
        backgroundColor: AppColors.primaryNeon,
        child: const Icon(Icons.add, color: Colors.black),
      ),
    );
  }
}

class _GridPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.white12
      ..strokeWidth = 1.0;

    for (double i = 0; i <= size.width; i += _gridSize) {
      canvas.drawLine(Offset(i, 0), Offset(i, size.height), paint);
    }
    for (double i = 0; i <= size.height; i += _gridSize) {
      canvas.drawLine(Offset(0, i), Offset(size.width, i), paint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
