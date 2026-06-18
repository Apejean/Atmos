import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';

class TrackCard extends ConsumerStatefulWidget {
  final TrackConfig track;
  final Color accentColor;
  final VoidChannel? onPlay;
  final VoidChannel? onStop;
  final VoidChannel? onDelete;
  final ValueChanged<double>? onVolumeChanged;
  final ValueChanged<bool>? onLoopChanged;
  final ValueChanged<String>? onNameChanged;
  final void Function(int channel, bool isStereo)? onOutputChanged;

  const TrackCard({
    super.key,
    required this.track,
    required this.accentColor,
    this.onPlay,
    this.onStop,
    this.onDelete,
    this.onVolumeChanged,
    this.onLoopChanged,
    this.onNameChanged,
    this.onOutputChanged,
  });

  @override
  ConsumerState<TrackCard> createState() => _TrackCardState();
}

class _TrackCardState extends ConsumerState<TrackCard> {
  late TextEditingController _nameController;
  late FocusNode _nameFocusNode;

  @override
  void initState() {
    super.initState();
    _nameController = TextEditingController(text: widget.track.name);
    _nameFocusNode = FocusNode();
    _nameFocusNode.addListener(() {
      if (!_nameFocusNode.hasFocus &&
          _nameController.text != widget.track.name) {
        widget.onNameChanged?.call(_nameController.text);
      }
    });
  }

