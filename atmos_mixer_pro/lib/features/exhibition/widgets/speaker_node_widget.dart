import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';

class SpeakerNodeWidget extends ConsumerStatefulWidget {
  final SpeakerNode node;
  final bool isDuplicateChannel;
  final ValueChanged<int> onChannelChanged;
  final VoidCallback onDelete;
  final VoidCallback? onEdit;
  final Color? roomColor;

  const SpeakerNodeWidget({
    super.key,
    required this.node,
    this.isDuplicateChannel = false,
    required this.onChannelChanged,
    required this.onDelete,
    this.onEdit,
    this.roomColor,
  });

  @override
  ConsumerState<SpeakerNodeWidget> createState() => _SpeakerNodeWidgetState();
}

class _SpeakerNodeWidgetState extends ConsumerState<SpeakerNodeWidget> {
  final ValueNotifier<double> _levelNotifier = ValueNotifier<double>(0.0);
  bool _isHovered = false;

  @override
  void dispose() {
    _levelNotifier.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    ref.listen(vuStreamProvider, (previous, next) {
      final levels = next.value;
      if (levels != null && mounted && widget.node.channel >= 0 && widget.node.channel < levels.length) {
        final newLevel = levels[widget.node.channel];
        double currentLevel = _levelNotifier.value;
        if (newLevel > currentLevel) {
          currentLevel = newLevel;
        } else {
          currentLevel -= 0.1; // Decay rate
          if (currentLevel < 0) currentLevel = 0;
        }
        _levelNotifier.value = currentLevel;
      }
    });

    final baseColor = widget.roomColor ?? AppColors.primaryNeon;
    final borderColor = widget.isDuplicateChannel
        ? Colors.redAccent
        : baseColor.withValues(alpha: 0.5);

    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Coordinate & 3D Orientation Badge
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
          decoration: BoxDecoration(
            color: Colors.black87,
            borderRadius: BorderRadius.circular(4),
            border: Border.all(color: Colors.white24, width: 0.5),
          ),
          child: Text(
            'X: ${widget.node.x.round()}, Y: ${widget.node.y.round()} | Z: ${widget.node.heightZ.toStringAsFixed(1)}m, ∠${widget.node.pitchTilt.toInt()}°',
            style: TextStyle(
              fontSize: 9,
              color: widget.isDuplicateChannel
                  ? Colors.redAccent
                  : Colors.white70,
              fontWeight: FontWeight.bold,
            ),
          ),
        ),
        const SizedBox(height: 4),
        MouseRegion(
          onEnter: (_) => setState(() => _isHovered = true),
          onExit: (_) => setState(() => _isHovered = false),
          child: AnimatedScale(
            scale: _isHovered ? 1.05 : 1.0,
            duration: const Duration(milliseconds: 200),
            curve: Curves.easeOutCubic,
            child: ValueListenableBuilder<double>(
              valueListenable: _levelNotifier,
              builder: (context, currentLevel, child) {
                final glowOpacity = (currentLevel * 0.8).clamp(0.0, 1.0);
                final glowRadius = currentLevel * 30.0;
                
                return Container(
                  width: 100,
                  height: 120,
                  decoration: BoxDecoration(
                    color: Colors.black54,
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(
                      color: _isHovered ? baseColor : borderColor,
                      width: widget.isDuplicateChannel ? 2.5 : (_isHovered ? 2.0 : 1.0),
                    ),
                    boxShadow: [
                      if (glowOpacity > 0 || _isHovered)
                        BoxShadow(
                          color: baseColor.withValues(
                            alpha: _isHovered ? 0.8 : glowOpacity,
                          ),
                          blurRadius: _isHovered ? 15.0 : glowRadius,
                          spreadRadius: _isHovered ? 2.0 : glowRadius / 2,
                        ),
                      if (widget.isDuplicateChannel)
                        const BoxShadow(
                          color: Colors.redAccent,
                          blurRadius: 8,
                          spreadRadius: 1,
                        ),
                    ],
                  ),
                  child: child,
                );
              },
              child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  if (widget.isDuplicateChannel)
                    const Padding(
                      padding: EdgeInsets.only(left: 4.0),
                      child: Tooltip(
                        message: '중복된 채널이 지정되었습니다!',
                        child: Icon(
                          Icons.warning_amber_rounded,
                          size: 16,
                          color: Colors.redAccent,
                        ),
                      ),
                    )
                  else
                    const SizedBox(width: 20),
                  ValueListenableBuilder<double>(
                    valueListenable: _levelNotifier,
                    builder: (context, currentLevel, child) {
                      return Icon(
                        Icons.speaker,
                        size: 38,
                        color: currentLevel > 0.1 ? baseColor : Colors.white70,
                      );
                    },
                  ),
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (widget.onEdit != null)
                        InkWell(
                          onTap: widget.onEdit,
                          child: const Padding(
                            padding: EdgeInsets.only(bottom: 12.0, right: 4.0),
                            child: Icon(
                              Icons.tune,
                              size: 15,
                              color: AppColors.primaryNeon,
                            ),
                          ),
                        ),
                      InkWell(
                        onTap: widget.onDelete,
                        child: const Padding(
                          padding: EdgeInsets.only(bottom: 12.0, right: 4.0),
                          child: Icon(
                            Icons.close,
                            size: 16,
                            color: Colors.redAccent,
                          ),
                        ),
                      ),
                    ],
                  ),
                ],
              ),
              const SizedBox(height: 4),
              Container(
                height: 30,
                padding: const EdgeInsets.symmetric(horizontal: 8),
                decoration: BoxDecoration(
                  color: Colors.black87,
                  borderRadius: BorderRadius.circular(6),
                  border: widget.isDuplicateChannel
                      ? Border.all(color: Colors.redAccent, width: 1.0)
                      : null,
                ),
                child: DropdownButtonHideUnderline(
                  child: Consumer(
                    builder: (context, ref, child) {
                      final maxChannels = ref.watch(
                        engineStateProvider.select((s) => s.outputChannelCount),
                      );
                      // Ensure current channel is within valid range
                      final currentValue = widget.node.channel < maxChannels
                          ? widget.node.channel
                          : 0;

                      return DropdownButton<int>(
                        value: currentValue,
                        dropdownColor: AppColors.background,
                        icon: const Icon(
                          Icons.arrow_drop_down,
                          size: 16,
                          color: Colors.white54,
                        ),
                        style: const TextStyle(
                          fontSize: 12,
                          color: Colors.white,
                        ),
                        items: List.generate(
                          maxChannels > 0 ? maxChannels : 2,
                          (index) {
                            return DropdownMenuItem<int>(
                              value: index,
                              child: Text('Ch ${index + 1}'),
                            );
                          },
                        ),
                        onChanged: (val) {
                          if (val != null) {
                            widget.onChannelChanged(val);
                          }
                        },
                      );
                    },
                  ),
                ),
              ),
            ],
          ),
            ),
          ),
        ),
      ],
    );
  }
}
