import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/room_card.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/preferences_modal.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:file_picker/file_picker.dart';

class DashboardScreen extends ConsumerStatefulWidget {
  const DashboardScreen({super.key});

  @override
  ConsumerState<DashboardScreen> createState() => _DashboardScreenState();
}

class _DashboardScreenState extends ConsumerState<DashboardScreen> {
  bool _isProcessing = false;
  final ScrollController _scrollController = ScrollController();

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: AppColors.background,
      body: PlatformMenuBar(
        menus: _buildMenus(context),
        child: Stack(
          children: [
            Column(
              children: [
                _buildHeader(context),
                Expanded(child: _buildRoomPanels(context)),
              ],
            ),
            _buildErrorBanner(),
          ],
        ),
      ),
    );
  }

  List<PlatformMenuItem> _buildMenus(BuildContext context) {
    return [
      PlatformMenu(
        label: 'File',
        menus: [
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'Load Preset',
                onSelected: () async {
                  FilePickerResult? result = await FilePicker.pickFiles(
                    type: FileType.custom,
                    allowedExtensions: ['json'],
                  );
                  if (result != null && result.files.single.path != null) {
                    try {
                      final importedConfig = await rust_api.apiGetConfig(
                        path: result.files.single.path!,
                      );
                      await rust_api.apiStopAll();
                      if (context.mounted) {
                        ref.read(engineStateProvider.notifier).reset();
                        ref.read(configProvider.notifier).saveConfig(importedConfig);
                        await rust_api.apiLoadPreset(config: importedConfig);
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(content: Text('프리셋이 성공적으로 로드되었습니다.'), backgroundColor: AppColors.success),
                          );
                        }
                      }
                    } catch (e) {
                      if (context.mounted) {
                        ref.read(globalErrorProvider.notifier).showError('설정 불러오기 실패: $e');
                      }
                    }
                  }
                },
              ),
              PlatformMenuItem(
                label: 'Save Preset',
                onSelected: () async {
                  final config = ref.read(configProvider);
                  if (config == null) return;
                  String? outputFile = await FilePicker.saveFile(
                    dialogTitle: '설정 저장',
                    fileName: 'atmos_config_backup.json',
                    allowedExtensions: ['json'],
                    type: FileType.custom,
                  );
                  if (outputFile != null) {
                    try {
                      await rust_api.apiSaveConfig(
                        path: outputFile,
                        config: config,
                      );
                      if (context.mounted) {
                        ScaffoldMessenger.of(context).showSnackBar(
                          const SnackBar(content: Text('설정이 저장되었습니다.'), backgroundColor: AppColors.success),
                        );
                      }
                    } catch (e) {
                      if (context.mounted) {
                        ref.read(globalErrorProvider.notifier).showError('설정 저장 실패: $e');
                      }
                    }
                  }
                },
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'Export Log',
                onSelected: () async {
                  try {
                    String dest = '';
                    if (Platform.isWindows) {
                      dest = '${Platform.environment['USERPROFILE']}\\Desktop';
                    } else {
                      dest = '${Platform.environment['HOME']}/Desktop';
                    }
                    await rust_api.apiExportLogs(destinationDir: dest);
                    if (context.mounted) {
                      ScaffoldMessenger.of(context).showSnackBar(
                        const SnackBar(content: Text('바탕화면에 로그가 저장되었습니다.'), backgroundColor: AppColors.success),
                      );
                    }
                  } catch (e) {
                    if (context.mounted) {
                      ref.read(globalErrorProvider.notifier).showError('로그 저장 실패: $e');
                    }
                  }
                },
              ),
            ],
          ),
        ],
      ),
      PlatformMenu(
        label: 'View',
        menus: [
          PlatformMenuItem(
            label: 'Toggle Exhibition Mode',
            onSelected: () {
              final config = ref.read(configProvider);
              if (config != null) {
                final updated = AppConfig(
                  oscPort: config.oscPort,
                  deviceName: config.deviceName,
                  bufferSize: config.bufferSize,
                  themeStartOscAddress: config.themeStartOscAddress,
                  systemResetOscAddress: config.systemResetOscAddress,
                  monoConfigs: config.monoConfigs,
                  stereoConfigs: config.stereoConfigs,
                  rooms: config.rooms,
                  isExhibitionMode: !config.isExhibitionMode,
                );
                ref.read(configProvider.notifier).saveConfig(updated);
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: Text(updated.isExhibitionMode ? '전시 모드가 켜졌습니다.' : '전시 모드가 꺼졌습니다.'),
                    backgroundColor: AppColors.primaryNeon,
                  ),
                );
              }
            },
          ),
        ],
      ),
      PlatformMenu(
        label: 'Settings',
        menus: [
          PlatformMenuItem(
            label: 'Preferences',
            onSelected: () {
              if (context.mounted) {
                showDialog(
                  context: context,
                  builder: (context) => const PreferencesModal(),
                );
              }
            },
          ),
        ],
      ),
    ];
  }

  Widget _buildErrorBanner() {
    return Consumer(
      builder: (context, ref, child) {
        final error = ref.watch(globalErrorProvider);
        return AnimatedPositioned(
          duration: const Duration(milliseconds: 300),
          curve: Curves.easeInOut,
          top: error != null ? 0 : -100,
          left: 0,
          right: 0,
          child: Material(
            elevation: 8,
            color: Colors.transparent,
            child: Container(
              color: AppColors.danger.withValues(alpha: 0.95),
              padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
              child: SafeArea(
                bottom: false,
                child: Row(
                  children: [
                    const Icon(
                      Icons.error_outline,
                      color: Colors.white,
                      size: 28,
                    ),
                    const SizedBox(width: 16),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(
                            error ?? '',
                            style: const TextStyle(
                              color: Colors.white,
                              fontSize: 16,
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                          const SizedBox(height: 4),
                          const Text(
                            '해결 방법: 케이블을 확인하고 상단 설정(Preferences)에서 오디오 장치를 다시 선택하세요.',
                            style: TextStyle(
                              color: Colors.white70,
                              fontSize: 14,
                            ),
                          ),
                        ],
                      ),
                    ),
                    IconButton(
                      icon: const Icon(Icons.close, color: Colors.white),
                      onPressed: () =>
                          ref.read(globalErrorProvider.notifier).clearError(),
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }

  Widget _buildHeader(BuildContext context) {
    return Container(
      color: AppColors.headerBackground,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      child: Wrap(
        alignment: WrapAlignment.spaceBetween,
        crossAxisAlignment: WrapCrossAlignment.center,
        spacing: 16,
        runSpacing: 12,
        children: [
          const Text(
            '🎛 Atmos Mixer Pro',
            style: TextStyle(
              color: AppColors.textPrimary,
              fontSize: 18,
              fontWeight: FontWeight.bold,
            ),
          ),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            alignment: WrapAlignment.center,
            children: [
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.primaryBlue,
                ),
                onPressed: () async {
                  if (_isProcessing) return;
                  setState(() {
                    _isProcessing = true;
                  });
                  try {
                    try {
                      await rust_api.apiStopAll();
                    } catch (e) {
                      ref
                          .read(globalErrorProvider.notifier)
                          .showError('정지 실패: $e');
                    }
                    final config = ref.read(configProvider);
                    if (config != null && config.rooms.isNotEmpty) {
                      if (config.isExhibitionMode) {
                        try {
                          await rust_api.apiPlayAllLoopTracks();
                        } catch (e) {
                          ref
                              .read(globalErrorProvider.notifier)
                              .showError('전시 모드 트랙 재생 실패: $e');
                        }
                      } else {
                        final firstRoom = config.rooms.first;
                        await ref
                            .read(engineStateProvider.notifier)
                            .startTheme(firstRoom.id);
                        for (final track in firstRoom.tracks) {
                          if (track.isLoop) {
                            try {
                              await rust_api.apiPlayTrack(
                                roomId: firstRoom.id,
                                trackId: track.id,
                              );
                            } catch (e) {
                              ref
                                  .read(globalErrorProvider.notifier)
                                  .showError('트랙 재생 실패: $e');
                            }
                          }
                        }
                      }
                    }
                  } finally {
                    setState(() {
                      _isProcessing = false;
                    });
                  }
                },
                child: const Text(
                  '테마 시작',
                  style: TextStyle(color: Colors.white),
                ),
              ),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.danger,
                ),
                onPressed: () async {
                  if (_isProcessing) return;
                  setState(() {
                    _isProcessing = true;
                  });
                  try {
                    await rust_api.apiStopAll();
                  } catch (e) {
                    ref
                        .read(globalErrorProvider.notifier)
                        .showError('비상 정지 실패: $e');
                  } finally {
                    setState(() {
                      _isProcessing = false;
                    });
                  }
                },
                child: const Text(
                  '비상 정지',
                  style: TextStyle(color: Colors.white),
                ),
              ),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.darkGrey,
                ),
                onPressed: () async {
                  if (_isProcessing) return;
                  setState(() {
                    _isProcessing = true;
                  });
                  try {
                    try {
                      await rust_api.apiStopAll();
                    } catch (e) {
                      ref
                          .read(globalErrorProvider.notifier)
                          .showError('시스템 리셋 실패: $e');
                    }
                    ref.read(engineStateProvider.notifier).reset();
                  } finally {
                    setState(() {
                      _isProcessing = false;
                    });
                  }
                },
                child: const Text(
                  '시스템 리셋',
                  style: TextStyle(color: Colors.white),
                ),
              ),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.success,
                ),
                onPressed: () {
                  final config = ref.read(configProvider);
                  if (config != null) {
                    final palette = [
                      '#1565C0',
                      '#6A1B9A',
                      '#2E7D32',
                      '#B71C1C',
                      '#E65100',
                    ];
                    final colorHex = palette[config.rooms.length % 5];
                    final newRoom = RoomConfig(
                      id: 'room_${DateTime.now().millisecondsSinceEpoch}',
                      name: '새로운 룸',
                      colorHex: colorHex,
                      volume: 1.0,
                      clearOscAddress: '/room/clear',
                      tracks: [],
                    );
                    final updated = AppConfig(
                      oscPort: config.oscPort,
                      deviceName: config.deviceName,
                      bufferSize: config.bufferSize,
                      themeStartOscAddress: config.themeStartOscAddress,
                      systemResetOscAddress: config.systemResetOscAddress,
                      monoConfigs: config.monoConfigs,
                      stereoConfigs: config.stereoConfigs,
                      rooms: [...config.rooms, newRoom],
                      isExhibitionMode: config.isExhibitionMode,
                    );
                    ref.read(configProvider.notifier).saveConfig(updated);
                  }
                },
                child: const Text(
                  '➕ 룸 추가',
                  style: TextStyle(color: Colors.white),
                ),
              ),
            ],
          ),
          Consumer(
            builder: (context, ref, child) {
              final config = ref.watch(configProvider);
              final engineState = ref.watch(engineStateProvider);

              return Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 8,
                children: [
                  if (engineState.duckingActive)
                    Container(
                      margin: const EdgeInsets.only(right: 4),
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 4,
                      ),
                      decoration: BoxDecoration(
                        color: AppColors.accentOrange.withValues(alpha: 0.2),
                        borderRadius: BorderRadius.circular(4),
                        border: Border.all(color: AppColors.accentOrange),
                      ),
                      child: const Text(
                        '🦆 스마트 더킹 작동중',
                        style: TextStyle(
                          color: AppColors.accentOrange,
                          fontSize: 12,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                  Text(
                    config?.deviceName ?? '기본 오디오 출력',
                    style: const TextStyle(
                      color: Colors.white,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.refresh, color: Colors.white),
                    tooltip: '스캔',
                    onPressed: () {},
                  ),
                ],
              );
            },
          ),
        ],
      ),
    );
  }

  Widget _buildRoomPanels(BuildContext context) {
    final config = ref.watch(configProvider);
    if (config == null) {
      return const Center(child: CircularProgressIndicator());
    }

    final engineState = ref.watch(engineStateProvider);

    return Listener(
      onPointerSignal: (pointerSignal) {
        if (pointerSignal is PointerScrollEvent) {
          GestureBinding.instance.pointerSignalResolver.register(
            pointerSignal,
            (PointerSignalEvent event) {
              if (event is PointerScrollEvent) {
                final offset = _scrollController.offset;
                final maxScroll = _scrollController.position.maxScrollExtent;
                final minScroll = _scrollController.position.minScrollExtent;
                final newOffset = (offset + event.scrollDelta.dy).clamp(
                  minScroll,
                  maxScroll,
                );
                _scrollController.jumpTo(newOffset);
              }
            },
          );
        }
      },
      child: ListView.builder(
        controller: _scrollController,
        scrollDirection: Axis.horizontal,
        physics: const BouncingScrollPhysics(),
        padding: const EdgeInsets.all(16),
        itemCount: config.rooms.length,
        itemBuilder: (context, index) {
          final room = config.rooms[index];
          Color accentColor;
          try {
            accentColor = Color(
              int.parse(room.colorHex.replaceFirst('#', '0xFF')),
            );
          } catch (e) {
            accentColor = AppColors.primaryNeon;
          }

          final isThemeStarted = engineState.themeStarted;
          final isActive = engineState.activeRoomId == room.id;
          final isCleared = engineState.clearedRoomIds.contains(room.id);

          return RoomCard(
            key: ValueKey(room.id),
            room: room,
            isThemeStarted: isThemeStarted,
            isActive: isActive,
            isCleared: isCleared,
            accentColor: accentColor,
            isExhibitionMode: config.isExhibitionMode,
          );
        },
      ),
    );
  }
}
