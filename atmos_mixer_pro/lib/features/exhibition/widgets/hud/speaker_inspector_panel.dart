import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_svg/flutter_svg.dart';
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
  Widget _buildControlBox(String iconPath, String label, double value, String unit, Function(double) onChanged) {
    return Container(
      margin: const EdgeInsets.only(bottom: 12),
      height: 48,
      decoration: BoxDecoration(
        color: Colors.white.withValues(alpha: 0.05),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
      ),
      child: Row(
        children: [
          Container(
            width: 48,
            alignment: Alignment.center,
            decoration: BoxDecoration(
              border: Border(right: BorderSide(color: Colors.white.withValues(alpha: 0.1))),
            ),
            child: SvgPicture.asset(iconPath, width: 24, height: 24, colorFilter: const ColorFilter.mode(Colors.lightBlueAccent, BlendMode.srcIn)),
          ),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12),
              child: GestureDetector(
                onDoubleTap: () {
                  _showEditDialog(label, value, onChanged);
                },
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(label, style: const TextStyle(color: Colors.white70, fontSize: 12)),
                    Text('\${value.toStringAsFixed(1)}$unit', style: const TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.bold)),
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _showEditDialog(String label, double currentValue, Function(double) onChanged) async {
    final controller = TextEditingController(text: currentValue.toString());
    await showDialog(
      context: context,
      builder: (context) => AlertDialog(
        backgroundColor: const Color(0xFF1E2632),
        title: Text('Edit \$label', style: const TextStyle(color: Colors.white)),
        content: TextField(
          controller: controller,
          keyboardType: const TextInputType.numberWithOptions(decimal: true),
          style: const TextStyle(color: Colors.white),
          autofocus: true,
          decoration: const InputDecoration(
            enabledBorder: UnderlineInputBorder(borderSide: BorderSide(color: Colors.white30)),
            focusedBorder: UnderlineInputBorder(borderSide: BorderSide(color: Colors.lightBlueAccent)),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel', style: TextStyle(color: Colors.white54)),
          ),
          TextButton(
            onPressed: () {
              final val = double.tryParse(controller.text);
              if (val != null) {
                onChanged(val);
              }
              Navigator.pop(context);
            },
            child: const Text('Apply', style: TextStyle(color: Colors.lightBlueAccent)),
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

    return Container(
      width: 320,
      decoration: BoxDecoration(
        color: const Color(0xFF151921).withValues(alpha: 0.85),
        border: Border(left: BorderSide(color: Colors.white.withValues(alpha: 0.1), width: 1)),
        boxShadow: [BoxShadow(color: Colors.black.withValues(alpha: 0.5), blurRadius: 20)],
      ),
      child: Column(
        children: [
          // Header
          Container(
            height: 60,
            padding: const EdgeInsets.symmetric(horizontal: 16),
            decoration: BoxDecoration(border: Border(bottom: BorderSide(color: Colors.white.withValues(alpha: 0.1)))),
            child: Row(
              children: [
                Expanded(
                  child: DropdownButtonHideUnderline(
                    child: DropdownButton<int>(
                      value: speaker.channel,
                      dropdownColor: const Color(0xFF1E2632),
                      style: const TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.bold),
                      items: List.generate(24, (i) => DropdownMenuItem(value: i, child: Text('Selected Speaker: CH \${i + 1}'))),
                      onChanged: (val) {
                        if (val != null) {
                          ref.read(speakerLayoutProvider.notifier).updateSpeaker(speaker!.copyWith(channel: val));
                        }
                      },
                    ),
                  ),
                ),
                IconButton(icon: const Icon(Icons.close, color: Colors.white54), onPressed: widget.onClose),
              ],
            ),
          ),
          
          // Body
          Expanded(
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: [
                _buildControlBox('assets/3d_simulator/icons/icon_x.svg', 'X Position', speaker.x, 'm', (v) => _updateSpeaker(speaker!, x: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_y.svg', 'Y Position', speaker.y, 'm', (v) => _updateSpeaker(speaker!, y: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_height.svg', 'Z Height', speaker.heightZ, 'm', (v) => _updateSpeaker(speaker!, z: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_pan.svg', 'Pan', speaker.panDeg, '°', (v) => _updateSpeaker(speaker!, pan: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_tilt.svg', 'Tilt', speaker.pitchTilt, '°', (v) => _updateSpeaker(speaker!, tilt: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_dispersion.svg', 'Dispersion', speaker.dispersionAngle, '°', (v) => _updateSpeaker(speaker!, disp: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_reverb.svg', 'Reverb Send', speaker.reverbSend, '%', (v) => _updateSpeaker(speaker!, rev: v)),
              ],
            ),
          ),
          
          // Footer
          Padding(
            padding: const EdgeInsets.all(16),
            child: OutlinedButton(
              onPressed: () {
                ref.read(speakerLayoutProvider.notifier).removeSpeaker(speaker!.id);
                widget.onClose();
              },
              style: OutlinedButton.styleFrom(
                side: const BorderSide(color: Colors.redAccent),
                minimumSize: const Size(double.infinity, 48),
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
              ),
              child: const Text('Remove Speaker', style: TextStyle(color: Colors.redAccent, fontWeight: FontWeight.bold)),
            ),
          ),
        ],
      ),
    );
  }

  void _updateSpeaker(SpeakerNode speaker, {double? x, double? y, double? z, double? pan, double? tilt, double? disp, double? rev}) {
    ref.read(speakerLayoutProvider.notifier).updateSpeaker(speaker.copyWith(
      x: x ?? speaker.x,
      y: y ?? speaker.y,
      heightZ: z ?? speaker.heightZ,
      pitchTilt: tilt ?? speaker.pitchTilt,
      panDeg: pan ?? speaker.panDeg,
      dispersionAngle: disp ?? speaker.dispersionAngle,
      reverbSend: rev ?? speaker.reverbSend,
    ));
    // Trigger real-time sync via global state or similar if needed.
  }
}
