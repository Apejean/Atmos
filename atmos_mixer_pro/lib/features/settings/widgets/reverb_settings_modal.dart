import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/spatial_reverb_state.dart';

import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';

class ReverbSettingsModal extends ConsumerStatefulWidget {
  final int initialChannel;

  const ReverbSettingsModal({super.key, this.initialChannel = 0});

  @override
  ConsumerState<ReverbSettingsModal> createState() => _ReverbSettingsModalState();
}

class _ReverbSettingsModalState extends ConsumerState<ReverbSettingsModal> {
  @override
  void initState() {
    super.initState();
    if (widget.initialChannel > 0) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) {
          ref.read(spatialReverbProvider.notifier).selectChannel(widget.initialChannel);
        }
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final reverbState = ref.watch(spatialReverbProvider);
    final reverb = reverbState.currentSettings;
    final notifier = ref.read(spatialReverbProvider.notifier);
    final speakers = ref.watch(speakerLayoutProvider);

    return Dialog(
      backgroundColor: Colors.transparent,
      insetPadding: const EdgeInsets.symmetric(horizontal: 20, vertical: 20),
      child: Container(
        width: 860,
        padding: const EdgeInsets.fromLTRB(28, 20, 28, 24),
        decoration: BoxDecoration(
          color: const Color(0xFF1E2127), // Ultra minimalist matte dark grey background
          borderRadius: BorderRadius.circular(14),
          border: Border.all(color: const Color(0xFF323640), width: 1.2),
          boxShadow: [
            BoxShadow(
              color: Colors.black.withValues(alpha: 0.7),
              blurRadius: 32,
              spreadRadius: 6,
              offset: const Offset(0, 8),
            ),
          ],
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Top Header: Title, Active Channel Indicator & Close
            Row(
              children: [
                GestureDetector(
                  onTap: notifier.toggleEnabled,
                  child: Container(
                    width: 10,
                    height: 10,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      color: reverb.isEnabled ? const Color(0xFFFFA000) : Colors.white24,
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                const Text(
                  'SPATIAL REVERB RACK',
                  style: TextStyle(
                    color: Colors.white,
                    fontSize: 13,
                    fontWeight: FontWeight.w800,
                    letterSpacing: 1.5,
                  ),
                ),
                const SizedBox(width: 12),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                  decoration: BoxDecoration(
                    color: reverbState.selectedChannel == 0
                        ? const Color(0xFF2C3240)
                        : const Color(0xFFFFA000).withValues(alpha: 0.2),
                    borderRadius: BorderRadius.circular(4),
                    border: Border.all(
                      color: reverbState.selectedChannel == 0
                          ? const Color(0xFF3F4758)
                          : const Color(0xFFFFA000).withValues(alpha: 0.6),
                      width: 1.0,
                    ),
                  ),
                  child: Text(
                    reverbState.selectedChannel == 0
                        ? 'ALL OUTPUTS'
                        : 'OUTPUT CH ${reverbState.selectedChannel}',
                    style: TextStyle(
                      color: reverbState.selectedChannel == 0 ? Colors.white70 : const Color(0xFFFFA000),
                      fontSize: 10,
                      fontWeight: FontWeight.bold,
                      letterSpacing: 0.8,
                    ),
                  ),
                ),
                const Spacer(),
                IconButton(
                  padding: EdgeInsets.zero,
                  constraints: const BoxConstraints(minWidth: 24, minHeight: 24),
                  icon: const Icon(Icons.close_rounded, color: Colors.white54, size: 18),
                  onPressed: () => Navigator.of(context).pop(),
                ),
              ],
            ),
            const SizedBox(height: 12),

            // Channel Selector Bar
            _buildChannelSelectorBar(reverbState, notifier, speakers),
            const SizedBox(height: 16),

            // 1. Top Segmented Buttons Bar ([ROOM] [HALL] [PLATE] [CHAMBER] [CATHEDRAL] [AMBIENCE])
            _buildSegmentedTypeBar(reverb, notifier),
            const SizedBox(height: 36),

            // 2. Main Row: 5 Minimalist Rotary Knobs
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                // 1. PRE-DELAY
                _buildMinimalKnob(
                  label: 'PRE-DELAY',
                  valueText: '${reverb.preDelayMs.toInt()} ms',
                  normalizedValue: (reverb.preDelayMs / 100.0).clamp(0.0, 1.0),
                  onChanged: (v) => notifier.updatePreDelay(v * 100.0),
                ),

                // 2. SIZE
                _buildMinimalKnob(
                  label: 'SIZE',
                  valueText: (reverb.roomSize / 200.0).toStringAsFixed(1),
                  normalizedValue: ((reverb.roomSize - 50.0) / (2000.0 - 50.0)).clamp(0.0, 1.0),
                  onChanged: (v) => notifier.updateRoomSize(50.0 + v * 1950.0),
                ),

                // 3. DECAY
                _buildMinimalKnob(
                  label: 'DECAY',
                  valueText: '${reverb.decayTime.toStringAsFixed(1)} s',
                  normalizedValue: ((reverb.decayTime - 0.2) / 19.8).clamp(0.0, 1.0),
                  onChanged: (v) => notifier.updateDecayTime(0.2 + v * 19.8),
                ),

                // 4. DAMPING
                _buildMinimalKnob(
                  label: 'DAMPING',
                  valueText: '${(reverb.damp * 0.16).toStringAsFixed(1)} kHz',
                  normalizedValue: (reverb.damp / 100.0).clamp(0.0, 1.0),
                  onChanged: (v) => notifier.updateDamp(v * 100.0),
                ),

                // 5. MIX
                _buildMinimalKnob(
                  label: 'MIX',
                  valueText: '${reverb.dryWetPercent.toInt()}%',
                  normalizedValue: (reverb.dryWetPercent / 100.0).clamp(0.0, 1.0),
                  onChanged: (v) => notifier.updateDryWet(v * 100.0),
                ),
              ],
            ),
            const SizedBox(height: 42),

            // 3. Bottom Row: 2 Clean Filter Sliders with ON/OFF Switches
            Row(
              children: [
                // LOW CUT
                Expanded(
                  child: _buildFilterSliderUnit(
                    label: 'LOW CUT',
                    valueText: '${reverb.loCutFreq.toInt()} Hz',
                    isEnabled: reverb.loCutEnabled,
                    value: reverb.loCutFreq,
                    min: 20.0,
                    max: 1000.0,
                    onToggle: notifier.toggleLoCut,
                    onChanged: (v) => notifier.updateLoCutFreq(v),
                  ),
                ),
                const SizedBox(width: 48),

                // HIGH CUT
                Expanded(
                  child: _buildFilterSliderUnit(
                    label: 'HIGH CUT',
                    valueText: '${(reverb.hiCutFreq / 1000).toStringAsFixed(1)} kHz',
                    isEnabled: reverb.hiCutEnabled,
                    value: reverb.hiCutFreq,
                    min: 1000.0,
                    max: 20000.0,
                    onToggle: notifier.toggleHiCut,
                    onChanged: (v) => notifier.updateHiCutFreq(v),
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  // ---------------------------------------------------------------------------
  // 0. Channel Selector Bar ([ALL], [CH 1], [CH 2], ... [Copy to All])
  // ---------------------------------------------------------------------------
  Widget _buildChannelSelectorBar(
    SpatialReverbState state,
    SpatialReverbNotifier notifier,
    List<dynamic> speakers,
  ) {
    // Collect available output channels (speaker.channel is 0-indexed, output is 1-indexed)
    final channels = <int>{};
    for (final spk in speakers) {
      if (spk.channel != null && spk.channel >= 0) {
        channels.add((spk.channel as int) + 1);
      }
    }
    if (channels.isEmpty) {
      for (int i = 1; i <= 12; i++) {
        channels.add(i);
      }
    }
    final sortedChannels = channels.toList()..sort();

    return Container(
      height: 38,
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
      decoration: BoxDecoration(
        color: const Color(0xFF16181D),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: const Color(0xFF2B303C), width: 1.0),
      ),
      child: Row(
        children: [
          // ALL CHANNELS Button
          GestureDetector(
            onTap: () => notifier.selectChannel(0),
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
              decoration: BoxDecoration(
                color: state.selectedChannel == 0
                    ? const Color(0xFF333842)
                    : Colors.transparent,
                borderRadius: BorderRadius.circular(5),
                border: Border.all(
                  color: state.selectedChannel == 0
                      ? const Color(0xFFFFA000).withValues(alpha: 0.8)
                      : Colors.transparent,
                  width: 1.0,
                ),
              ),
              child: Text(
                'ALL',
                style: TextStyle(
                  color: state.selectedChannel == 0 ? const Color(0xFFFFA000) : Colors.white60,
                  fontSize: 11,
                  fontWeight: state.selectedChannel == 0 ? FontWeight.w800 : FontWeight.w600,
                  letterSpacing: 0.6,
                ),
              ),
            ),
          ),
          const SizedBox(width: 6),
          Container(width: 1, height: 18, color: const Color(0xFF2B303C)),
          const SizedBox(width: 6),

          // Scrollable Speaker Channels List
          Expanded(
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              physics: const BouncingScrollPhysics(),
              child: Row(
                children: sortedChannels.map((ch) {
                  final isSelected = state.selectedChannel == ch;
                  final hasCustom = state.channelSettings.containsKey(ch);
                  final chSettings = state.getSettingsForChannel(ch);

                  return Padding(
                    padding: const EdgeInsets.only(right: 6),
                    child: GestureDetector(
                      onTap: () => notifier.selectChannel(ch),
                      child: Container(
                        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                        decoration: BoxDecoration(
                          color: isSelected
                              ? const Color(0xFF2D323E)
                              : hasCustom
                                  ? const Color(0xFF1F232B)
                                  : Colors.transparent,
                          borderRadius: BorderRadius.circular(5),
                          border: Border.all(
                            color: isSelected
                                ? const Color(0xFFFFA000)
                                : hasCustom
                                    ? const Color(0xFF3F4656)
                                    : const Color(0xFF262B35),
                            width: 1.0,
                          ),
                        ),
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Text(
                              'CH $ch',
                              style: TextStyle(
                                color: isSelected
                                    ? Colors.white
                                    : hasCustom
                                        ? Colors.white70
                                        : Colors.white38,
                                fontSize: 11,
                                fontWeight: isSelected ? FontWeight.w800 : FontWeight.w500,
                              ),
                            ),
                            if (hasCustom && chSettings.isEnabled) ...[
                              const SizedBox(width: 4),
                              Container(
                                width: 5,
                                height: 5,
                                decoration: BoxDecoration(
                                  shape: BoxShape.circle,
                                  color: isSelected ? const Color(0xFFFFA000) : Colors.white38,
                                ),
                              ),
                            ],
                          ],
                        ),
                      ),
                    ),
                  );
                }).toList(),
              ),
            ),
          ),

          // Copy to All Channels Button
          if (state.selectedChannel != 0) ...[
            const SizedBox(width: 6),
            Container(width: 1, height: 18, color: const Color(0xFF2B303C)),
            const SizedBox(width: 6),
            Tooltip(
              message: 'Copy current Output Channel ${state.selectedChannel} settings to all channels',
              child: GestureDetector(
                onTap: () {
                  notifier.copyToAll();
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(
                      content: Text('Applied Output Channel ${state.selectedChannel} Reverb to all channels'),
                      duration: const Duration(seconds: 2),
                      backgroundColor: const Color(0xFF2A2E38),
                    ),
                  );
                },
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                  decoration: BoxDecoration(
                    color: const Color(0xFF262A34),
                    borderRadius: BorderRadius.circular(5),
                    border: Border.all(color: const Color(0xFF3B4150), width: 1.0),
                  ),
                  child: const Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(Icons.copy_all_rounded, size: 12, color: Colors.white70),
                      SizedBox(width: 4),
                      Text(
                        'Copy to All',
                        style: TextStyle(
                          color: Colors.white70,
                          fontSize: 10,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }

  // ---------------------------------------------------------------------------
  // 1. Top Segmented Buttons Bar
  // ---------------------------------------------------------------------------
  Widget _buildSegmentedTypeBar(SpatialReverbSettings reverb, SpatialReverbNotifier notifier) {
    return Container(
      height: 44,
      decoration: BoxDecoration(
        color: const Color(0xFF282C34),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: const Color(0xFF3A3F4D), width: 1.0),
      ),
      child: Row(
        children: ReverbType.values.asMap().entries.map((entry) {
          final idx = entry.key;
          final type = entry.value;
          final isSelected = reverb.reverbType == type;

          return Expanded(
            child: GestureDetector(
              onTap: () => notifier.setReverbType(type),
              child: Container(
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  color: isSelected ? const Color(0xFF333842) : Colors.transparent,
                  border: Border(
                    right: idx < ReverbType.values.length - 1
                        ? const BorderSide(color: Color(0xFF3A3F4D), width: 1.0)
                        : BorderSide.none,
                  ),
                ),
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Text(
                      type.label.toUpperCase(),
                      style: TextStyle(
                        color: isSelected ? Colors.white : const Color(0xFF8A93A4),
                        fontSize: 12,
                        fontWeight: isSelected ? FontWeight.w800 : FontWeight.w600,
                        letterSpacing: 0.8,
                      ),
                    ),
                    if (isSelected) ...[
                      const SizedBox(height: 3),
                      Container(
                        width: 24,
                        height: 2.0,
                        color: Colors.white,
                      ),
                    ],
                  ],
                ),
              ),
            ),
          );
        }).toList(),
      ),
    );
  }

  // ---------------------------------------------------------------------------
  // 2. Minimalist Rotary Knob (Dieter Rams style clean circle + pointer line)
  // ---------------------------------------------------------------------------
  Widget _buildMinimalKnob({
    required String label,
    required String valueText,
    required double normalizedValue,
    required ValueChanged<double> onChanged,
  }) {
    return SizedBox(
      width: 110,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            label,
            style: const TextStyle(
              color: Color(0xFF9AA4B2),
              fontSize: 12,
              fontWeight: FontWeight.w700,
              letterSpacing: 1.0,
            ),
          ),
          const SizedBox(height: 14),
          GestureDetector(
            onVerticalDragUpdate: (details) {
              final delta = -details.primaryDelta! / 120.0;
              final newVal = (normalizedValue + delta).clamp(0.0, 1.0);
              onChanged(newVal);
            },
            child: SizedBox(
              width: 78,
              height: 78,
              child: CustomPaint(
                painter: _DieterRamsKnobPainter(value: normalizedValue),
              ),
            ),
          ),
          const SizedBox(height: 12),
          Text(
            valueText,
            style: const TextStyle(
              color: Colors.white,
              fontSize: 13,
              fontWeight: FontWeight.w700,
            ),
          ),
        ],
      ),
    );
  }

  // ---------------------------------------------------------------------------
  // 3. Filter Slider Unit (LOW CUT / HIGH CUT) with ON/OFF Pill
  // ---------------------------------------------------------------------------
  Widget _buildFilterSliderUnit({
    required String label,
    required String valueText,
    required bool isEnabled,
    required double value,
    required double min,
    required double max,
    required VoidCallback onToggle,
    required ValueChanged<double> onChanged,
  }) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Text(
              label,
              style: const TextStyle(
                color: Color(0xFF9AA4B2),
                fontSize: 12,
                fontWeight: FontWeight.w700,
                letterSpacing: 1.0,
              ),
            ),
            const SizedBox(width: 8),
            Text(
              '($valueText)',
              style: TextStyle(
                color: isEnabled ? Colors.white70 : Colors.white24,
                fontSize: 11,
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
        ),
        const SizedBox(height: 10),
        Row(
          children: [
            // ON / OFF Toggle Pill
            GestureDetector(
              onTap: onToggle,
              child: Container(
                height: 26,
                padding: const EdgeInsets.symmetric(horizontal: 8),
                decoration: BoxDecoration(
                  color: isEnabled ? const Color(0xFFE2E8F0) : const Color(0xFF282C34),
                  borderRadius: BorderRadius.circular(4),
                  border: Border.all(color: const Color(0xFF4A5160)),
                ),
                alignment: Alignment.center,
                child: Text(
                  isEnabled ? 'ON' : 'OFF',
                  style: TextStyle(
                    color: isEnabled ? const Color(0xFF1E2127) : const Color(0xFF8A93A4),
                    fontSize: 10,
                    fontWeight: FontWeight.w900,
                    letterSpacing: 0.5,
                  ),
                ),
              ),
            ),
            const SizedBox(width: 10),

            // Minimalist Slider
            Expanded(
              child: SizedBox(
                height: 26,
                child: SliderTheme(
                  data: SliderThemeData(
                    trackHeight: 2.5,
                    activeTrackColor: isEnabled ? const Color(0xFFCBD5E1) : const Color(0xFF3A3F4D),
                    inactiveTrackColor: const Color(0xFF2E333D),
                    thumbColor: isEnabled ? const Color(0xFFE2E8F0) : const Color(0xFF4A5160),
                    thumbShape: const _DieterRamsSliderThumbShape(),
                    overlayShape: const RoundSliderOverlayShape(overlayRadius: 8.0),
                  ),
                  child: Slider(
                    value: value.clamp(min, max),
                    min: min,
                    max: max,
                    onChanged: isEnabled ? onChanged : null,
                  ),
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 4),
        // Frequency Scale Markings (Concept 2 style)
        const Padding(
          padding: EdgeInsets.only(left: 42, right: 4),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('20 Hz', style: TextStyle(color: Color(0xFF64748B), fontSize: 9, fontWeight: FontWeight.w600)),
              Text('100 Hz', style: TextStyle(color: Color(0xFF64748B), fontSize: 9, fontWeight: FontWeight.w600)),
              Text('1 kHz', style: TextStyle(color: Color(0xFF64748B), fontSize: 9, fontWeight: FontWeight.w600)),
              Text('10 kHz', style: TextStyle(color: Color(0xFF64748B), fontSize: 9, fontWeight: FontWeight.w600)),
              Text('20 kHz', style: TextStyle(color: Color(0xFF64748B), fontSize: 9, fontWeight: FontWeight.w600)),
            ],
          ),
        ),
      ],
    );
  }
}

// -----------------------------------------------------------------------------
// Dieter Rams Minimalist Rotary Knob Painter
// -----------------------------------------------------------------------------
class _DieterRamsKnobPainter extends CustomPainter {
  final double value; // 0.0 ~ 1.0

