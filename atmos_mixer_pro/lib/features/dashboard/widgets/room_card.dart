import 'dart:async';
import 'dart:ui';
import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:file_picker/file_picker.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'track_card.dart';

Future<bool?> _showDeleteConfirmDialog(
  BuildContext context,
  String title,
  String content,
) {
  return showDialog<bool>(
    context: context,
    barrierColor: Colors.black87,
    builder: (context) => AlertDialog(
      backgroundColor: AppColors.cardSurfaceSolid,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      title: Row(
        children: [
          const Icon(Icons.warning_amber_rounded, color: AppColors.danger),
          const SizedBox(width: 8),
          Text(
            title,
            style: const TextStyle(
              color: AppColors.danger,
              fontWeight: FontWeight.bold,
            ),
          ),
        ],
      ),
      content: Text(content, style: const TextStyle(color: Colors.white)),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text('취소', style: TextStyle(color: Colors.white70)),
        ),
        ElevatedButton(
          style: ElevatedButton.styleFrom(backgroundColor: AppColors.danger),
          onPressed: () => Navigator.of(context).pop(true),
          child: const Text('삭제', style: TextStyle(color: Colors.white)),
        ),
      ],
    ),
  );
}

class HoverGlowButton extends StatefulWidget {
  final VoidCallback? onPressed;
  final Widget icon;
  final Widget label;
  final Color baseColor;
  final Color glowColor;

  const HoverGlowButton({
    super.key,
    required this.onPressed,
    required this.icon,
    required this.label,
    required this.baseColor,
    required this.glowColor,
  });

  @override
  State<HoverGlowButton> createState() => _HoverGlowButtonState();
}

class _HoverGlowButtonState extends State<HoverGlowButton> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final isDisabled = widget.onPressed == null;
    final bgColor = isDisabled
        ? AppColors.darkGrey.withValues(alpha: 0.5)
        : widget.baseColor;

    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      cursor: isDisabled ? SystemMouseCursors.basic : SystemMouseCursors.click,
      child: GestureDetector(
        onTap: widget.onPressed,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 150),
          curve: Curves.easeInOut,
          padding: const EdgeInsets.symmetric(horizontal: 16.0, vertical: 12.0),
          decoration: BoxDecoration(
            color: _isHovered && !isDisabled
                ? bgColor.withValues(alpha: 0.85)
                : bgColor,
            borderRadius: BorderRadius.circular(24),
            border: Border.all(
              color: _isHovered && !isDisabled ? widget.glowColor : Colors.transparent,
              width: 1,
            ),
          ),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [widget.icon, const SizedBox(width: 8), widget.label],
          ),
        ),
      ),
    );
  }
}

class RoomCard extends ConsumerStatefulWidget {
  final RoomConfig room;
  final bool isThemeStarted;
  final bool isActive;
  final bool isCleared;
  final Color accentColor;
  final bool isExhibitionMode;

  const RoomCard({
    super.key,
    required this.room,
    required this.isThemeStarted,
    required this.isActive,
    required this.isCleared,
    required this.accentColor,
    this.isExhibitionMode = false,
  });

  @override
  ConsumerState<RoomCard> createState() => _RoomCardState();
}

class _RoomCardState extends ConsumerState<RoomCard> {
  late TextEditingController _nameController;
  late FocusNode _nameFocusNode;
  bool _isProcessing = false;
  double? _localVolume;
  Timer? _debounce;

  @override
  void initState() {
    super.initState();
    _nameController = TextEditingController(text: widget.room.name);
    _nameFocusNode = FocusNode();
    _nameFocusNode.addListener(() {
      if (!_nameFocusNode.hasFocus &&
          _nameController.text != widget.room.name) {
        _saveName(_nameController.text);
      }
    });
  }

  void _onNameChanged(String value) {
    if (_debounce?.isActive ?? false) _debounce!.cancel();
    _debounce = Timer(const Duration(milliseconds: 500), () {
      if (_nameController.text != widget.room.name) {
        _saveName(_nameController.text);
      }
    });
  }

