import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/core/utils/channel_routing.dart';

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

    return Stack(
      clipBehavior: Clip.none,
      alignment: Alignment.center,
      children: [
        // The main 100x120 container
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
              child: Stack(
                children: [
                  if (widget.onEdit != null)
                    Positioned(
                      top: 4,
                      left: 4,
                      child: InkWell(
                        onTap: widget.onEdit,
                        child: const Padding(
                          padding: EdgeInsets.all(4.0),
                          child: Icon(
                            Icons.tune,
                            size: 16,
                            color: AppColors.primaryNeon,
                          ),
                        ),
                      ),
                    ),
                  Positioned(
                    top: 4,
                    right: 4,
                    child: InkWell(
                      onTap: widget.onDelete,
                      child: const Padding(
                        padding: EdgeInsets.all(4.0),
                        child: Icon(
                          Icons.close,
                          size: 16,
                          color: Colors.redAccent,
                        ),
                      ),
                    ),
                  ),
                  Center(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Stack(
                          alignment: Alignment.center,
                          clipBehavior: Clip.none,
                          children: [
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
                            if (widget.isDuplicateChannel)
                              const Positioned(
                                left: -22,
                                child: Tooltip(
                                  message: '중복된 채널이 지정되었습니다!',
                                  child: Icon(
                                    Icons.warning_amber_rounded,
                                    size: 16,
                                    color: Colors.redAccent,
                                  ),
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
                      ? Border.all(color: Colors.redAccent, width: 1.0)
                      : null,
                ),
                child: DropdownButtonHideUnderline(
                  child: Consumer(
                    builder: (context, ref, child) {
                      final outputChannels = ref.watch(outputChannelsProvider);

                      // loading: 최초 조회/장치 전환 중. 마지막으로 알려진 목록이
                      // 있으면 그대로 쓰고, 없으면 로딩 표시.
                      if (outputChannels.status == OutputChannelsStatus.loading &&
                          outputChannels.channelNames.isEmpty) {
                        return const Text(
                          '채널 조회 중...',
                          style: TextStyle(fontSize: 11, color: Colors.white54),
                        );
                      }

                      // noDevice: 라우팅할 출력 장치 자체가 없음.
                      if (outputChannels.status == OutputChannelsStatus.noDevice) {
                        return const Text(
                          '출력 장치 없음',
                          style: TextStyle(fontSize: 11, color: Colors.redAccent),
                        );
                      }

                      // error: 조회 실패. lastKnownGoodChannelNames로 폴백하고
                      // 사용자에게 무음으로 실패하지 않았음을 알린다.
                      final channelNames = outputChannels.status == OutputChannelsStatus.error
                          ? (outputChannels.lastKnownGoodChannelNames ?? const [])
                          : outputChannels.channelNames;

                      if (channelNames.isEmpty) {
                        return const Text(
                          '채널 없음',
                          style: TextStyle(fontSize: 11, color: Colors.white54),
                        );
                      }

                      // 저장된 channel이 현재 하드웨어 채널 수를 넘는 경우
                      // (장치가 더 작은 것으로 바뀐 경우) 조용히 0으로 리셋하지
                      // 않고 사용자가 인지할 수 있게 별도 항목으로 표시한다.
                      // 저장된 채널이 목록에 없는 경우는 두 가지다: 하드웨어
                      // 범위를 벗어났거나(더 작은 장치로 교체), 드라이버 내부
                      // 가상 채널이라 목록에서 걸러졌거나. 둘 다 DropdownButton의
                      // "값에 해당하는 항목이 정확히 하나" assert를 깨뜨리므로
                      // 별도 항목으로 보존해야 한다.
                      final selectable =
                          physicalOutputChannelIndices(channelNames);
                      final isOutOfRange =
                          !selectable.contains(widget.node.channel);

                      return Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          if (outputChannels.status == OutputChannelsStatus.error)
                            const Padding(
                              padding: EdgeInsets.only(right: 4.0),
                              child: Tooltip(
                                message: '채널 인식 실패 — 마지막으로 확인된 목록 표시 중',
                                child: Icon(
                                  Icons.warning_amber_rounded,
                                  size: 14,
                                  color: Colors.orangeAccent,
                                ),
                              ),
                            ),
                          DropdownButton<int>(
                            value: widget.node.channel,
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
                            items: [
                              // 물리적으로 연결 가능한 채널만 제시한다.
                              // 인덱스는 하드웨어 인덱스를 유지한다.
                              ...selectable.map((index) {
                                // 라벨은 다른 UI(스피커 인스펙터, FX 튜닝,
                                // 트랙 라우팅)와 같은 `Ch-N (하드웨어 이름)`
                                // 규약을 쓴다. 화면마다 같은 채널이 같은 이름
                                // 으로 보여야 한다.
                                var channelName =
                                    channelDisplayName(index, channelNames);
                                // Truncate if too long to prevent UI breaking
                                if (channelName.length > 25) {
                                  channelName = '${channelName.substring(0, 22)}...';
                                }

                                return DropdownMenuItem<int>(
                                  value: index,
                                  child: Text(channelName),
                                );
                              }),
                              // 범위를 벗어난 기존 값은 목록에 없으면 드롭다운이
                              // 크래시하므로, 별도 항목으로 보존해 사용자가 직접
                              // 재선택하게 한다.
                              if (isOutOfRange)
                                DropdownMenuItem<int>(
                                  value: widget.node.channel,
                                  child: Text(
                                    channelOutOfRangeName(widget.node.channel),
                                    style: const TextStyle(color: Colors.redAccent),
                                  ),
                                ),
                            ],
                            onChanged: (val) {
                              if (val != null) {
                                widget.onChannelChanged(val);
                              }
                            },
                          ),
                        ],
                      );
                    },
                  ),
                ),
              ),
            ],
          ),
        ),
        ],
      ),
            ),
          ),
        ),
        // Coordinate & 3D Orientation Badge positioned outside
        Positioned(
          bottom: -28,
          child: Container(
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
        ),
      ],
    );

  }
}
