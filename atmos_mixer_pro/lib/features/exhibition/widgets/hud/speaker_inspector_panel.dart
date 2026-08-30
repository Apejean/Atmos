import 'package:flutter/material.dart';
import 'dart:math' as math;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';

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
  Widget _buildControlBox(
    String iconPath,
    String label,
    double value,
    String unit,
    double min,
    double max,
    Function(double)? onChanged,
  ) {
    return Container(
      margin: const EdgeInsets.only(bottom: 12),
      padding: const EdgeInsets.only(bottom: 4),
      decoration: BoxDecoration(
        color: Colors.white.withValues(alpha: 0.05),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
      ),
      child: Column(
        children: [
          Row(
            children: [
              Container(
                width: 48,
                height: 48,
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
                    onDoubleTap: onChanged == null ? null : () {
                      _showEditDialog(label, value, min, max, onChanged!);
                    },
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Expanded(child: Text(label, style: const TextStyle(color: Colors.white70, fontSize: 12), overflow: TextOverflow.ellipsis)),
                        Flexible(child: Text('${value.toStringAsFixed(1)}$unit', style: const TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.bold), overflow: TextOverflow.ellipsis)),
                      ],
                    ),
                  ),
                ),
              ),
            ],
          ),
          SizedBox(
            height: 24,
            child: SliderTheme(
              data: SliderThemeData(
                trackHeight: 2.0,
                activeTrackColor: Colors.lightBlueAccent,
                inactiveTrackColor: Colors.white.withValues(alpha: 0.1),
                thumbColor: Colors.lightBlueAccent,
                overlayColor: Colors.lightBlueAccent.withValues(alpha: 0.2),
                thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6.0),
                overlayShape: const RoundSliderOverlayShape(overlayRadius: 14.0),
              ),
              child: GestureDetector(
                onDoubleTap: onChanged == null ? null : () {
                  final middle = (min + max) / 2;
                  onChanged!(middle);
                },
                child: Slider(
                  value: value.clamp(min, max),
                  min: min,
                  max: max,
                  onChanged: onChanged,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _showEditDialog(String label, double currentValue, double min, double max, Function(double) onChanged) async {
    final controller = TextEditingController(text: currentValue.toString());
    await showDialog(
      context: context,
      builder: (context) => AlertDialog(
        backgroundColor: const Color(0xFF1E2632),
        title: Text('Edit $label', style: const TextStyle(color: Colors.white)),
        content: TextField(
          controller: controller,
          keyboardType: const TextInputType.numberWithOptions(decimal: true),
          style: const TextStyle(color: Colors.white),
          autofocus: true,
          decoration: InputDecoration(
            hintText: 'Min: $min, Max: $max',
            hintStyle: const TextStyle(color: Colors.white30),
            enabledBorder: const UnderlineInputBorder(borderSide: BorderSide(color: Colors.white30)),
            focusedBorder: const UnderlineInputBorder(borderSide: BorderSide(color: Colors.lightBlueAccent)),
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
                onChanged(val.clamp(min, max));
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
    final rooms = ref.watch(roomZoneProvider);
    final engineState = ref.watch(engineStateProvider);
    final maxChannels = engineState.outputChannelCount;
    final speakers = layout;
    SpeakerNode? speaker;
    try {
      speaker = layout.firstWhere((s) => s.id == widget.speakerId);
    } catch (e) {
      speaker = null;
    }

    if (speaker == null) return const SizedBox.shrink();

    double roomW = 10.0;
    double roomD = 10.0;
    double roomH = 5.0;
    
    if (rooms.isNotEmpty) {
      roomW = rooms.first.physicalWidth;
      roomD = rooms.first.physicalHeight;
      roomH = rooms.first.ceilingHeight;
    }

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
                      value: speaker.channel < maxChannels ? speaker.channel : (maxChannels > 0 ? 0 : speaker.channel),
                      dropdownColor: const Color(0xFF1E2632),
                      style: const TextStyle(color: Colors.white, fontSize: 14, fontWeight: FontWeight.bold),
                      items: List.generate(maxChannels > 0 ? maxChannels : 2, (i) {
                        final inUseBy = speakers.where((s) => s.channel == i && s.id != speaker!.id).firstOrNull;
                        final label = inUseBy != null ? 'Output CH ${i + 1} (In Use: ${inUseBy.id.substring(0, math.min(3, inUseBy.id.length))})' : 'Output CH ${i + 1}';
                        return DropdownMenuItem(
                          value: i, 
                          child: Row(
                            children: [
                              Text(
                                label, 
                                style: TextStyle(color: inUseBy != null ? Colors.white54 : Colors.white)
                              ),
                              if (speaker!.channel == i) const Padding(
                                padding: EdgeInsets.only(left: 8.0),
                                child: Icon(Icons.check, size: 16, color: Colors.lightBlueAccent),
                              )
                            ]
                          )
                        );
                      }),
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
                _buildControlBox('assets/3d_simulator/icons/icon_x.svg', 'X Position', speaker.x, 'm', 0.25, roomW - 0.25, speaker.isFixed ? null : (v) => _updateSpeaker(speaker!, x: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_y.svg', 'Y Position', speaker.y, 'm', 0.25, roomD - 0.25, speaker.isFixed ? null : (v) => _updateSpeaker(speaker!, y: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_height.svg', 'Z Height', speaker.heightZ, 'm', 0.25, roomH - 0.25, speaker.isFixed ? null : (v) => _updateSpeaker(speaker!, z: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_tilt.svg', 'Yaw (Rotation)', speaker.rotation, '°', -180.0, 180.0, speaker.isFixed ? null : (v) => _updateSpeaker(speaker!, rot: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_tilt.svg', 'Pitch (Tilt)', speaker.pitchTilt, '°', -90.0, 90.0, speaker.isFixed ? null : (v) => _updateSpeaker(speaker!, tilt: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_dispersion.svg', 'Dispersion', speaker.dispersionAngle, '°', 10.0, 180.0, speaker.isFixed ? null : (v) => _updateSpeaker(speaker!, disp: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_reverb.svg', 'Reverb Send', speaker.reverbSend, '%', 0.0, 100.0, (v) => _updateSpeaker(speaker!, rev: v)),
              ],
            ),
          ),
          
          // Footer
          Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Expanded(
                  child: ElevatedButton.icon(
                    onPressed: () {
                      _updateSpeaker(speaker!, isFixed: !speaker.isFixed);
                    },
                    icon: Icon(
                      speaker.isFixed ? Icons.lock_rounded : Icons.lock_open_rounded,
                      size: 18,
                      color: speaker.isFixed ? const Color(0xFFF59E0B) : Colors.white70,
                    ),
                    label: Text(
                      speaker.isFixed ? 'FIX ON' : 'FIX OFF',
                      style: TextStyle(
                        fontWeight: FontWeight.bold,
                        color: speaker.isFixed ? const Color(0xFFF59E0B) : Colors.white70,
                      ),
                    ),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: const Color(0xFF0F172A),
                      padding: const EdgeInsets.symmetric(vertical: 14),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(8),
                        side: BorderSide(
                          color: speaker.isFixed ? const Color(0xFFF59E0B) : const Color(0xFF334155),
                          width: 1.5,
                        ),
                      ),
                      elevation: 0,
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: OutlinedButton.icon(
                    onPressed: () {
                      ref.read(speakerLayoutProvider.notifier).removeSpeaker(speaker!.id);
                      widget.onClose();
                    },
                    icon: const Icon(Icons.delete_outline_rounded, size: 18, color: Colors.redAccent),
                    label: const Text('Remove', style: TextStyle(color: Colors.redAccent, fontWeight: FontWeight.bold)),
                    style: OutlinedButton.styleFrom(
                      side: const BorderSide(color: Colors.redAccent, width: 1.5),
                      padding: const EdgeInsets.symmetric(vertical: 14),
                      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  void _updateSpeaker(SpeakerNode speaker, {double? x, double? y, double? z, double? pan, double? tilt, double? rot, double? disp, double? rev, bool? isFixed}) {
    ref.read(speakerLayoutProvider.notifier).updateSpeaker(speaker.copyWith(
      x: x ?? speaker.x,
      y: y ?? speaker.y,
      heightZ: z ?? speaker.heightZ,
      pitchTilt: tilt ?? speaker.pitchTilt,
      rotation: rot ?? speaker.rotation,
      panDeg: pan ?? speaker.panDeg,
      dispersionAngle: disp ?? speaker.dispersionAngle,
      reverbSend: rev ?? speaker.reverbSend,
      isFixed: isFixed ?? speaker.isFixed,
    ));
    // Trigger real-time sync via global state or similar if needed.
  }
}