  _DieterRamsKnobPainter({required this.value});

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final radius = size.width / 2;

    // 1. Knob Base Circle (Flat Dark Grey Matte)
    final basePaint = Paint()
      ..color = const Color(0xFF333742)
      ..style = PaintingStyle.fill;
    canvas.drawCircle(center, radius - 2, basePaint);

    // 2. Subtle Outer Border Ring
    final borderPaint = Paint()
      ..color = const Color(0xFF434957)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.5;
    canvas.drawCircle(center, radius - 2, borderPaint);

    // 3. Pointer Line Needle (Angle from 135 deg to 405 deg)
    const startAngle = 135 * math.pi / 180;
    const sweepAngle = 270 * math.pi / 180;
    final currentAngle = startAngle + sweepAngle * value;

    final needleStart = Offset(
      center.dx + (radius * 0.28) * math.cos(currentAngle),
      center.dy + (radius * 0.28) * math.sin(currentAngle),
    );
    final needleEnd = Offset(
      center.dx + (radius * 0.76) * math.cos(currentAngle),
      center.dy + (radius * 0.76) * math.sin(currentAngle),
    );

    final needlePaint = Paint()
      ..color = const Color(0xFFE2E8F0)
      ..strokeWidth = 2.5
      ..strokeCap = StrokeCap.round;
    canvas.drawLine(needleStart, needleEnd, needlePaint);
  }

