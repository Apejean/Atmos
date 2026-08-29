with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write("""import 'dart:math' as math;
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:vector_math/vector_math_64.dart' as vector;

import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/viewport_3d/speaker_3d_box.dart';

class SceneObject {
  final vector.Vector3 position;
  final Widget child;

  SceneObject({required this.position, required this.child});
}

class Dynamic3DRoom extends ConsumerStatefulWidget {
  final Function(String)? onSpeakerTapped;
  const Dynamic3DRoom({super.key, this.onSpeakerTapped});

  @override
  ConsumerState<Dynamic3DRoom> createState() => _Dynamic3DRoomState();
}

class _Dynamic3DRoomState extends ConsumerState<Dynamic3DRoom> {
  double _pitch = 0.3; 
  double _yaw = -0.5;
  double _zoom = 0.8;
  
  final double PPM = 100.0; // Pixels Per Meter

  @override
  Widget build(BuildContext context) {
    final speakers = ref.watch(speakerLayoutProvider);
    final bp = ref.watch(blueprintProvider);

    final roomW = bp.canvasWidthMeters * PPM;
    final roomD = bp.canvasHeightMeters * PPM;
    final roomH = 3.5 * PPM; // standard height

    final cameraMatrix = Matrix4.identity()
      ..setEntry(3, 2, 0.001) // perspective
      ..scale(_zoom, _zoom, _zoom)
      ..rotateX(_pitch)
      ..rotateY(_yaw);

    final objects = <SceneObject>[];

    // 1. Floor Grid
    objects.add(SceneObject(
      position: vector.Vector3(0, roomH / 2, 0),
      child: Transform(
        transform: Matrix4.translationValues(0, roomH / 2, 0)..rotateX(math.pi / 2),
        alignment: Alignment.center,
        child: Container(
          width: roomW,
          height: roomD,
          decoration: BoxDecoration(
            border: Border.all(color: Colors.lightBlueAccent.withValues(alpha: 0.3), width: 2),
            color: Colors.lightBlueAccent.withValues(alpha: 0.05),
          ),
          child: CustomPaint(painter: GridPainter(roomW, roomD)),
        ),
      ),
    ));

    // 2. Ceiling Wireframe
    objects.add(SceneObject(
      position: vector.Vector3(0, -roomH / 2, 0),
      child: Transform(
        transform: Matrix4.translationValues(0, -roomH / 2, 0)..rotateX(math.pi / 2),
        alignment: Alignment.center,
        child: Container(
          width: roomW,
          height: roomD,
          decoration: BoxDecoration(
            border: Border.all(color: Colors.lightBlueAccent.withValues(alpha: 0.3), width: 1),
          ),
        ),
      ),
    ));

    // 3. Pillars
    final pColor = Colors.lightBlueAccent.withValues(alpha: 0.3);
    final halfW = roomW / 2;
    final halfD = roomD / 2;
    
    Widget buildPillar(double x, double z) {
      return Transform(
        transform: Matrix4.translationValues(x, 0, z),
        alignment: Alignment.center,
        child: Container(width: 1, height: roomH, color: pColor),
      );
    }

    objects.add(SceneObject(position: vector.Vector3(-halfW, 0, -halfD), child: buildPillar(-halfW, -halfD)));
    objects.add(SceneObject(position: vector.Vector3(halfW, 0, -halfD), child: buildPillar(halfW, -halfD)));
    objects.add(SceneObject(position: vector.Vector3(-halfW, 0, halfD), child: buildPillar(-halfW, halfD)));
    objects.add(SceneObject(position: vector.Vector3(halfW, 0, halfD), child: buildPillar(halfW, halfD)));

    // 4. Dummy Head (Listener) at center, ear level 1.2m
    final headY = roomH / 2 - (1.2 * PPM);
    objects.add(SceneObject(
      position: vector.Vector3(0, headY, 0),
      child: Transform(
        transform: Matrix4.translationValues(0, headY, 0)
          ..rotateY(-_yaw)
          ..rotateX(-_pitch), // Billboard effect
        alignment: Alignment.center,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 32, height: 32,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: Colors.grey.shade300,
                boxShadow: const [BoxShadow(color: Colors.black54, blurRadius: 4)],
              ),
            ),
            Container(width: 8, height: 12, color: Colors.grey.shade400),
            Container(
              width: 48, height: 20,
              decoration: BoxDecoration(
                color: Colors.grey.shade500,
                borderRadius: BorderRadius.circular(10),
              ),
            )
          ],
        ),
      ),
    ));

    // 5. Speakers
    for (var spk in speakers) {
      final sx = (spk.x - bp.canvasWidthMeters / 2) * PPM;
      final sz = (spk.y - bp.canvasHeightMeters / 2) * PPM;
      final sy = roomH / 2 - (spk.heightZ * PPM);

      objects.add(SceneObject(
        position: vector.Vector3(sx, sy, sz),
        child: Transform(
          transform: Matrix4.translationValues(sx, sy, sz),
          alignment: Alignment.center,
          child: GestureDetector(
            onTap: () {
              if (widget.onSpeakerTapped != null) widget.onSpeakerTapped!(spk.id);
            },
            child: Transform.scale(
              scale: 0.25, // Adjust for SVG box size
              child: Speaker3DBox(
                angleX: spk.pitchTilt * math.pi / 180,
                angleY: spk.panDeg * math.pi / 180,
                angleZ: 0,
              ),
            ),
          ),
        ),
      ));
    }

    // Painter's Algorithm: Sort by Z-depth after camera transform
    objects.sort((a, b) {
      final aTrans = cameraMatrix.transform3(a.position);
      final bTrans = cameraMatrix.transform3(b.position);
      return bTrans.z.compareTo(aTrans.z);
    });

    return Scaffold(
      backgroundColor: const Color(0xFF0F111A),
      body: Stack(
        children: [
          GestureDetector(
            onPanUpdate: (details) {
              setState(() {
                _yaw -= details.delta.dx * 0.005;
                _pitch += details.delta.dy * 0.005;
                _pitch = _pitch.clamp(-math.pi / 2, math.pi / 2);
              });
            },
            child: Listener(
              onPointerSignal: (event) {
                if (event is PointerScrollEvent) {
                  setState(() {
                    _zoom -= event.scrollDelta.dy * 0.001;
                    _zoom = _zoom.clamp(0.2, 3.0);
                  });
                }
              },
              child: Container(
                color: Colors.transparent, // catch gestures
                child: Center(
                  child: Transform(
                    transform: cameraMatrix,
                    alignment: Alignment.center,
                    child: Stack(
                      clipBehavior: Clip.none,
                      alignment: Alignment.center,
                      children: objects.map((o) => o.child).toList(),
                    ),
                  ),
                ),
              ),
            ),
          ),
          
          // Floating Controls
          Positioned(
            bottom: 32,
            right: 32,
            child: FloatingActionButton.extended(
              onPressed: () {
                final newId = 'spk_${DateTime.now().millisecondsSinceEpoch}';
                final newNode = SpeakerNode(
                  id: newId,
                  x: bp.canvasWidthMeters / 2,
                  y: bp.canvasHeightMeters / 2,
                  channel: speakers.length,
                );
                ref.read(speakerLayoutProvider.notifier).addSpeaker(newNode);
              },
              backgroundColor: Colors.lightBlueAccent,
              icon: const Icon(Icons.add),
              label: const Text('Add Speaker'),
            ),
          ),
        ],
      ),
    );
  }
}

class GridPainter extends CustomPainter {
  final double w;
  final double d;

  GridPainter(this.w, this.d);

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.lightBlueAccent.withValues(alpha: 0.15)
      ..strokeWidth = 1.0;

    for (int i = 1; i < 4; i++) {
      final x = w * (i / 4);
      canvas.drawLine(Offset(x, 0), Offset(x, d), paint);
    }
    for (int i = 1; i < 4; i++) {
      final y = d * (i / 4);
      canvas.drawLine(Offset(0, y), Offset(w, y), paint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
""")
