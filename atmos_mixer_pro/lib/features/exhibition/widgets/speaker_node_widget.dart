import 'dart:async';
import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';

class SpeakerNodeWidget extends StatefulWidget {
  final SpeakerNode node;
  final bool isDuplicateChannel;
  final ValueChanged<int> onChannelChanged;
  final VoidCallback onDelete;
  final Color? roomColor;

  const SpeakerNodeWidget({
    super.key,
    required this.node,
    this.isDuplicateChannel = false,
    required this.onChannelChanged,
    required this.onDelete,
    this.roomColor,
  });

  @override
  State<SpeakerNodeWidget> createState() => _SpeakerNodeWidgetState();
}

class _SpeakerNodeWidgetState extends State<SpeakerNodeWidget> {
  StreamSubscription<List<double>>? _vuSubscription;
  double _currentLevel = 0.0;

  @override
  void initState() {
    super.initState();
    _vuSubscription = rust_api.apiCreateVuStream().listen((levels) {
      if (mounted && widget.node.channel >= 0 && widget.node.channel < levels.length) {
        final newLevel = levels[widget.node.channel];
        setState(() {
          if (newLevel > _currentLevel) {
            _currentLevel = newLevel;
          } else {
            _currentLevel -= 0.1; // Decay rate
            if (_currentLevel < 0) _currentLevel = 0;
          }
        });
      }
    });
  }

  @override
  void dispose() {
    _vuSubscription?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    // 0.0 to 1.0 glow intensity
    final glowOpacity = (_currentLevel * 0.8).clamp(0.0, 1.0);
    final glowRadius = _currentLevel * 30.0;

    final baseColor = widget.roomColor ?? AppColors.primaryNeon;
    final borderColor = widget.isDuplicateChannel
        ? Colors.orangeAccent
        : baseColor.withOpacity(0.5);

    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Coordinate Badge
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
          decoration: BoxDecoration(
            color: Colors.black87,
            borderRadius: BorderRadius.circular(4),
            border: Border.all(color: Colors.white24, width: 0.5),
          ),
          child: Text(
            'X: ${widget.node.x.round()}, Y: ${widget.node.y.round()}',
            style: TextStyle(
              fontSize: 10,
              color: widget.isDuplicateChannel ? Colors.orangeAccent : Colors.white70,
              fontWeight: FontWeight.bold,
            ),
          ),
        ),
        const SizedBox(height: 4),
        Container(
          width: 100,
          height: 120,
          decoration: BoxDecoration(
            color: Colors.black54,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(
              color: borderColor,
              width: widget.isDuplicateChannel ? 2.5 : 1.0,
            ),
            boxShadow: [
              if (glowOpacity > 0)
                BoxShadow(
                  color: baseColor.withOpacity(glowOpacity),
                  blurRadius: glowRadius,
                  spreadRadius: glowRadius / 2,
                ),
              if (widget.isDuplicateChannel)
                const BoxShadow(
                  color: Colors.orangeAccent,
                  blurRadius: 8,
                  spreadRadius: 1,
                ),
            ],
          ),
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
                        child: Icon(Icons.warning_amber_rounded, size: 16, color: Colors.orangeAccent),
                      ),
                    )
                  else
                    const SizedBox(width: 20),
                  Icon(
                    Icons.speaker,
                    size: 38,
                    color: _currentLevel > 0.1 ? baseColor : Colors.white70,
                  ),
                  InkWell(
                    onTap: widget.onDelete,
                    child: const Padding(
                      padding: EdgeInsets.only(bottom: 12.0, right: 4.0),
                      child: Icon(Icons.close, size: 16, color: Colors.redAccent),
                    ),
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
                      ? Border.all(color: Colors.orangeAccent, width: 1.0)
                      : null,
                ),
                child: DropdownButtonHideUnderline(
                  child: Consumer(
                    builder: (context, ref, child) {
                      final maxChannels = ref.watch(engineStateProvider.select((s) => s.outputChannelCount));
                      // Ensure current channel is within valid range
                      final currentValue = widget.node.channel < maxChannels ? widget.node.channel : 0;
                      
                      return DropdownButton<int>(
                        value: currentValue,
                        dropdownColor: AppColors.background,
                        icon: const Icon(Icons.arrow_drop_down, size: 16, color: Colors.white54),
                        style: const TextStyle(fontSize: 12, color: Colors.white),
                        items: List.generate(maxChannels > 0 ? maxChannels : 2, (index) {
                          return DropdownMenuItem<int>(
                            value: index,
                            child: Text('Ch ${index + 1}'),
                          );
                        }),
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
      ],
    );
  }
}