  @override
  bool shouldRepaint(covariant _DieterRamsKnobPainter oldDelegate) {
    return oldDelegate.value != value;
  }
}

// -----------------------------------------------------------------------------
// Minimalist Rectangular Slider Thumb Shape
// -----------------------------------------------------------------------------
class _DieterRamsSliderThumbShape extends SliderComponentShape {
  const _DieterRamsSliderThumbShape();

  @override
  Size getPreferredSize(bool isEnabled, bool isDiscrete) => const Size(10, 18);

  @override
  void paint(
    PaintingContext context,
    Offset center, {
    required Animation<double> activationAnimation,
    required Animation<double> enableAnimation,
    required bool isDiscrete,
    required TextPainter labelPainter,
    required RenderBox parentBox,
    required SliderThemeData sliderTheme,
    required TextDirection textDirection,
    required double value,
    required double textScaleFactor,
    required Size sizeWithOverflow,
  }) {
    final canvas = context.canvas;
    final rect = Rect.fromCenter(center: center, width: 8.0, height: 18.0);
    final rrect = RRect.fromRectAndRadius(rect, const Radius.circular(2.5));

    final paint = Paint()
      ..color = sliderTheme.thumbColor ?? const Color(0xFFE2E8F0)
      ..style = PaintingStyle.fill;
    canvas.drawRRect(rrect, paint);

    final borderPaint = Paint()
      ..color = const Color(0xFF1E2127).withValues(alpha: 0.4)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.0;
    canvas.drawRRect(rrect, borderPaint);
  }
}
