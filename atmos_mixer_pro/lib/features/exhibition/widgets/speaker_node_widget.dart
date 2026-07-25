import 'dart:async';
import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/core/theme/colors.dart';

class SpeakerNodeWidget extends StatefulWidget {
  final SpeakerNode node;
  final ValueChanged<int> onChannelChanged;
  final VoidCallback onDelete;

  const SpeakerNodeWidget({
    super.key,
    required this.node,
    required this.onChannelChanged,
    required this.onDelete,
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

    return Container(
      width: 100,
      height: 120,
      decoration: BoxDecoration(
        color: Colors.black54,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: AppColors.primaryNeon.withOpacity(0.5)),
        boxShadow: [
          if (glowOpacity > 0)
            BoxShadow(
              color: AppColors.primaryNeon.withOpacity(glowOpacity),
              blurRadius: glowRadius,
              spreadRadius: glowRadius / 2,
            ),
        ],
      ),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const SizedBox(width: 24), // Balance the close button
              Icon(
                Icons.speaker,
                size: 40,
                color: _currentLevel > 0.1 ? AppColors.primaryNeon : Colors.white70,
              ),
              InkWell(
                onTap: widget.onDelete,
                child: const Padding(
                  padding: EdgeInsets.only(bottom: 16.0, right: 4.0),
                  child: Icon(Icons.close, size: 16, color: Colors.redAccent),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Container(
            height: 30,
            padding: const EdgeInsets.symmetric(horizontal: 8),
            decoration: BoxDecoration(
              color: Colors.black87,
              borderRadius: BorderRadius.circular(6),
            ),
            child: DropdownButtonHideUnderline(
              child: DropdownButton<int>(
                value: widget.node.channel,
                dropdownColor: AppColors.background,
                icon: const Icon(Icons.arrow_drop_down, size: 16, color: Colors.white54),
                style: const TextStyle(fontSize: 12, color: Colors.white),
                items: List.generate(24, (index) {
                  return DropdownMenuItem<int>(
                    value: index,
                    child: Text('Ch ${index + 1}'),
                  );
                }),
                onChanged: (val) {
                  if (val != null) {
                    widget.onChannelChanged(val);
                    // try {
                    //   rust_api.apiPlayTestNoise(channel: val);
                    // } catch (e) {
                    //   print('apiPlayTestNoise not available yet: \$e');
                    // }
                  }
                },
              ),
            ),
          ),
        ],
      ),
    );
  }
}
