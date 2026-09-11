import 'dart:async';
import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/core/utils/channel_dropdown_helper.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/trajectory_settings_modal.dart';

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
    final bool isObjectMode = widget.track.outputChannel == 4294967295;

    final engineState = ref.watch(engineStateProvider);
    final isPlaying = engineState.playingTrackIds.contains(widget.track.id);

    final hwChannelsAsync = ref.watch(hardwareChannelsProvider);
    
    final hwChannels = hwChannelsAsync.value ?? [];
    // 하드웨어 채널 이름 목록을 아직 못 받았으면(FutureProvider 로딩/실패) 엔진이 보고한
    // 실제 출력 채널 수를 쓴다. 예전에는 64로 폴백해서 2채널 장치에서도 64개 항목이 생성됐다.
    final int maxChannels = hwChannels.isNotEmpty
        ? hwChannels.length
        : (engineState.outputChannelCount > 0 ? engineState.outputChannelCount : 2);

    final List<DropdownMenuItem<String>> outputItems = [];
    final isMulti = _fileChannels != null && _fileChannels! > 2;
    final isMono = _fileChannels == 1;

    // 멀티채널 파일이 아니면(모노/스테레오) 각 하드웨어 채널을 모노 목적지로 제공한다.
    // 스테레오 파일을 모노 채널 하나로 보내면 엔진이 다운믹스해서 출력한다
    // (mixer.rs: `!output_stereo && ch_limit > 1` 분기).
    if (!isMulti) {
      for (int i = 0; i < maxChannels; i++) {
        outputItems.add(
          DropdownMenuItem(
            value: ChannelDropdownValueHelper.getMonoValue(i),
            child: Text(
              'Mono (Ch-${i + 1})',
              style: const TextStyle(fontSize: 12, color: Colors.white),
            ),
          ),
        );
      }
    }

    // 스테레오 파일은 위의 모노 선택지에 더해 스테레오 쌍 선택지도 제공한다.
    // 쌍의 두 채널이 모두 실재해야 하므로 i + 1 < maxChannels 범위만 생성한다.
    if (!isMono && !isMulti) {
      for (int i = 0; i + 1 < maxChannels; i++) {
        outputItems.add(
          DropdownMenuItem(
            value: ChannelDropdownValueHelper.getStereoValue(i),
            child: Text(
              'Stereo (Ch-${i + 1}/Ch-${i + 2})',
              style: const TextStyle(fontSize: 12, color: Colors.white),
            ),
          ),
        );
      }
    }

    // 멀티채널은 파일 채널 수가 전부 들어갈 수 있는 시작 위치만 생성한다.
    // 하드웨어 채널이 파일 채널 수보다 적어 들어갈 자리가 없으면, 트랙이 아예
    // 선택 불가가 되지 않도록 Ch-1 시작 항목 하나는 남긴다(엔진이 다운믹스한다).
    if (isMulti) {
      final int fileCh = _fileChannels!;
      final int lastStart = maxChannels - fileCh;
      if (lastStart >= 0) {
        for (int i = 0; i <= lastStart; i++) {
          outputItems.add(
            DropdownMenuItem(
              value: ChannelDropdownValueHelper.getMultiValue(i),
              child: Text(
                'N-Ch (Ch-${i + 1}~${i + fileCh})',
                style: const TextStyle(fontSize: 12, color: Colors.white),
              ),
            ),
          );
        }
      } else {
        outputItems.add(
          DropdownMenuItem(
            value: ChannelDropdownValueHelper.getMultiValue(0),
            child: Text(
              'N-Ch (Ch-1~$fileCh)',
              style: const TextStyle(fontSize: 12, color: Colors.white),
            ),
          ),
        );
      }
    }

    int currentKey = widget.track.outputChannel;
    String currentValue;
    if (isMulti) {
      currentValue = ChannelDropdownValueHelper.getMultiValue(currentKey);
    } else {
      currentValue = widget.track.outputStereo
          ? ChannelDropdownValueHelper.getStereoValue(currentKey)
          : ChannelDropdownValueHelper.getMonoValue(currentKey);
    }

    bool valueExists = outputItems.any((item) => item.value == currentValue);

    if (!valueExists && currentKey != 4294967295) {
      if (outputItems.isNotEmpty) {
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
      }
    }

    return Container(
      margin: const EdgeInsets.symmetric(vertical: 4, horizontal: 8),
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: AppColors.background.withValues(alpha: 0.35),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(
          color: widget.accentColor.withValues(alpha: 0.15),
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
                  // Row 2: Mode Segment
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Row(
                        children: [
                          SegmentedButton<bool>(
                            segments: const [
                              ButtonSegment<bool>(value: false, label: Text('BED', style: TextStyle(fontSize: 10))),
                              ButtonSegment<bool>(value: true, label: Text('3D OBJECT', style: TextStyle(fontSize: 10))),
                            ],
                            selected: {isObjectMode},
                            style: SegmentedButton.styleFrom(
                              backgroundColor: AppColors.cardSurfaceSolid,
                              selectedBackgroundColor: widget.accentColor.withValues(alpha: 0.2),
                              selectedForegroundColor: widget.accentColor,
                              side: BorderSide(color: widget.accentColor.withValues(alpha: 0.5)),
                            ),
                            onSelectionChanged: (Set<bool> newSelection) {
                              // BED <-> 3D OBJECT 전환 시 사용자가 고른 모노/스테레오
                              // 선택은 유지한다. 예전에는 양쪽 모두 false를 넘겨서
                              // BED로 돌아올 때마다 스테레오 설정이 모노로 리셋됐다.
                              final bool keepStereo = widget.track.outputStereo;
                              if (newSelection.first) {
                                widget.onOutputChanged?.call(4294967295, keepStereo);
                              } else {
                                widget.onOutputChanged?.call(0, keepStereo);
                              }
                            },
                          ),
                          const SizedBox(width: 8),
                          // Loop & Stream
                          IconButton(
                            icon: const Icon(Icons.all_inclusive),
                            color: widget.track.isLoop ? widget.accentColor : AppColors.darkGrey,
                            iconSize: 18,
                            onPressed: () => widget.onLoopChanged?.call(!widget.track.isLoop),
                          ),
                          IconButton(
                            icon: Icon(widget.track.isStreaming ? Icons.storage : Icons.memory),
                            color: widget.track.isStreaming ? widget.accentColor : AppColors.darkGrey,
                            iconSize: 18,
                            onPressed: () => widget.onStreamChanged?.call(!widget.track.isStreaming),
                          ),
                        ],
                      ),
                    ],
                  ),
                  const SizedBox(height: 4),
                  // Row 3: Output / Trajectory
                  if (!isObjectMode) ...[
                    Row(
                      children: [
                        const Text('Ext. Out: ', style: TextStyle(color: AppColors.textSecondary, fontSize: 12)),
                        DropdownButtonHideUnderline(
                          child: DropdownButton<String>(
                            value: currentValue,
                            items: outputItems,
                            dropdownColor: AppColors.cardSurfaceSolid,
                            style: const TextStyle(color: Colors.white, fontSize: 12),
                            onChanged: (val) {
                              if (val != null) {
                                final isStereo = ChannelDropdownValueHelper.isStereo(val) || ChannelDropdownValueHelper.isMulti(val);
                                final key = ChannelDropdownValueHelper.getChannel(val);
                                if (key != null) widget.onOutputChanged?.call(key, isStereo);
                              }
                            },
                          ),
                        ),
                      ],
                    ),
                  ] else ...[
                    Row(
                      children: [
                        const Icon(Icons.view_in_ar, size: 16, color: Colors.amberAccent),
                        const SizedBox(width: 4),
                        const Text('3D Spatial Routing Active', style: TextStyle(color: Colors.amberAccent, fontSize: 12, fontWeight: FontWeight.bold)),
                        const Spacer(),
                        ElevatedButton(
                          style: ElevatedButton.styleFrom(backgroundColor: widget.accentColor, padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 0)),
                          onPressed: () {
                            showDialog(
                              context: context,
                              builder: (context) => TrajectorySettingsModal(trackId: widget.track.id),
                            );
                          },
                          child: const Text('3D Trajectory', style: TextStyle(color: Colors.black, fontSize: 10)),
                        )
                      ],
                    ),
                  ],
                  const SizedBox(height: 4),
                  // Row 4: Volume
                  Row(
                    children: [
                      Expanded(
                        child: SliderTheme(
                          data: SliderThemeData(
                            activeTrackColor: widget.accentColor,
                            inactiveTrackColor: AppColors.darkGrey,
                            thumbColor: widget.accentColor,
                            trackHeight: 2.0,
                            thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6.0),
                          ),
                          child: TweenAnimationBuilder<double>(
                            duration: const Duration(milliseconds: 100),
                            curve: Curves.easeOut,
                            tween: Tween<double>(
                              begin: _localVolume ?? widget.track.volume,
                              end: _localVolume ?? widget.track.volume,
                            ),
                            builder: (context, animVolume, child) {
                              return Slider(
                                value: animVolume.clamp(0.0, 1.0),
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
                              );
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
