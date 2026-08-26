import re

with open('lib/features/dashboard/widgets/track_card.dart', 'r') as f:
    content = f.read()

# We want to replace the `build` method
start_idx = content.find('  Widget build(BuildContext context) {')
end_idx = content.find('typedef VoidChannel = void Function();')

new_build = """  Widget build(BuildContext context) {
    final bool isObjectMode = widget.track.outputChannel == 4294967295;

    final engineState = ref.watch(engineStateProvider);
    final isPlaying = engineState.playingTrackIds.contains(widget.track.id);

    final config = ref.watch(configProvider);
    final hwChannelsAsync = ref.watch(hardwareChannelsProvider);
    
    final hwChannels = hwChannelsAsync.value ?? [];
    int maxChannels = hwChannels.isNotEmpty ? hwChannels.length : 64;

    final List<DropdownMenuItem<String>> outputItems = [];
    final isMulti = _fileChannels != null && _fileChannels! > 2;
    final isMono = _fileChannels == 1;

    for (int i = 0; i < maxChannels; i++) {
      String hwName = i < hwChannels.length ? hwChannels[i] : 'Out ${i + 1}';
      
      if (isMono) {
        outputItems.add(
          DropdownMenuItem(
            value: ChannelDropdownValueHelper.getMonoValue(i),
            child: Text(
              'Ch ${i + 1} ($hwName)',
              style: const TextStyle(fontSize: 12, color: Colors.white),
            ),
          ),
        );
      } else if (!isMulti) {
        // Stereo
        String nextName = (i + 1) < hwChannels.length ? hwChannels[i+1] : 'Out ${i + 2}';
        outputItems.add(
          DropdownMenuItem(
            value: ChannelDropdownValueHelper.getStereoValue(i),
            child: Text(
              'Ch ${i + 1}-${i + 2} ($hwName / $nextName)',
              style: const TextStyle(fontSize: 12, color: Colors.white),
            ),
          ),
        );
      } else {
        // Multi
        int endCh = i + _fileChannels! - 1;
        outputItems.add(
          DropdownMenuItem(
            value: ChannelDropdownValueHelper.getMultiValue(i),
            child: Text(
              'Ch ${i + 1}-${endCh + 1} (Multi)',
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
                              if (newSelection.first) {
                                widget.onOutputChanged?.call(4294967295, false);
                              } else {
                                widget.onOutputChanged?.call(0, false);
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
                            // Show TrajectorySettingsModal
                            import_trajectory_modal(context);
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
"""

new_content = content[:start_idx] + new_build + "\n" + content[end_idx:]

with open('lib/features/dashboard/widgets/track_card.dart', 'w') as f:
    f.write(new_content)