  @override
  void didUpdateWidget(TrackCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.track.name != widget.track.name &&
        _nameController.text != widget.track.name) {
      _nameController.text = widget.track.name;
    }
  }

  @override
  void dispose() {
    _nameFocusNode.dispose();
    _nameController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final engineState = ref.watch(engineStateProvider);
    final isPlaying = engineState.playingTrackIds.contains(widget.track.id);

    final config = ref.watch(configProvider);

    final List<DropdownMenuItem<String>> outputItems = [];
    if (config != null) {
      final sortedMonoKeys = config.monoConfigs.keys.toList()..sort();
      for (final key in sortedMonoKeys) {
        final setting = config.monoConfigs[key]!;
        if (setting.enabled) {
          final displayCh = key + 1;
          final displayName = setting.customName.isNotEmpty
              ? '$displayCh (${setting.customName})'
              : '$displayCh';
          outputItems.add(
            DropdownMenuItem(
              value: 'mono_$key',
              child: Text(
                'Mono $displayName',
                style: const TextStyle(fontSize: 12, color: Colors.white),
              ),
            ),
          );
        }
      }

      final sortedStereoKeys = config.stereoConfigs.keys.toList()..sort();
      for (final key in sortedStereoKeys) {
        final setting = config.stereoConfigs[key]!;
        if (setting.enabled) {
          final displayCh = key + 1;
          final displayName = setting.customName.isNotEmpty
              ? '$displayCh/${displayCh + 1} (${setting.customName})'
              : '$displayCh/${displayCh + 1}';
          outputItems.add(
            DropdownMenuItem(
              value: 'stereo_$key',
              child: Text(
                'Stereo $displayName',
                style: const TextStyle(fontSize: 12, color: Colors.white),
              ),
            ),
          );
        }
      }
    }

    final currentKey = widget.track.outputChannel;
    final currentValue = widget.track.outputStereo
        ? 'stereo_$currentKey'
        : 'mono_$currentKey';

    final bool valueExists = outputItems.any(
      (item) => item.value == currentValue,
    );
    if (!valueExists && config != null) {
      final displayCh = currentKey + 1;
      outputItems.add(
        DropdownMenuItem(
          value: currentValue,
          child: Text(
            widget.track.outputStereo
                ? 'Stereo $displayCh/${displayCh + 1} (Disabled)'
                : 'Mono $displayCh (Disabled)',
            style: const TextStyle(fontSize: 12, color: AppColors.danger),
          ),
        ),
      );
    }

    return Container(
      margin: const EdgeInsets.symmetric(vertical: 4, horizontal: 8),
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: AppColors.background.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: widget.accentColor.withValues(alpha: 0.3)),
      ),
      child: IntrinsicHeight(
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // Row 1: Controls & Name
                  Row(
                    children: [
                      Padding(
                        padding: const EdgeInsets.only(right: 4.0),
                        child: IconButton(
                          icon: Icon(isPlaying ? Icons.stop : Icons.play_arrow),
                          color: isPlaying
                              ? AppColors.danger
                              : AppColors.success,
                          iconSize: 22,
                          padding: EdgeInsets.zero,
                          constraints: const BoxConstraints(
                            minWidth: 32,
                            minHeight: 32,
                          ),
                          onPressed: () {
                            if (isPlaying) {
                              widget.onStop?.call();
                            } else {
                              widget.onPlay?.call();
                            }
                          },
                          tooltip: isPlaying ? '정지' : '재생',
                        ),
                      ),
                      Expanded(
                        child: TextField(
                          controller: _nameController,
                          focusNode: _nameFocusNode,
                          style: const TextStyle(
                            color: AppColors.textPrimary,
                            fontSize: 14,
                          ),
                          decoration: const InputDecoration(
                            isDense: true,
                            contentPadding: EdgeInsets.symmetric(
                              horizontal: 8,
                              vertical: 8,
                            ),
                            border: InputBorder.none,
                          ),
                          onSubmitted: widget.onNameChanged,
                        ),
                      ),
                      IconButton(
                        icon: const Icon(Icons.close),
                        color: AppColors.danger,
                        iconSize: 18,
                        onPressed: widget.onDelete,
                        tooltip: '삭제',
                      ),
                    ],
                  ),
                  // Row 2: Volume & Settings
                  Row(
                    children: [
                      Expanded(
                        child: SliderTheme(
                          data: SliderThemeData(
                            activeTrackColor: widget.accentColor,
                            inactiveTrackColor: AppColors.darkGrey,
                            thumbColor: widget.accentColor,
                            trackHeight: 2.0,
                            thumbShape: const RoundSliderThumbShape(
                              enabledThumbRadius: 6.0,
                            ),
                          ),
                          child: Slider(
                            value: widget.track.volume,
                            min: 0.0,
                            max: 1.0,
                            onChanged: widget.onVolumeChanged,
                          ),
                        ),
                      ),
                      Text(
                        '${(widget.track.volume * 100).toInt()}%',
                        style: const TextStyle(
                          color: AppColors.textSecondary,
                          fontSize: 12,
                        ),
                      ),
                      const SizedBox(width: 8),
                      IconButton(
                        icon: Icon(
                          Icons.all_inclusive,
                          shadows: widget.track.isLoop
                              ? [
                                  Shadow(
                                    color: widget.accentColor,
                                    blurRadius: 8,
                                  ),
                                ]
                              : null,
                        ),
                        color: widget.track.isLoop
                            ? widget.accentColor
                            : AppColors.darkGrey,
                        iconSize: 20,
                        onPressed: () =>
                            widget.onLoopChanged?.call(!widget.track.isLoop),
                        tooltip: '무한 루프 (BGM)',
                      ),
                      const SizedBox(width: 8),
                      const Text(
                        'Ext. Out: ',
                        style: TextStyle(
                          color: AppColors.textSecondary,
                          fontSize: 12,
                        ),
                      ),
                      DropdownButtonHideUnderline(
                        child: DropdownButton<String>(
                          value: currentValue,
                          items: outputItems,
                          icon: const Icon(
                            Icons.arrow_drop_down,
                            color: AppColors.textSecondary,
                            size: 16,
                          ),
                          dropdownColor: AppColors.cardSurface,
                          style: const TextStyle(
                            color: Colors.white,
                            fontSize: 12,
                          ),
                          onChanged: (val) {
                            if (val != null) {
                              final isStereo = val.startsWith('stereo_');
                              final keyStr = val.replaceFirst(
                                isStereo ? 'stereo_' : 'mono_',
                                '',
                              );
                              final key = int.tryParse(keyStr);
                              if (key != null) {
                                widget.onOutputChanged?.call(key, isStereo);
                              }
                            }
                          },
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

typedef VoidChannel = void Function();
