import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';

class SpeakerInspectorPanel extends ConsumerStatefulWidget {
  final String speakerId;
  final VoidCallback onClose;

  const SpeakerInspectorPanel({
    super.key,
    required this.speakerId,
    required this.onClose,
  });

  @override
  ConsumerState<SpeakerInspectorPanel> createState() => _SpeakerInspectorPanelState();
}

class _SpeakerInspectorPanelState extends ConsumerState<SpeakerInspectorPanel> {
  @override
  Widget build(BuildContext context) {
    final blueprint = ref.watch(blueprintProvider);
    SpeakerNode? speaker;
    try {
      speaker = blueprint.speakers.firstWhere((s) => s.id == widget.speakerId);
    } catch (e) {
      speaker = null;
    }

    if (speaker == null) {
      return const SizedBox.shrink();
    }

    const bgColor = Color(0xFF232C3A);
    const borderColor = Color(0xFF3F556D);
    const textLight = Color(0xFFE2E8F0);

    return Container(
      width: 300,
      decoration: BoxDecoration(
        color: bgColor.withValues(alpha: 0.95),
        border: const Border(
          left: BorderSide(color: borderColor, width: 1),
        ),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.5),
            blurRadius: 10,
            offset: const Offset(-2, 0),
          ),
        ],
      ),
      child: Column(
        children: [
          // Header
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
            decoration: const BoxDecoration(
              color: Color(0xFF1E2632),
              border: Border(bottom: BorderSide(color: borderColor, width: 1)),
            ),
            child: Row(
              children: [
                const Icon(Icons.speaker, color: Colors.lightBlueAccent, size: 18),
                const SizedBox(width: 8),
                const Expanded(
                  child: Text(
                    'SPEAKER INSPECTOR',
                    style: TextStyle(
                      color: textLight,
                      fontSize: 13,
                      fontWeight: FontWeight.w600,
                      letterSpacing: 0.5,
                    ),
                  ),
                ),
                InkWell(
                  onTap: widget.onClose,
                  child: const Icon(Icons.close, color: Colors.white54, size: 18),
                ),
              ],
            ),
          ),
          
          // Body
          Expanded(
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  _buildSectionTitle('1. IDENTITY'),
                  _buildRow('Channel:', 'Ch ${speaker.channelIndex + 1} (${speaker.label})'),
                  const SizedBox(height: 16),
                  
                  _buildSectionTitle('2. POSITION & ORIENTATION'),
                  _buildSliderRow('X Position', speaker.x, 0.0, 20.0, (val) => _updateSpeaker(speaker!, x: val)),
                  _buildSliderRow('Y Position', speaker.y, 0.0, 20.0, (val) => _updateSpeaker(speaker!, y: val)),
                  // Note: Assuming heightZ, pitchTilt, panDeg exist or will be added. Default to 0 for now.
                  _buildSliderRow('Z Height', 2.0, 0.0, 10.0, (val) {}),
                  _buildSliderRow('Tilt', 0.0, -90.0, 90.0, (val) {}),
                  _buildSliderRow('Pan', 0.0, -180.0, 180.0, (val) {}),
                  
                  const SizedBox(height: 16),
                  _buildSectionTitle('3. POWER & COVERAGE'),
                  _buildRow('Type:', 'Mid-size (300W)'),
                  
                  const SizedBox(height: 16),
                  _buildSectionTitle('4. DSP MATRIX'),
                  _buildSliderRow('Gain', 0.0, -24.0, 12.0, (val) {}),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildSectionTitle(String title) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Text(
        title,
        style: const TextStyle(
          color: Colors.lightBlueAccent,
          fontSize: 11,
          fontWeight: FontWeight.bold,
          letterSpacing: 0.5,
        ),
      ),
    );
  }

  Widget _buildRow(String label, String value) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label, style: const TextStyle(color: Colors.white70, fontSize: 12)),
          Text(value, style: const TextStyle(color: Colors.white, fontSize: 12, fontWeight: FontWeight.bold)),
        ],
      ),
    );
  }

  Widget _buildSliderRow(String label, double value, double min, double max, Function(double) onChanged) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        children: [
          SizedBox(
            width: 70,
            child: Text(label, style: const TextStyle(color: Colors.white70, fontSize: 12)),
          ),
          Expanded(
            child: SliderTheme(
              data: SliderThemeData(
                trackHeight: 2,
                thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
                overlayShape: const RoundSliderOverlayShape(overlayRadius: 14),
                activeTrackColor: Colors.lightBlueAccent,
                inactiveTrackColor: Colors.white12,
                thumbColor: Colors.white,
              ),
              child: Slider(
                value: value.clamp(min, max),
                min: min,
                max: max,
                onChanged: onChanged,
              ),
            ),
          ),
          SizedBox(
            width: 40,
            child: Text(
              value.toStringAsFixed(1),
              textAlign: TextAlign.right,
              style: const TextStyle(color: Colors.lightBlueAccent, fontSize: 12, fontWeight: FontWeight.bold),
            ),
          ),
        ],
      ),
    );
  }

  void _updateSpeaker(SpeakerNode speaker, {double? x, double? y}) {
    final updated = speaker.copyWith(
      x: x ?? speaker.x,
      y: y ?? speaker.y,
    );
    ref.read(blueprintProvider.notifier).updateSpeaker(updated);
  }
}
