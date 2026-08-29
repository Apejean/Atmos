with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write("""import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:atmos_mixer_pro/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';

class SpeakerCanvasScreen extends ConsumerStatefulWidget {
  const SpeakerCanvasScreen({super.key});

  @override
  ConsumerState<SpeakerCanvasScreen> createState() => _SpeakerCanvasScreenState();
}

class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {
  String? _selectedInspectorSpeakerId;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0F111A),
      body: Stack(
        children: [
          // The new 3D Orbit View Engine
          Dynamic3DRoom(
            onSpeakerTapped: (id) {
              setState(() {
                _selectedInspectorSpeakerId = id;
              });
            },
          ),
          
          // Back Button
          Positioned(
            top: 24,
            left: 24,
            child: Material(
              color: Colors.white.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(8),
              clipBehavior: Clip.antiAlias,
              child: IconButton(
                icon: const Icon(Icons.arrow_back, color: Colors.white),
                onPressed: () => Navigator.of(context).pop(),
              ),
            ),
          ),
          
          // Inspector Panel (Glassmorphism right side)
          if (_selectedInspectorSpeakerId != null)
            Align(
              alignment: Alignment.centerRight,
              child: SizedBox(
                width: 320,
                child: SpeakerInspectorPanel(
                  speakerId: _selectedInspectorSpeakerId!,
                  onClose: () {
                    setState(() {
                      _selectedInspectorSpeakerId = null;
                    });
                  },
                ),
              ),
            ),
            
          // Top Title
          const Positioned(
            top: 32,
            left: 0,
            right: 0,
            child: Center(
              child: Text(
                "3D 룸 시뮬레이터 (Orbit View)",
                style: TextStyle(
                  color: Colors.white70,
                  fontSize: 16,
                  fontWeight: FontWeight.bold,
                  letterSpacing: 2.0,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
""")