  void _saveName(String newName) {
    final config = ref.read(configProvider);
    if (config != null) {
      final newRooms = List<RoomConfig>.from(config.rooms);
      final idx = newRooms.indexWhere((r) => r.id == widget.room.id);
      if (idx != -1) {
        newRooms[idx] = RoomConfig(
          id: widget.room.id,
          name: newName,
          colorHex: widget.room.colorHex,
          volume: widget.room.volume,
          clearOscAddress: widget.room.clearOscAddress,
          tracks: widget.room.tracks,
        );
        ref
            .read(configProvider.notifier)
            .saveConfig(
              AppConfig(
                oscPort: config.oscPort,
                deviceName: config.deviceName,
                bufferSize: config.bufferSize,
                themeStartOscAddress: config.themeStartOscAddress,
                systemResetOscAddress: config.systemResetOscAddress,
                monoConfigs: config.monoConfigs,
                stereoConfigs: config.stereoConfigs,
                rooms: newRooms,
                isExhibitionMode: config.isExhibitionMode,
              ),
            );
      }
    }
  }

  @override
  void didUpdateWidget(RoomCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.room.name != widget.room.name &&
        _nameController.text != widget.room.name) {
      _nameController.text = widget.room.name;
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
    final room = widget.room;
    final isThemeStarted = widget.isThemeStarted;
    final isActive = widget.isActive;
    final isCleared = widget.isCleared;
    final accentColor = widget.accentColor;

    final canInteract = !isThemeStarted || isActive;

    return Container(
      width: 350,
      margin: const EdgeInsets.only(right: 16),
      decoration: BoxDecoration(
        color: AppColors.cardSurface,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: isActive ? accentColor : AppColors.darkGrey,
          width: isActive ? 2.0 : 1.0,
        ),
      ),
      child: Stack(
        children: [
          Positioned.fill(
            child: IgnorePointer(
              ignoring: !canInteract,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // Header
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 8,
                    ),
                    decoration: BoxDecoration(
                      color: accentColor.withValues(alpha: 0.1),
                      borderRadius: const BorderRadius.vertical(
                        top: Radius.circular(8),
                      ),
                    ),
                    child: Row(
                      children: [
                        Expanded(
                          child: Row(
                            children: [
                              Expanded(
                                child: TextField(
                                  controller: _nameController,
                                  focusNode: _nameFocusNode,
                                  style: TextStyle(
                                    color: accentColor,
                                    fontWeight: FontWeight.bold,
                                    fontSize: 16,
                                  ),
                                  decoration: const InputDecoration(
                                    isDense: true,
                                    contentPadding: EdgeInsets.zero,
                                    border: InputBorder.none,
                                  ),
                                  onChanged: _onNameChanged,
                                  onSubmitted: _saveName,
                                ),
                              ),
                              Icon(
                                Icons.edit,
                                size: 14,
                                color: accentColor.withValues(alpha: 0.5),
                              ),
                            ],
                          ),
                        ),
                        SizedBox(
                          width: 100,
                          child: SliderTheme(
                            data: SliderThemeData(
                              activeTrackColor: accentColor,
                              thumbColor: accentColor,
                              trackHeight: 2.0,
                              thumbShape: const RoundSliderThumbShape(
                                enabledThumbRadius: 5.0,
                              ),
                            ),
                            child: Slider(
                              value: _localVolume ?? room.volume,
                              min: 0.0,
                              max: 1.0,
                              onChanged: (v) {
                                setState(() => _localVolume = v);
                                rust_api.apiSetMasterVolume(
                                  roomId: room.id,
                                  volume: v,
                                );
                                // We don't save config here to prevent rapid UI freezing & I/O bottleneck
                              },
                              onChangeEnd: (v) {
                                setState(() => _localVolume = null);
                                final currentConfig = ref.read(configProvider);
                                if (currentConfig != null) {
                                  final newRooms = List<RoomConfig>.from(
                                    currentConfig.rooms,
                                  );
                                  final idx = newRooms.indexWhere(
                                    (r) => r.id == room.id,
                                  );
                                  if (idx != -1) {
                                    newRooms[idx] = RoomConfig(
                                      id: room.id,
                                      name: room.name,
                                      colorHex: room.colorHex,
                                      volume: v,
                                      clearOscAddress: room.clearOscAddress,
                                      tracks: room.tracks,
                                    );
                                    ref
                                        .read(configProvider.notifier)
                                        .saveConfig(
                                          AppConfig(
                                            oscPort: currentConfig.oscPort,
                                            deviceName:
                                                currentConfig.deviceName,
                                            bufferSize:
                                                currentConfig.bufferSize,
                                            themeStartOscAddress: currentConfig
                                                .themeStartOscAddress,
                                            systemResetOscAddress: currentConfig
                                                .systemResetOscAddress,
                                            monoConfigs:
                                                currentConfig.monoConfigs,
                                            stereoConfigs:
                                                currentConfig.stereoConfigs,
                                            rooms: newRooms,
                                            isExhibitionMode:
                                                currentConfig.isExhibitionMode,
                                          ),
                                          skipPreload: true,
                                        );
                                  }
                                }
                              },
                            ),
                          ),
                        ),
                        IconButton(
                          icon: const Icon(Icons.close, size: 16),
                          color: AppColors.danger,
                          onPressed: () async {
                            bool? confirm = await _showDeleteConfirmDialog(
                              context,
                              '룸 삭제 경고',
                              '정말 삭제하시겠습니까?\n(현재 룸에 ${room.tracks.length}개의 오디오 트랙이 포함되어 있습니다.)',
                            );
                            if (confirm == true) {
                              final currentConfig = ref.read(configProvider);
                              if (currentConfig != null) {
                                final newRooms = currentConfig.rooms
                                    .where((r) => r.id != room.id)
                                    .toList();
                                ref
                                    .read(configProvider.notifier)
                                    .saveConfig(
                                      AppConfig(
                                        oscPort: currentConfig.oscPort,
                                        deviceName: currentConfig.deviceName,
                                        bufferSize: currentConfig.bufferSize,
                                        themeStartOscAddress:
                                            currentConfig.themeStartOscAddress,
                                        systemResetOscAddress:
                                            currentConfig.systemResetOscAddress,
                                        monoConfigs: currentConfig.monoConfigs,
                                        stereoConfigs:
                                            currentConfig.stereoConfigs,
                                        rooms: newRooms,
                                        isExhibitionMode:
                                            currentConfig.isExhibitionMode,
                                      ),
                                    );
                              }
                            }
                          },
                          padding: EdgeInsets.zero,
                          constraints: const BoxConstraints(),
                        ),
                      ],
                    ),
                  ),

                  // Action Buttons
                  Padding(
                    padding: const EdgeInsets.all(8.0),
                    child: Row(
                      children: [
                        Expanded(
                          child: HoverGlowButton(
                            icon: const Icon(
                              Icons.add,
                              size: 16,
                              color: Colors.white,
                            ),
                            label: const Text(
                              '오디오 파일 추가',
                              style: TextStyle(
                                fontSize: 11,
                                color: Colors.white,
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                            baseColor: AppColors.darkGrey,
                            glowColor: Colors.white,
                            onPressed: () async {
                              FilePickerResult? result =
                                  await FilePicker.pickFiles(
                                    type: FileType.custom,
                                    allowedExtensions: [
                                      'mp3',
                                      'wav',
                                      'aac',
                                      'flac',
                                      'm4a',
                                      'ogg',
                                      'aiff',
                                    ],
                                  );
                              if (result != null &&
                                  result.files.single.path != null) {
                                final currentConfig = ref.read(configProvider);
                                if (currentConfig != null) {
                                  final path = result.files.single.path!;
                                  final name = result.files.single.name;
                                  final newTrack = TrackConfig(
                                    id: 'track_${DateTime.now().millisecondsSinceEpoch}',
                                    name: name,
                                    filePath: path,
                                    volume: 1.0,
                                    isLoop: false,
                                    outputChannel: 0,
                                    outputStereo: true,
                                    playOscAddress: '/play',
                                    stopOscAddress: '/stop',
                                  );
                                  final newRooms = List<RoomConfig>.from(
                                    currentConfig.rooms,
                                  );
                                  final idx = newRooms.indexWhere(
                                    (r) => r.id == room.id,
                                  );
                                  if (idx != -1) {
                                    final newTracks = List<TrackConfig>.from(
                                      newRooms[idx].tracks,
                                    )..add(newTrack);
                                    newRooms[idx] = RoomConfig(
                                      id: room.id,
                                      name: room.name,
                                      colorHex: room.colorHex,
                                      volume: room.volume,
                                      clearOscAddress: room.clearOscAddress,
                                      tracks: newTracks,
                                    );
                                    ref
                                        .read(configProvider.notifier)
                                        .saveConfig(
                                          AppConfig(
                                            oscPort: currentConfig.oscPort,
                                            deviceName:
                                                currentConfig.deviceName,
                                            bufferSize:
                                                currentConfig.bufferSize,
                                            themeStartOscAddress: currentConfig
                                                .themeStartOscAddress,
                                            systemResetOscAddress: currentConfig
                                                .systemResetOscAddress,
                                            monoConfigs:
                                                currentConfig.monoConfigs,
                                            stereoConfigs:
                                                currentConfig.stereoConfigs,
                                            rooms: newRooms,
                                            isExhibitionMode:
                                                currentConfig.isExhibitionMode,
                                          ),
                                        );
                                  }
                                }
                              }
                            },
                          ),
                        ),
                        if (!widget.isExhibitionMode) ...[
                          const SizedBox(width: 8),
                          Expanded(
                            child: HoverGlowButton(
                              icon: const Icon(
                                Icons.check_circle_outline,
                                size: 16,
                                color: Colors.white,
                              ),
                              label: const Text(
                                '룸 클리어',
                                style: TextStyle(
                                  fontSize: 11,
                                  color: Colors.white,
                                  fontWeight: FontWeight.bold,
                                ),
                              ),
                              baseColor: AppColors.brown,
                              glowColor: AppColors.primaryNeon,
                              onPressed: isActive
                                  ? () async {
                                      if (_isProcessing) return;
                                      setState(() {
                                        _isProcessing = true;
                                      });
                                      try {
                                        // TODO: Call rust_api.apiClearRoom when Backend is ready,
                                        // for now we update frontend state directly to simulate
                                        try {
                                          await rust_api.apiClearRoom(
                                            roomId: room.id,
                                          );
                                        } catch (e) {
                                          ref
                                              .read(
                                                globalErrorProvider.notifier,
                                              )
                                              .showError('룸 클리어 중단: $e');
                                          return; // Error means it's already cleared or not active. Do not auto-promote.
                                        }

                                        ref
                                            .read(engineStateProvider.notifier)
                                            .clearRoom(room.id);

                                        // Automatically activate next room if possible
                                        final currentConfig = ref.read(
                                          configProvider,
                                        );
                                        if (currentConfig != null) {
                                          final idx = currentConfig.rooms
                                              .indexWhere(
                                                (r) => r.id == room.id,
                                              );
                                          if (idx != -1 &&
                                              idx + 1 <
                                                  currentConfig.rooms.length) {
                                            final nextRoom =
                                                currentConfig.rooms[idx + 1];
                                            ref
                                                .read(
                                                  engineStateProvider.notifier,
                                                )
                                                .setActiveRoom(nextRoom.id);
                                            // Auto play loop tracks of next room
                                            for (final track
                                                in nextRoom.tracks) {
                                              if (track.isLoop) {
                                                try {
                                                  await rust_api.apiPlayTrack(
                                                    roomId: nextRoom.id,
                                                    trackId: track.id,
                                                  );
                                                } catch (e) {
                                                  ref
                                                      .read(
                                                        globalErrorProvider
                                                            .notifier,
                                                      )
                                                      .showError(
                                                        '트랙 재생 실패: $e',
                                                      );
                                                }
                                              }
                                            }
                                          } else {
                                            // It's the last room
                                            ref
                                                .read(
                                                  engineStateProvider.notifier,
                                                )
                                                .clearActiveRoom();
                                          }
                                        }
                                      } finally {
                                        if (mounted) {
                                          setState(() {
                                            _isProcessing = false;
                                          });
                                        }
                                      }
                                    }
                                  : null,
                            ),
                          ),
                        ],
                      ],
                    ),
                  ),

                  // Track List
                  Expanded(
                    child: ListView.builder(
                      padding: const EdgeInsets.only(bottom: 8),
                      itemCount: room.tracks.length,
                      itemBuilder: (context, index) {
                        final track = room.tracks[index];
                        return TrackCard(
                          key: ValueKey(track.id),
                          track: track,
                          accentColor: accentColor,
                          onPlay: () async {
                            try {
                              await rust_api.apiPlayTrack(
                                roomId: room.id,
                                trackId: track.id,
                              );
                            } catch (e) {
                              ref
                                  .read(globalErrorProvider.notifier)
                                  .showError('트랙 재생 실패: $e');
                            }
                          },
                          onStop: () async {
                            try {
                              await rust_api.apiStopTrack(
                                roomId: room.id,
                                trackId: track.id,
                              );
                            } catch (e) {
                              ref
                                  .read(globalErrorProvider.notifier)
                                  .showError('트랙 정지 실패: $e');
                            }
                          },
                          onDelete: () async {
                            bool? confirm = await _showDeleteConfirmDialog(
                              context,
                              '트랙 삭제',
                              '[${track.name}] 오디오 트랙을 영구적으로 삭제하시겠습니까?',
                            );
                            if (confirm == true) {
                              final currentConfig = ref.read(configProvider);
                              if (currentConfig != null) {
                                final newRooms = List<RoomConfig>.from(
                                  currentConfig.rooms,
                                );
                                final idx = newRooms.indexWhere(
                                  (r) => r.id == room.id,
                                );
                                if (idx != -1) {
                                  final newTracks = newRooms[idx].tracks
                                      .where((t) => t.id != track.id)
                                      .toList();
                                  newRooms[idx] = RoomConfig(
                                    id: room.id,
                                    name: room.name,
                                    colorHex: room.colorHex,
                                    volume: room.volume,
                                    clearOscAddress: room.clearOscAddress,
                                    tracks: newTracks,
                                  );
                                  ref
                                      .read(configProvider.notifier)
                                      .saveConfig(
                                        AppConfig(
                                          oscPort: currentConfig.oscPort,
                                          deviceName: currentConfig.deviceName,
                                          bufferSize: currentConfig.bufferSize,
                                          themeStartOscAddress: currentConfig
                                              .themeStartOscAddress,
                                          systemResetOscAddress: currentConfig
                                              .systemResetOscAddress,
                                          monoConfigs:
                                              currentConfig.monoConfigs,
                                          stereoConfigs:
                                              currentConfig.stereoConfigs,
                                          rooms: newRooms,
                                          isExhibitionMode:
                                              currentConfig.isExhibitionMode,
                                        ),
                                      );
                                }
                              }
                            }
                          },
                          onVolumeChanged: (v) {
                            rust_api.apiSetTrackVolume(
                              roomId: room.id,
                              trackId: track.id,
                              volume: v,
                            );
                          },
                          onVolumeChangeEnd: (v) {
                            final currentConfig = ref.read(configProvider);
                            if (currentConfig != null) {
                              final newRooms = List<RoomConfig>.from(
                                currentConfig.rooms,
                              );
                              final idx = newRooms.indexWhere(
                                (r) => r.id == room.id,
                              );
                              if (idx != -1) {
                                final newTracks = List<TrackConfig>.from(
                                  newRooms[idx].tracks,
                                );
                                final tIdx = newTracks.indexWhere(
                                  (t) => t.id == track.id,
                                );
                                if (tIdx != -1) {
                                  newTracks[tIdx] = TrackConfig(
                                    id: track.id,
                                    name: track.name,
                                    filePath: track.filePath,
                                    volume: v,
                                    isLoop: track.isLoop,
                                    outputChannel: track.outputChannel,
                                    outputStereo: track.outputStereo,
                                    playOscAddress: track.playOscAddress,
                                    stopOscAddress: track.stopOscAddress,
                                  );
                                  newRooms[idx] = RoomConfig(
                                    id: room.id,
                                    name: room.name,
                                    colorHex: room.colorHex,
                                    volume: room.volume,
                                    clearOscAddress: room.clearOscAddress,
                                    tracks: newTracks,
                                  );
                                  ref
                                      .read(configProvider.notifier)
                                      .saveConfig(
                                        AppConfig(
                                          oscPort: currentConfig.oscPort,
                                          deviceName: currentConfig.deviceName,
                                          bufferSize: currentConfig.bufferSize,
                                          themeStartOscAddress: currentConfig
                                              .themeStartOscAddress,
                                          systemResetOscAddress: currentConfig
                                              .systemResetOscAddress,
                                          monoConfigs:
                                              currentConfig.monoConfigs,
                                          stereoConfigs:
                                              currentConfig.stereoConfigs,
                                          rooms: newRooms,
                                          isExhibitionMode:
                                              currentConfig.isExhibitionMode,
                                        ),
                                        skipPreload: true,
                                      );
                                }
                              }
                            }
                          },
                          onLoopChanged: (v) {
                            final currentConfig = ref.read(configProvider);
                            if (currentConfig != null) {
                              final newRooms = List<RoomConfig>.from(
                                currentConfig.rooms,
                              );
                              final idx = newRooms.indexWhere(
                                (r) => r.id == room.id,
                              );
                              if (idx != -1) {
                                final newTracks = List<TrackConfig>.from(
                                  newRooms[idx].tracks,
                                );
                                final tIdx = newTracks.indexWhere(
                                  (t) => t.id == track.id,
                                );
                                if (tIdx != -1) {
                                  newTracks[tIdx] = TrackConfig(
                                    id: track.id,
                                    name: track.name,
                                    filePath: track.filePath,
                                    volume: track.volume,
                                    isLoop: v,
                                    outputChannel: track.outputChannel,
                                    outputStereo: track.outputStereo,
                                    playOscAddress: track.playOscAddress,
                                    stopOscAddress: track.stopOscAddress,
                                  );
                                  newRooms[idx] = RoomConfig(
                                    id: room.id,
                                    name: room.name,
                                    colorHex: room.colorHex,
                                    volume: room.volume,
                                    clearOscAddress: room.clearOscAddress,
                                    tracks: newTracks,
                                  );
                                  ref
                                      .read(configProvider.notifier)
                                      .saveConfig(
                                        AppConfig(
                                          oscPort: currentConfig.oscPort,
                                          deviceName: currentConfig.deviceName,
                                          bufferSize: currentConfig.bufferSize,
                                          themeStartOscAddress: currentConfig
                                              .themeStartOscAddress,
                                          systemResetOscAddress: currentConfig
                                              .systemResetOscAddress,
                                          monoConfigs:
                                              currentConfig.monoConfigs,
                                          stereoConfigs:
                                              currentConfig.stereoConfigs,
                                          rooms: newRooms,
                                          isExhibitionMode:
                                              currentConfig.isExhibitionMode,
                                        ),
                                      );
                                }
                              }
                            }
                          },
                          onNameChanged: (v) {
                            final currentConfig = ref.read(configProvider);
                            if (currentConfig != null) {
                              final newRooms = List<RoomConfig>.from(
                                currentConfig.rooms,
                              );
                              final idx = newRooms.indexWhere(
                                (r) => r.id == room.id,
                              );
                              if (idx != -1) {
                                final newTracks = List<TrackConfig>.from(
                                  newRooms[idx].tracks,
                                );
                                final tIdx = newTracks.indexWhere(
                                  (t) => t.id == track.id,
                                );
                                if (tIdx != -1) {
                                  newTracks[tIdx] = TrackConfig(
                                    id: track.id,
                                    name: v,
                                    filePath: track.filePath,
                                    volume: track.volume,
                                    isLoop: track.isLoop,
                                    outputChannel: track.outputChannel,
                                    outputStereo: track.outputStereo,
                                    playOscAddress: track.playOscAddress,
                                    stopOscAddress: track.stopOscAddress,
                                  );
                                  newRooms[idx] = RoomConfig(
                                    id: room.id,
                                    name: room.name,
                                    colorHex: room.colorHex,
                                    volume: room.volume,
                                    clearOscAddress: room.clearOscAddress,
                                    tracks: newTracks,
                                  );
                                  ref
                                      .read(configProvider.notifier)
                                      .saveConfig(
                                        AppConfig(
                                          oscPort: currentConfig.oscPort,
                                          deviceName: currentConfig.deviceName,
                                          bufferSize: currentConfig.bufferSize,
                                          themeStartOscAddress: currentConfig
                                              .themeStartOscAddress,
                                          systemResetOscAddress: currentConfig
                                              .systemResetOscAddress,
                                          monoConfigs:
                                              currentConfig.monoConfigs,
                                          stereoConfigs:
                                              currentConfig.stereoConfigs,
                                          rooms: newRooms,
                                          isExhibitionMode:
                                              currentConfig.isExhibitionMode,
                                        ),
                                      );
                                }
                              }
                            }
                          },
                          onOutputChanged: (ch, isStereo) {
                            final currentConfig = ref.read(configProvider);
                            if (currentConfig != null) {
                              final newRooms = List<RoomConfig>.from(
                                currentConfig.rooms,
                              );
                              final idx = newRooms.indexWhere(
                                (r) => r.id == room.id,
                              );
                              if (idx != -1) {
                                final newTracks = List<TrackConfig>.from(
                                  newRooms[idx].tracks,
                                );
                                final tIdx = newTracks.indexWhere(
                                  (t) => t.id == track.id,
                                );
                                if (tIdx != -1) {
                                  newTracks[tIdx] = TrackConfig(
                                    id: track.id,
                                    name: track.name,
                                    filePath: track.filePath,
                                    volume: track.volume,
                                    isLoop: track.isLoop,
                                    outputChannel: ch,
                                    outputStereo: isStereo,
                                    playOscAddress: track.playOscAddress,
                                    stopOscAddress: track.stopOscAddress,
                                  );
                                  newRooms[idx] = RoomConfig(
                                    id: room.id,
                                    name: room.name,
                                    colorHex: room.colorHex,
                                    volume: room.volume,
                                    clearOscAddress: room.clearOscAddress,
                                    tracks: newTracks,
                                  );
                                  rust_api.apiSetTrackOutput(
                                    roomId: room.id,
                                    trackId: track.id,
                                    outputChannel: BigInt.from(ch),
                                    outputStereo: isStereo,
                                  );
                                  ref
                                      .read(configProvider.notifier)
                                      .saveConfig(
                                        AppConfig(
                                          oscPort: currentConfig.oscPort,
                                          deviceName: currentConfig.deviceName,
                                          bufferSize: currentConfig.bufferSize,
                                          themeStartOscAddress: currentConfig
                                              .themeStartOscAddress,
                                          systemResetOscAddress: currentConfig
                                              .systemResetOscAddress,
                                          monoConfigs:
                                              currentConfig.monoConfigs,
                                          stereoConfigs:
                                              currentConfig.stereoConfigs,
                                          rooms: newRooms,
                                          isExhibitionMode:
                                              currentConfig.isExhibitionMode,
                                        ),
                                        skipPreload: true,
                                      );
                                }
                              }
                            }
                          },
                        );
                      },
                    ),
                  ),
                ],
              ),
            ),
          ),

          // Locked / Cleared Badge & Glassmorphism Overlay
          if (isThemeStarted && !isActive && !widget.isExhibitionMode)
            Positioned.fill(
              child: ClipRRect(
                borderRadius: BorderRadius.circular(10),
                child: BackdropFilter(
                  filter: ImageFilter.blur(sigmaX: 20.0, sigmaY: 20.0),
                  child: Container(
                    color: Colors.black.withValues(alpha: 0.4),
                    alignment: Alignment.topCenter,
                    padding: const EdgeInsets.only(top: 40),
                    child: Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 16,
                        vertical: 8,
                      ),
                      decoration: BoxDecoration(
                        color: Colors.black87,
                        borderRadius: BorderRadius.circular(20),
                        border: Border.all(color: Colors.white24),
                      ),
                      child: Text(
                        isCleared ? '✅ 룸 클리어됨' : '🔒 잠금 — 이전 룸을 클리어하세요',
                        style: TextStyle(
                          color: isCleared
                              ? AppColors.primaryNeon
                              : Colors.white,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}
