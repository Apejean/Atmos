import 'dart:async';
import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/core/utils/channel_dropdown_helper.dart';
import 'package:atmos_mixer_pro/core/utils/channel_routing.dart';

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

  /// 라우팅 항목이 0개일 때 사용자에게 보일 이유. `outputChannelsProvider`가
  /// 조회 중/장치 없음/조회 실패를 구분해 주므로 그대로 문구로 옮긴다.
  String _emptyOutputReason(OutputChannelsState channels) {
    switch (channels.status) {
      case OutputChannelsStatus.loading:
        return '출력 채널 조회 중...';
      case OutputChannelsStatus.noDevice:
        return '연결된 출력 장치 없음';
      case OutputChannelsStatus.error:
        return '채널 조회 실패: ${channels.errorMessage ?? '알 수 없는 오류'}';
      case OutputChannelsStatus.ready:
        // 채널은 있는데 항목이 0개면 Output Config에서 해당 그룹을 전부
        // 닫아둔 경우다(예: 멀티 파일인데 Multi 그룹이 모두 비활성).
        return 'Output Config에서 사용할 채널 그룹을 열어주세요';
    }
  }

  @override
  Widget build(BuildContext context) {
    // 엔진은 무한루프 트랙을 저장값과 상관없이 디스크에서 읽는다(play_track_from·api_preload_all_sounds의 is_loop 조건).
    final bool readsFromDisk = widget.track.isLoop || widget.track.isStreaming;

    final engineState = ref.watch(engineStateProvider);
    final isPlaying = engineState.playingTrackIds.contains(widget.track.id);

    // Ext. Out 드롭다운 항목은 환경설정의 "트랙별 출력 채널 매핑"과 동일한
    // 순수 함수로 만든다. 같은 저장값이 두 화면에서 같은 라벨로 보이는 것을
    // 이 공유로 보장한다(라벨을 여기서 직접 만들지 않는다).
    final outputChannels = ref.watch(outputChannelsProvider);
    final config = ref.watch(configProvider);

    final List<ChannelRoutingItem> routingItems = config == null
        ? const []
        : buildChannelRoutingItems(
            channelNames: outputChannels.channelNames,
            config: config,
            fileChannels: _fileChannels,
          );

    final List<DropdownMenuItem<String>> outputItems = [
      for (final item in routingItems)
        DropdownMenuItem(
          value: item.value,
          child: Text(
            item.label,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(
              fontSize: 12,
              // 부분 출력(파일 채널 수 > 하드웨어 채널 수)은 정상 동작이지만
              // 사용자가 무음으로 오해하지 않도록 색으로 구분한다.
              color: item.isPartialOutput
                  ? Colors.amberAccent
                  : Colors.white,
            ),
          ),
        ),
    ];

    // 저장된 값을 드롭다운 값 형식으로 되돌린다. 멀티채널 파일은 Multi
    // 인코딩을, 그 밖에는 outputStereo 플래그에 따라 Stereo/Mono 인코딩을 쓴다.
    final bool isMulti = _fileChannels != null && _fileChannels! > 2;

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

    // 예전에 3D OBJECT로 저장한 트랙(출력 4294967295)도 여기서 첫 출력으로 옮겨진다.
    if (!valueExists) {
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
                  // Row 2: Loop & Memory/Disk
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Row(
                        children: [
                          IconButton(
                            icon: const Icon(Icons.all_inclusive),
                            color: widget.track.isLoop ? widget.accentColor : AppColors.darkGrey,
                            iconSize: 18,
                            onPressed: () => widget.onLoopChanged?.call(!widget.track.isLoop),
                          ),
                          Tooltip(
                            message: widget.track.isLoop
                                ? '무한루프 트랙은 항상 디스크에서 읽습니다'
                                : readsFromDisk
                                    ? '재생하면서 디스크에서 읽습니다. 누르면 메모리로 바꿉니다'
                                    : '파일 전체를 미리 메모리에 올려 둡니다. 누르면 디스크로 바꿉니다',
                            child: TextButton.icon(
                              style: TextButton.styleFrom(
                                // 방 색이 어두워도 이름은 읽혀야 하므로 글자는 무채색, 아이콘만 방 색으로 켠다.
                                foregroundColor: AppColors.textSecondary,
                                iconColor: readsFromDisk ? widget.accentColor : AppColors.textSecondary,
                                disabledForegroundColor: Colors.white38,
                                disabledIconColor: Colors.white38,
                              ),
                              icon: Icon(readsFromDisk ? Icons.storage : Icons.memory, size: 18),
                              label: Text(readsFromDisk ? '디스크' : '메모리', style: const TextStyle(fontSize: 12)),
                              onPressed: widget.track.isLoop
                                  ? null
                                  : () => widget.onStreamChanged?.call(!widget.track.isStreaming),
                            ),
                          ),
                        ],
                      ),
                    ],
                  ),
                  const SizedBox(height: 4),
                  // Row 3: Output
                  Row(
                    children: [
                      const Text('Ext. Out: ', style: TextStyle(color: AppColors.textSecondary, fontSize: 12)),
                      // 항목이 하나도 없으면 왜 없는지 알려준다. 예전에는 빈
                      // 드롭다운만 남아 장치 미인식과 조회 실패를 구분할 수
                      // 없었다(무음 실패).
                      if (outputItems.isEmpty)
                        Expanded(
                          child: Text(
                            _emptyOutputReason(outputChannels),
                            style: const TextStyle(
                              color: Colors.orangeAccent,
                              fontSize: 11,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        )
                      else
                      // 장치 채널 이름이 붙은 라벨은 길어서, 남은 폭을 채우고 넘치면 줄인다.
                      Expanded(
                        child: DropdownButtonHideUnderline(
                        child: DropdownButton<String>(
                          isExpanded: true,
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
                      ),
                    ],
                  ),
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
