import 'dart:async';
import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/core/utils/channel_dropdown_helper.dart';

class TrackCard extends ConsumerStatefulWidget {
  final TrackConfig track;
  final Color accentColor;
  final VoidChannel? onPlay;
  final VoidChannel? onStop;
  final VoidChannel? onDelete;
  final ValueChanged<double>? onVolumeChanged;
  final ValueChanged<double>? onVolumeChangeEnd;
  final ValueChanged<bool>? onLoopChanged;
  final ValueChanged<bool>? onStreamChanged;
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
    this.onStreamChanged,
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
                value: ChannelDropdownValueHelper.getMonoValue(realCh1),
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
                  value: ChannelDropdownValueHelper.getMonoValue(realCh2),
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
                value: ChannelDropdownValueHelper.getStereoValue(realCh),
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
                value: ChannelDropdownValueHelper.getMultiValue(realCh),
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
      currentValue = widget.track.outputStereo
          ? ChannelDropdownValueHelper.getMultiValue(currentKey)
          : ChannelDropdownValueHelper.getMonoValue(currentKey);
    } else {
      currentValue = widget.track.outputStereo
          ? ChannelDropdownValueHelper.getStereoValue(currentKey)
          : ChannelDropdownValueHelper.getMonoValue(currentKey);
    }

    bool valueExists = outputItems.any((item) => item.value == currentValue);

    if (!valueExists && config != null) {
      if (currentKey >= maxChannels && outputItems.isNotEmpty) {
        final firstVal = outputItems.first.value;
        if (firstVal != null) {
          currentValue = firstVal;
          final isStereo =
              ChannelDropdownValueHelper.isStereo(firstVal) ||
              ChannelDropdownValueHelper.isMulti(firstVal);
          currentKey = ChannelDropdownValueHelper.getChannel(firstVal) ?? 0;

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
        color: AppColors.background.withOpacity(0.35),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(
          color: widget.accentColor.withOpacity(0.15),
          width: 1.0,
        ),
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
                                  ? widget.accentColor.withOpacity(0.12)
                                  : Colors.transparent,
                              borderRadius: BorderRadius.circular(4),
                              border: Border.all(
                                color: widget.track.isLoop
                                    ? widget.accentColor.withOpacity(0.7)
                                    : Colors.white.withOpacity(0.05),
                                width: 1.0,
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
                          const SizedBox(width: 8),
                          Container(
                            decoration: BoxDecoration(
                              color: widget.track.isStreaming
                                  ? widget.accentColor.withOpacity(0.12)
                                  : Colors.transparent,
                              borderRadius: BorderRadius.circular(4),
                              border: Border.all(
                                color: widget.track.isStreaming
                                    ? widget.accentColor.withOpacity(0.7)
                                    : Colors.white.withOpacity(0.05),
                                width: 1.0,
                              ),
                            ),
                            child: IconButton(
                              icon: Icon(
                                widget.track.isStreaming
                                    ? Icons.storage
                                    : Icons.memory,
                              ),
                              color: widget.track.isStreaming
                                  ? widget.accentColor
                                  : AppColors.darkGrey,
                              iconSize: 20,
                              padding: EdgeInsets.zero,
                              constraints: const BoxConstraints(
                                minWidth: 32,
                                minHeight: 32,
                              ),
                              onPressed: () {
                                if (widget.track.isStreaming) {
                                  // Warning user before switching to memory preload
                                  showDialog(
                                    context: context,
                                    builder: (context) => AlertDialog(
                                      backgroundColor: AppColors.background,
                                      title: const Text(
                                        '경고: OOM 위험',
                                        style: TextStyle(
                                          color: AppColors.danger,
                                        ),
                                      ),
                                      content: const Text(
                                        '큰 파일을 메모리에 프리로드하면 OOM(메모리 부족) 문제가 발생할 수 있습니다.\n\n긴 BGM은 "디스크 스트리밍" 모드를 권장합니다. 계속하시겠습니까?',
                                        style: TextStyle(color: Colors.white70),
                                      ),
                                      actions: [
                                        TextButton(
                                          onPressed: () =>
                                              Navigator.of(context).pop(),
                                          child: const Text(
                                            '취소',
                                            style: TextStyle(
                                              color: Colors.white54,
                                            ),
                                          ),
                                        ),
                                        TextButton(
                                          onPressed: () {
                                            Navigator.of(context).pop();
                                            widget.onStreamChanged?.call(false);
                                          },
                                          child: const Text(
                                            '프리로드',
                                            style: TextStyle(
                                              color: AppColors.primaryNeon,
                                            ),
                                          ),
                                        ),
                                      ],
                                    ),
                                  );
                                } else {
                                  widget.onStreamChanged?.call(true);
                                }
                              },
                              tooltip: widget.track.isStreaming
                                  ? '디스크 스트리밍 모드'
                                  : '메모리 프리로드 모드',
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
                                  final isStereo =
                                      ChannelDropdownValueHelper.isStereo(
                                        val,
                                      ) ||
                                      ChannelDropdownValueHelper.isMulti(val);
                                  final key =
                                      ChannelDropdownValueHelper.getChannel(
                                        val,
                                      );
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
