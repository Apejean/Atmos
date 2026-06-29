import 'dart:async';
import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;

class TrackCard extends ConsumerStatefulWidget {
  final TrackConfig track;
  final Color accentColor;
  final VoidChannel? onPlay;
  final VoidChannel? onStop;
  final VoidChannel? onDelete;
  final ValueChanged<double>? onVolumeChanged;
  final ValueChanged<double>? onVolumeChangeEnd;
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
    this.onVolumeChangeEnd,
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
  double? _localVolume;
  Timer? _debounce;
  int? _fileChannels;

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
    _loadFileChannels(widget.track.filePath);
  }

  Future<void> _loadFileChannels(String filePath) async {
    try {
      final ch = await rust_api.apiGetAudioFileChannels(filePath: filePath);
      if (mounted) {
        setState(() {
          _fileChannels = ch;
        });
      }
    } catch (e) {
      // Ignore
    }
  }

  void _onNameChanged(String value) {
    if (_debounce?.isActive ?? false) _debounce!.cancel();
    _debounce = Timer(const Duration(milliseconds: 500), () {
      if (_nameController.text != widget.track.name) {
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
    if (oldWidget.track.filePath != widget.track.filePath) {
      _loadFileChannels(widget.track.filePath);
    }
  }

  @override
  void dispose() {
    _debounce?.cancel();
    _nameFocusNode.dispose();
    _nameController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final engineState = ref.watch(engineStateProvider);
    final isPlaying = engineState.playingTrackIds.contains(widget.track.id);

    final config = ref.watch(configProvider);
    final hwChannelsAsync = ref.watch(hardwareChannelsProvider);
    int maxChannels = 256;
    if (config != null) {
      if (config.deviceName != null &&
          GlobalDeviceCache.channels.containsKey(config.deviceName)) {
        maxChannels = GlobalDeviceCache.channels[config.deviceName]!.length;
      } else if (hwChannelsAsync.value != null &&
          hwChannelsAsync.value!.isNotEmpty) {
        maxChannels = hwChannelsAsync.value!.length;
      }

      int maxConfigured = 0;
      if (config.monoConfigs.isNotEmpty) {
        maxConfigured =
            config.monoConfigs.keys.reduce((a, b) => a > b ? a : b) + 1;
      }
      if (config.stereoConfigs.isNotEmpty) {
        final maxStereo =
            config.stereoConfigs.keys.reduce((a, b) => a > b ? a : b) + 1;
        if (maxStereo > maxConfigured) maxConfigured = maxStereo;
      }
      if (maxConfigured > maxChannels) {
        maxChannels = maxConfigured;
      }
    }

    final List<DropdownMenuItem<String>> outputItems = [];
    if (config != null) {
      final isMulti = _fileChannels != null && _fileChannels! > 2;
      final isMono = _fileChannels == 1;

      if (!isMulti) {
        final sortedMonoKeys = config.monoConfigs.keys.toList()..sort();
        for (final key in sortedMonoKeys) {
          final setting = config.monoConfigs[key]!;
          if (setting.enabled) {
            final realCh1 = key - 1;
            if (realCh1 >= maxChannels) continue;
            final name1 = setting.customName.isNotEmpty
                ? '$key (${setting.customName} L)'
                : '$key';
            outputItems.add(
              DropdownMenuItem(
                value: 'mono_$realCh1',
                child: Text(
                  '1-Ch (모노) $name1',
                  style: const TextStyle(fontSize: 12, color: Colors.white),
                ),
              ),
            );

            final realCh2 = key;
            if (realCh2 < maxChannels) {
              final displayCh2 = key + 1;
              final name2 = setting.customName.isNotEmpty
                  ? '$displayCh2 (${setting.customName} R)'
                  : '$displayCh2';
              outputItems.add(
                DropdownMenuItem(
                  value: 'mono_$realCh2',
                  child: Text(
                    '1-Ch (모노) $name2',
                    style: const TextStyle(fontSize: 12, color: Colors.white),
                  ),
                ),
              );
            }
          }
        }
      }

      if (!isMono && !isMulti) {
        final sortedStereoKeys = config.stereoConfigs.keys.toList()..sort();
        for (final key in sortedStereoKeys) {
          final setting = config.stereoConfigs[key]!;
          if (setting.enabled) {
            final realCh = key - 1;
            if (realCh >= maxChannels) continue;
            final displayCh2 = key + 1;
            final displayName = setting.customName.isNotEmpty
                ? '$key/$displayCh2 (${setting.customName})'
                : '$key/$displayCh2';
            outputItems.add(
              DropdownMenuItem(
                value: 'stereo_$realCh',
                child: Text(
                  '2-Ch (Stereo) $displayName',
                  style: const TextStyle(fontSize: 12, color: Colors.white),
                ),
              ),
            );
          }
        }
      }

      if (isMulti) {
        final sortedMultiKeys = config.multiConfigs.keys.toList()..sort();
        for (final key in sortedMultiKeys) {
          final setting = config.multiConfigs[key]!;
          if (setting.enabled) {
            final realCh = key - 1;
            if (realCh >= maxChannels) continue;
            final endCh = math.min(key - 1 + _fileChannels!, maxChannels);
            var labelText = 'N-Ch (다채널) Ch $key~$endCh ($_fileChannels ch)';
            if (setting.customName.isNotEmpty) {
              labelText += ' (${setting.customName})';
            }
            outputItems.add(
              DropdownMenuItem(
                value: 'multi_$realCh',
                child: Text(
                  labelText,
                  style: const TextStyle(fontSize: 12, color: Colors.white),
                ),
              ),
            );
          }
        }
      }
    }

    int currentKey = widget.track.outputChannel;
    String currentValue;
    if (config != null && _fileChannels != null && _fileChannels! > 2) {
      currentValue = widget.track.outputStereo ? 'multi_$currentKey' : 'mono_$currentKey';
    } else {
      currentValue = widget.track.outputStereo ? 'stereo_$currentKey' : 'mono_$currentKey';
    }

    bool valueExists = outputItems.any((item) => item.value == currentValue);

    if (!valueExists && config != null) {
      if (currentKey >= maxChannels && outputItems.isNotEmpty) {
        final firstVal = outputItems.first.value;
        if (firstVal != null) {
          currentValue = firstVal;
          final isStereo = firstVal.startsWith('stereo_') || firstVal.startsWith('multi_');
          final keyStr = firstVal
              .replaceFirst('stereo_', '')
              .replaceFirst('mono_', '')
              .replaceFirst('multi_', '');
          currentKey = int.tryParse(keyStr) ?? 0;

          WidgetsBinding.instance.addPostFrameCallback((_) {
            widget.onOutputChanged?.call(currentKey, isStereo);
          });
        }
      } else {
        final displayCh = currentKey + 1;
        outputItems.add(
          DropdownMenuItem(
            value: currentValue,
            child: Text(
              widget.track.outputStereo
                  ? 'N-Ch $displayCh~ (Disabled)'
                  : '1-Ch $displayCh (Disabled)',
              style: const TextStyle(fontSize: 12, color: AppColors.danger),
            ),
          ),
        );
      }
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
                          onChanged: _onNameChanged,
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
                  // Row 2: Settings (Loop & Routing)
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Row(
                        children: [
                          Container(
                            decoration: BoxDecoration(
                              color: widget.track.isLoop
                                  ? widget.accentColor.withValues(alpha: 0.15)
                                  : Colors.transparent,
                              borderRadius: BorderRadius.circular(8),
                              border: Border.all(
                                color: widget.track.isLoop
                                    ? widget.accentColor
                                    : Colors.transparent,
                                width: 1.5,
                              ),
                            ),
                            child: IconButton(
                              icon: const Icon(Icons.all_inclusive),
                              color: widget.track.isLoop
                                  ? widget.accentColor
                                  : AppColors.darkGrey,
                              iconSize: 20,
                              padding: EdgeInsets.zero,
                              constraints: const BoxConstraints(
                                minWidth: 32,
                                minHeight: 32,
                              ),
                              onPressed: () => widget.onLoopChanged?.call(
                                !widget.track.isLoop,
                              ),
                              tooltip: '무한 루프 (BGM)',
                            ),
                          ),
                          const SizedBox(width: 4),
                          Text(
                            '무한 루프',
                            style: TextStyle(
                              color: widget.track.isLoop
                                  ? widget.accentColor
                                  : AppColors.textSecondary,
                              fontSize: 12,
                            ),
                          ),
                        ],
                      ),
                      Row(
                        children: [
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
                              dropdownColor: AppColors.cardSurfaceSolid,
                              style: const TextStyle(
                                color: Colors.white,
                                fontSize: 12,
                              ),
                              onChanged: (val) {
                                if (val != null) {
                                  final isStereo = val.startsWith('stereo_') || val.startsWith('multi_');
                                  final keyStr = val
                                      .replaceFirst('stereo_', '')
                                      .replaceFirst('mono_', '')
                                      .replaceFirst('multi_', '');
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
                  const SizedBox(height: 4),
                  // Row 3: Volume
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
                            value: _localVolume ?? widget.track.volume,
                            min: 0.0,
                            max: 1.0,
                            onChanged: (v) {
                              setState(() => _localVolume = v);
                              widget.onVolumeChanged?.call(v);
                            },
                            onChangeEnd: (v) {
                              setState(() => _localVolume = null);
                              widget.onVolumeChangeEnd?.call(v);
                            },
                          ),
                        ),
                      ),
                      SizedBox(
                        width: 40,
                        child: Text(
                          '${((_localVolume ?? widget.track.volume) * 100).toInt()}%',
                          textAlign: TextAlign.right,
                          style: const TextStyle(
                            color: AppColors.textSecondary,
                            fontSize: 12,
                          ),
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
