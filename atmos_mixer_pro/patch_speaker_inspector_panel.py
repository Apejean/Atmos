import re

content = """import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';

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
  
  Widget _buildVerticalSliderSection(String title, double value, double min, double max, String suffix, Function(double) onChanged) {
    return Container(
      padding: const EdgeInsets.all(12),
      margin: const EdgeInsets.only(bottom: 12),
      decoration: BoxDecoration(
        color: const Color(0xFF1B232D).withValues(alpha: 0.7),
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title, style: const TextStyle(color: Colors.white70, fontSize: 12, fontWeight: FontWeight.bold)),
          const SizedBox(height: 12),
          Row(
            children: [
              SizedBox(
                height: 100,
                child: RotatedBox(
                  quarterTurns: 3,
                  child: SliderTheme(
                    data: SliderThemeData(
                      trackHeight: 4,
                      activeTrackColor: Colors.lightBlueAccent,
                      inactiveTrackColor: Colors.white12,
                      thumbColor: Colors.white,
                      overlayColor: Colors.lightBlueAccent.withValues(alpha: 0.2),
                      thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
                    ),
                    child: Slider(
                      value: value,
                      min: min,
                      max: max,
                      onChanged: onChanged,
                    ),
                  ),
                ),
              ),
              const Spacer(),
              Column(
                children: [
                  const Icon(Icons.speaker, color: Colors.white54, size: 28),
                  const SizedBox(height: 4),
                  const Icon(Icons.arrow_upward, color: Colors.lightBlueAccent, size: 16),
                  const SizedBox(height: 12),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                    decoration: BoxDecoration(
                      color: const Color(0xFF1B232D),
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(color: Colors.lightBlueAccent.withValues(alpha: 0.5)),
                    ),
                    child: Text('${value.toStringAsFixed(1)}$suffix', style: const TextStyle(color: Colors.lightBlueAccent, fontSize: 13, fontWeight: FontWeight.bold)),
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildHorizontalSliderSection(String title, double value, double min, double max, String suffix, IconData icon, Function(double) onChanged) {
    return Container(
      padding: const EdgeInsets.all(12),
      margin: const EdgeInsets.only(bottom: 12),
      decoration: BoxDecoration(
        color: const Color(0xFF1B232D).withValues(alpha: 0.7),
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title, style: const TextStyle(color: Colors.white70, fontSize: 12, fontWeight: FontWeight.bold)),
          const SizedBox(height: 12),
          Row(
            children: [
              Expanded(
                flex: 2,
                child: Column(
                  children: [
                    Icon(Icons.speed, color: Colors.lightBlueAccent.withValues(alpha: 0.5), size: 40),
                    SliderTheme(
                      data: SliderThemeData(
                        trackHeight: 4,
                        activeTrackColor: Colors.lightBlueAccent,
                        inactiveTrackColor: Colors.white12,
                        thumbColor: Colors.white,
                        overlayColor: Colors.lightBlueAccent.withValues(alpha: 0.2),
                        thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
                      ),
                      child: Slider(
                        value: value,
                        min: min,
                        max: max,
                        onChanged: onChanged,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                flex: 1,
                child: Column(
                  children: [
                    Icon(icon, color: Colors.white54, size: 36),
                    const SizedBox(height: 12),
                    Container(
                      width: double.infinity,
                      alignment: Alignment.center,
                      padding: const EdgeInsets.symmetric(vertical: 6),
                      decoration: BoxDecoration(
                        color: const Color(0xFF1B232D),
                        borderRadius: BorderRadius.circular(4),
                        border: Border.all(color: Colors.lightBlueAccent.withValues(alpha: 0.5)),
                      ),
                      child: Text('${value.toStringAsFixed(1)}$suffix', style: const TextStyle(color: Colors.lightBlueAccent, fontSize: 13, fontWeight: FontWeight.bold)),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final layout = ref.watch(speakerLayoutProvider);
    SpeakerNode? speaker;
    try {
      speaker = layout.firstWhere((s) => s.id == widget.speakerId);
    } catch (e) {
      speaker = null;
    }

    if (speaker == null) return const SizedBox.shrink();

    final String channelLabel = 'S${(speaker.channel + 1).toString().padLeft(2, '0')}';

    return Container(
      width: 340,
      decoration: BoxDecoration(
        color: const Color(0xFF151921).withValues(alpha: 0.95),
        border: Border(left: BorderSide(color: Colors.lightBlueAccent.withValues(alpha: 0.3), width: 1)),
        boxShadow: [BoxShadow(color: Colors.black.withValues(alpha: 0.5), blurRadius: 20)],
      ),
      child: Column(
        children: [
          // Header
          Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(border: Border(bottom: BorderSide(color: Colors.white.withValues(alpha: 0.1)))),
            child: Row(
              children: [
                const Icon(Icons.volume_up, color: Colors.lightBlueAccent),
                const SizedBox(width: 8),
                const Expanded(child: Text('SPEAKER INSPECTOR', style: TextStyle(color: Colors.lightBlueAccent, fontSize: 16, fontWeight: FontWeight.bold))),
                IconButton(
                  icon: const Icon(Icons.close, color: Colors.white54, size: 20),
                  padding: EdgeInsets.zero,
                  constraints: const BoxConstraints(),
                  onPressed: widget.onClose,
                ),
              ],
            ),
          ),
          
          // Body
          Expanded(
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: [
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                  margin: const EdgeInsets.only(bottom: 16),
                  decoration: BoxDecoration(
                    color: const Color(0xFF1B232D),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text('Selected Speaker: $channelLabel (Front Right)', style: const TextStyle(color: Colors.white70, fontSize: 12)),
                ),

                _buildVerticalSliderSection('HEIGHT (Z)', speaker.heightZ, 0.0, 10.0, ' m', (v) => _updateSpeaker(speaker!, z: v)),
                _buildHorizontalSliderSection('TILT ANGLE', speaker.pitchTilt, -90.0, 90.0, '° Down', Icons.screen_rotation, (v) => _updateSpeaker(speaker!, tilt: v)),
                _buildHorizontalSliderSection('PAN ANGLE', speaker.panDeg, -180.0, 180.0, '°', Icons.explore, (v) => _updateSpeaker(speaker!, pan: v)),
              ],
            ),
          ),
          
          // Footer
          Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Expanded(
                  child: OutlinedButton(
                    onPressed: () {},
                    style: OutlinedButton.styleFrom(
                      foregroundColor: Colors.white,
                      side: const BorderSide(color: Colors.white24),
                      padding: const EdgeInsets.symmetric(vertical: 12),
                    ),
                    child: const Text('Mute', style: TextStyle(fontSize: 12)),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: OutlinedButton(
                    onPressed: () {},
                    style: OutlinedButton.styleFrom(
                      foregroundColor: Colors.white,
                      side: const BorderSide(color: Colors.white24),
                      padding: const EdgeInsets.symmetric(vertical: 12),
                    ),
                    child: const Text('Solo', style: TextStyle(fontSize: 12)),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  flex: 2,
                  child: OutlinedButton(
                    onPressed: () {
                      ref.read(speakerLayoutProvider.notifier).removeSpeaker(speaker!.id);
                      widget.onClose();
                    },
                    style: OutlinedButton.styleFrom(
                      foregroundColor: Colors.lightBlueAccent,
                      side: const BorderSide(color: Colors.lightBlueAccent),
                      padding: const EdgeInsets.symmetric(vertical: 12),
                    ),
                    child: const Text('Remove Speaker', style: TextStyle(fontSize: 12)),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  void _updateSpeaker(SpeakerNode speaker, {double? z, double? pan, double? tilt}) {
    ref.read(speakerLayoutProvider.notifier).updateSpeaker(speaker.copyWith(
      heightZ: z ?? speaker.heightZ,
      pitchTilt: tilt ?? speaker.pitchTilt,
      panDeg: pan ?? speaker.panDeg,
    ));
  }
}
"""

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'w') as f:
    f.write(content)

