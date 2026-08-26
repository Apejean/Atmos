import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';

class SpeakerInspectorPanel extends ConsumerStatefulWidget {
  final SpeakerNode? selectedSpeaker;

  const SpeakerInspectorPanel({super.key, this.selectedSpeaker});

  @override
  ConsumerState<SpeakerInspectorPanel> createState() =>
      _SpeakerInspectorPanelState();
}

class _SpeakerInspectorPanelState extends ConsumerState<SpeakerInspectorPanel> {
  @override
  Widget build(BuildContext context) {
    if (widget.selectedSpeaker == null) return const SizedBox.shrink();

    return Container(
      width: 280,
      margin: const EdgeInsets.only(top: 16, right: 16, bottom: 16),
      decoration: BoxDecoration(
        color: AppColors.cardSurfaceSolid.withValues(alpha: 0.8),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: AppColors.lightGrey, width: 1.5),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header
          Container(
            padding: const EdgeInsets.all(12),
            decoration: const BoxDecoration(
              border: Border(bottom: BorderSide(color: AppColors.lightGrey)),
            ),
            child: Row(
              children: [
                const Icon(Icons.volume_up, color: AppColors.primaryBlue, size: 20),
                const SizedBox(width: 8),
                const Text(
                  'SPEAKER INSPECTOR',
                  style: TextStyle(
                    color: AppColors.primaryBlue,
                    fontSize: 14,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ],
            ),
          ),
          
          Expanded(
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(16.0),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                    decoration: BoxDecoration(
                      border: Border.all(color: AppColors.lightGrey),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Row(
                      children: [
                        const Text(
                          'Selected Speaker: ',
                          style: TextStyle(color: Colors.white70, fontSize: 12),
                        ),
                        Text(
                          'S${widget.selectedSpeaker!.id.padLeft(2, '0')}',
                          style: const TextStyle(color: AppColors.primaryBlue, fontSize: 12, fontWeight: FontWeight.bold),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: 24),
                  
                  // Height Slider
                  _buildSectionTitle('HEIGHT (Z)'),
                  const SizedBox(height: 16),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      SizedBox(
                        height: 100,
                        child: RotatedBox(
                          quarterTurns: 3,
                          child: SliderTheme(
                            data: SliderThemeData(
                              trackHeight: 4,
                              activeTrackColor: AppColors.primaryBlue,
                              inactiveTrackColor: Colors.white24,
                              thumbColor: Colors.white,
                              thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
                            ),
                            child: Slider(
                              value: widget.selectedSpeaker!.heightZ,
                              min: 0.0,
                              max: 5.0,
                              onChanged: (val) {
                                _updateSpeaker(widget.selectedSpeaker!.copyWith(heightZ: val));
                              },
                            ),
                          ),
                        ),
                      ),
                      Container(
                        width: 100,
                        padding: const EdgeInsets.all(8),
                        decoration: BoxDecoration(
                          border: Border.all(color: AppColors.lightGrey),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Column(
                          children: [
                            const Icon(Icons.speaker, color: Colors.white54, size: 32),
                            const Icon(Icons.arrow_upward, color: AppColors.primaryBlue, size: 24),
                            const SizedBox(height: 8),
                            Container(
                              width: double.infinity,
                              padding: const EdgeInsets.symmetric(vertical: 4),
                              decoration: BoxDecoration(
                                border: Border.all(color: AppColors.primaryBlue),
                                borderRadius: BorderRadius.circular(4),
                              ),
                              child: Text(
                                '${widget.selectedSpeaker!.heightZ.toStringAsFixed(1)} m',
                                textAlign: TextAlign.center,
                                style: const TextStyle(color: Colors.white, fontSize: 12),
                              ),
                            )
                          ],
                        ),
                      )
                    ],
                  ),
                  
                  const SizedBox(height: 24),
                  const Divider(color: AppColors.lightGrey),
                  const SizedBox(height: 16),
                  
                  // Tilt Angle
                  _buildSectionTitle('TILT ANGLE'),
                  const SizedBox(height: 8),
                  SliderTheme(
                    data: SliderThemeData(
                      trackHeight: 4,
                      activeTrackColor: AppColors.primaryBlue,
                      inactiveTrackColor: Colors.white24,
                      thumbColor: Colors.white,
                      thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
                    ),
                    child: Slider(
                      value: widget.selectedSpeaker!.pitchTilt,
                      min: 0.0,
                      max: 90.0,
                      onChanged: (val) {
                        _updateSpeaker(widget.selectedSpeaker!.copyWith(pitchTilt: val));
                      },
                    ),
                  ),
                  
                  const SizedBox(height: 24),
                  const Divider(color: AppColors.lightGrey),
                  const SizedBox(height: 16),
                  
                  // Pan Angle
                  _buildSectionTitle('PAN ANGLE'),
                  const SizedBox(height: 8),
                  SliderTheme(
                    data: SliderThemeData(
                      trackHeight: 4,
                      activeTrackColor: AppColors.primaryBlue,
                      inactiveTrackColor: Colors.white24,
                      thumbColor: Colors.white,
                      thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
                    ),
                    child: Slider(
                      value: widget.selectedSpeaker!.rotation,
                      min: -180.0,
                      max: 180.0,
                      onChanged: (val) {
                        _updateSpeaker(widget.selectedSpeaker!.copyWith(rotation: val));
                      },
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildSectionTitle(String title) {
    return Text(
      title,
      style: const TextStyle(
        color: Colors.white70,
        fontSize: 13,
        fontWeight: FontWeight.w600,
        letterSpacing: 1.2,
      ),
    );
  }

  void _updateSpeaker(SpeakerNode updatedNode) {
    ref.read(speakerLayoutProvider.notifier).updateSpeaker(updatedNode);
  }
}
