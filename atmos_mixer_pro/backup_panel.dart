import 'package:flutter/material.dart';
import 'dart:math' as math;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/spatial_reverb_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/bass_management_provider.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/reverb_settings_modal.dart';

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
                      _showEditDialog(label, value, min, max, onChanged);
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
                  onChanged(middle);
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
    final speaker = layout.where((s) => s.id == widget.speakerId).firstOrNull;

    if (speaker == null) return const SizedBox.shrink();

    double roomW = 10.0;
    double roomD = 10.0;
    double roomH = 5.0;
    
    if (rooms.isNotEmpty) {
      final activeRoom = rooms.firstWhere((r) => r.id == speaker.roomId, orElse: () => rooms.first);
      roomW = activeRoom.physicalWidth;
      roomD = activeRoom.physicalHeight;
      roomH = activeRoom.ceilingHeight;
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
                        final inUseBy = speakers.where((s) => s.channel == i && s.id != speaker.id).firstOrNull;
                        final label = inUseBy != null ? 'Output CH ${i + 1} (In Use: ${inUseBy.id.substring(0, math.min(3, inUseBy.id.length))})' : 'Output CH ${i + 1}';
                        return DropdownMenuItem(
                          value: i, 
                          child: Row(
                            children: [
                              Text(
                                label, 
                                style: TextStyle(color: inUseBy != null ? Colors.white54 : Colors.white)
                              ),
                              if (speaker.channel == i) const Padding(
                                padding: EdgeInsets.only(left: 8.0),
                                child: Icon(Icons.check, size: 16, color: Colors.lightBlueAccent),
                              )
                            ]
                          )
                        );
                      }),
                      onChanged: (val) {
                        if (val != null) {
                          ref.read(speakerLayoutProvider.notifier).updateSpeaker(speaker.copyWith(channel: val));
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
                _buildControlBox('assets/3d_simulator/icons/icon_x.svg', 'X Position', speaker.x, 'm', 0.25, roomW - 0.25, speaker.isFixed ? null : (v) => _updateSpeaker(speaker, x: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_y.svg', 'Y Position', speaker.y, 'm', 0.25, roomD - 0.25, speaker.isFixed ? null : (v) => _updateSpeaker(speaker, y: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_height.svg', 'Z Height', speaker.heightZ, 'm', 0.25, roomH - 0.25, speaker.isFixed ? null : (v) => _updateSpeaker(speaker, z: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_yaw.svg', 'Yaw (Rotation)', speaker.rotation, '°', -180.0, 180.0, speaker.isFixed ? null : (v) => _updateSpeaker(speaker, rot: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_tilt.svg', 'Pitch (Tilt)', speaker.pitchTilt, '°', -90.0, 90.0, speaker.isFixed ? null : (v) => _updateSpeaker(speaker, tilt: v)),
                _buildControlBox('assets/3d_simulator/icons/icon_dispersion.svg', 'Dispersion', speaker.dispersionAngle, '°', 10.0, 180.0, speaker.isFixed ? null : (v) => _updateSpeaker(speaker, disp: v)),
                const SizedBox(height: 8),
                _buildReverbSendCard(context, speaker),
                _buildBassManagementCard(context, speaker),
              ],
            ),
          ),
          
          // Auto-Aim Button
          if (!speaker.isFixed)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: SizedBox(
                width: double.infinity,
                child: OutlinedButton.icon(
                  onPressed: () {
                    final bp = ref.read(blueprintProvider);
                    final currentRoom = ref.read(roomZoneProvider).where((r) => r.id == speaker.roomId).firstOrNull;
                    final roomW = currentRoom?.physicalWidth ?? bp.canvasWidthMeters;
                    final roomD = currentRoom?.physicalHeight ?? bp.canvasHeightMeters;
                    final earLevel = currentRoom?.earLevel ?? 1.2;

                    // Speaker coordinates relative to center (0,0)
                    final spkX = speaker.x - (roomW / 2);
                    final spkZ = speaker.y - (roomD / 2);
                    final spkY = speaker.heightZ;

                    // Target (Mannequin Ear)
                    final tarX = 0.0;
                    final tarY = earLevel;
                    final tarZ = 0.0;

                    // Calculate direction
                    final dx = tarX - spkX;
                    final dy = tarY - spkY;
                    final dz = tarZ - spkZ;

                    // Yaw = atan2(dx, dz)
                    final yawDeg = math.atan2(dx, dz) * 180 / math.pi;
                    
                    // Pitch = atan2(dy, distance_xz)
                    final distXZ = math.sqrt(dx * dx + dz * dz);
                    final pitchDeg = math.atan2(dy, distXZ) * 180 / math.pi;

                    _updateSpeaker(speaker, rot: yawDeg, tilt: pitchDeg);
                  },
                  icon: const Icon(Icons.my_location_rounded, size: 18),
                  label: const Text('Auto-Aim to Listener'),
                  style: OutlinedButton.styleFrom(
                    foregroundColor: const Color(0xFF22C55E),
                    side: const BorderSide(color: Color(0xFF22C55E)),
                    padding: const EdgeInsets.symmetric(vertical: 12),
                  ),
                ),
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
                      _updateSpeaker(speaker, isFixed: !speaker.isFixed);
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
                      ref.read(speakerLayoutProvider.notifier).removeSpeaker(speaker.id);
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

  Widget _buildReverbSendCard(BuildContext context, SpeakerNode speaker) {
    final targetCh = speaker.channel >= 0 ? speaker.channel + 1 : 1;
    final reverbState = ref.watch(spatialReverbProvider);
    final chSettings = reverbState.getSettingsForChannel(targetCh);
    final isEnabled = chSettings.isEnabled;

    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFF1E2632).withValues(alpha: 0.8),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: isEnabled
              ? const Color(0xFFFFA000).withValues(alpha: 0.35)
              : Colors.white.withValues(alpha: 0.1),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                width: 30,
                height: 30,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  color: const Color(0xFFFFA000).withValues(alpha: 0.12),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: const Icon(
                  Icons.waves_rounded,
                  size: 16,
                  color: Color(0xFFFFA000),
                ),
              ),
              const SizedBox(width: 8),
              const Expanded(
                child: Text(
                  'SPATIAL REVERB',
                  style: TextStyle(
                    color: Colors.white,
                    fontSize: 12,
                    fontWeight: FontWeight.bold,
                    letterSpacing: 0.5,
                  ),
                ),
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: const Color(0xFF0E1219),
                  borderRadius: BorderRadius.circular(4),
                  border: Border.all(
                    color: isEnabled
                        ? const Color(0xFFFFA000).withValues(alpha: 0.5)
                        : Colors.white24,
                  ),
                ),
                child: Text(
                  isEnabled
                      ? '${chSettings.reverbType.label.toUpperCase()} (${chSettings.dryWetPercent.toInt()}%)'
                      : 'OFF',
                  style: TextStyle(
                    color: isEnabled ? const Color(0xFFFFA000) : Colors.white38,
                    fontSize: 11,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 10),

          // Open Per-Channel Reverb Rack Shortcut
          SizedBox(
            width: double.infinity,
            child: OutlinedButton.icon(
              onPressed: () {
                showDialog(
                  context: context,
                  builder: (_) => ReverbSettingsModal(initialChannel: targetCh),
                );
              },
              icon: const Icon(Icons.tune_rounded, size: 14, color: Color(0xFFFFA000)),
              label: Text(
                'Open Output CH $targetCh Reverb Rack',
                style: const TextStyle(
                  fontSize: 11,
                  fontWeight: FontWeight.bold,
                  color: Color(0xFFFFA000),
                ),
              ),
              style: OutlinedButton.styleFrom(
                side: BorderSide(color: const Color(0xFFFFA000).withValues(alpha: 0.5)),
                padding: const EdgeInsets.symmetric(vertical: 8),
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
                backgroundColor: const Color(0xFF131923),
              ),
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

  Widget _buildBassManagementCard(BuildContext context, SpeakerNode speaker) {
    final bmState = ref.watch(bassManagementProvider);
    final isLfe = bmState.lfeChannel == speaker.channel;

    return Theme(
      data: Theme.of(context).copyWith(dividerColor: Colors.transparent),
      child: Container(
        margin: const EdgeInsets.only(top: 8),
        decoration: BoxDecoration(
          color: const Color(0xFF1E2632).withValues(alpha: 0.8),
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isLfe
                ? const Color(0xFFFF5722).withValues(alpha: 0.35)
                : Colors.white.withValues(alpha: 0.1),
          ),
        ),
        child: Material(
          color: Colors.transparent,
          child: ExpansionTile(
            collapsedIconColor: Colors.white54,
          iconColor: const Color(0xFFFF5722),
          title: Row(
            children: [
              Container(
                width: 30,
                height: 30,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  color: const Color(0xFFFF5722).withValues(alpha: 0.12),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: const Icon(Icons.speaker_group_rounded, size: 16, color: Color(0xFFFF5722)),
              ),
              const SizedBox(width: 8),
              const Expanded(
                child: Text(
                  'BASS MANAGEMENT',
                  style: TextStyle(color: Colors.white, fontSize: 12, fontWeight: FontWeight.bold, letterSpacing: 0.5),
                ),
              ),
              if (bmState.isEnabled && isLfe)
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(color: const Color(0xFF0E1219), borderRadius: BorderRadius.circular(4)),
                  child: const Text('LFE', style: TextStyle(color: Color(0xFFFF5722), fontSize: 10, fontWeight: FontWeight.bold)),
                ),
            ],
          ),
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
              child: Column(
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      const Text('Set as LFE Subwoofer', style: TextStyle(color: Colors.white70, fontSize: 12)),
                      Switch(
                        value: isLfe,
                        activeColor: const Color(0xFFFF5722),
                        onChanged: (val) {
                          ref.read(bassManagementProvider.notifier).setLfeChannel(val ? speaker.channel : null);
                        },
                      ),
                    ],
                  ),
                  if (isLfe) ...[
                    const Divider(color: Colors.white10, height: 16),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        const Text('Enable Bass Mgmt', style: TextStyle(color: Colors.white70, fontSize: 12)),
                        Switch(
                          value: bmState.isEnabled,
                          activeColor: const Color(0xFFFF5722),
                          onChanged: (val) {
                            ref.read(bassManagementProvider.notifier).setEnabled(val);
                          },
                        ),
                      ],
                    ),
                    const SizedBox(height: 12),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        const Text('Crossover Freq', style: TextStyle(color: Colors.white70, fontSize: 12)),
                        Container(
                          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                          decoration: BoxDecoration(color: const Color(0xFF0E1219), borderRadius: BorderRadius.circular(4)),
                          child: Text('${bmState.crossoverFreq.toInt()} Hz', style: const TextStyle(color: Color(0xFFFF5722), fontSize: 12, fontWeight: FontWeight.bold)),
                        ),
                      ],
                    ),
                    SliderTheme(
                      data: SliderTheme.of(context).copyWith(
                        activeTrackColor: const Color(0xFFFF5722),
                        thumbColor: const Color(0xFFFF5722),
                        trackHeight: 2.0,
                      ),
                      child: Slider(
                        value: bmState.crossoverFreq,
                        min: 40.0,
                        max: 120.0,
                        divisions: 8,
                        onChanged: (val) {
                          ref.read(bassManagementProvider.notifier).setCrossoverFreq(val);
                        },
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ],
        ),
        ),
      ),
    );
  }
}
