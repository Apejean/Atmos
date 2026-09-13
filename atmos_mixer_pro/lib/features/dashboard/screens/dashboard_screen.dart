import 'dart:io';
import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/core/utils/channel_routing.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/room_card.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/preferences_modal.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/osc_monitor_dialog.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/master_limiter_meter.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/resampler_status_badge.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/rta_spectrum_overlay.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/multitrack_timeline.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/advanced_physics_panel.dart';
import 'package:atmos_mixer_pro/features/exhibition/screens/speaker_canvas_screen.dart'
    as atmos_exhibition;
import 'package:atmos_mixer_pro/features/dashboard/widgets/safety_alert_border.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart' as exhibition_model;
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:file_picker/file_picker.dart';

/// 인식된 출력 채널을 한 줄로 요약한다. 장치 이름 옆에 붙는다.
///
/// 인터페이스가 광고하는 출력 개수와 CoreAudio가 보고하는 채널 수는 자주
/// 다르다. 예를 들어 Scarlett 6i6은 물리 출력이 6개(아날로그 4 + S/PDIF 2)
/// 지만, 드라이버가 내부 DAW 리턴 채널까지 더해 12채널을 보고한다. 어느 쪽이
/// 맞는지 사용자가 판단할 수 있도록 총 채널 수와 메인 아웃 이름을 함께 보여준다.
String _outputChannelSummary(OutputChannelsState channels) {
  switch (channels.status) {
    case OutputChannelsStatus.loading:
      return '채널 조회 중...';
    case OutputChannelsStatus.noDevice:
      return '출력 장치 없음';
    case OutputChannelsStatus.error:
      return '채널 조회 실패';
    case OutputChannelsStatus.ready:
      break;
  }

  final names = channels.channelNames;
  if (names.isEmpty) return '출력 채널 0개';

  // 실제로 소리를 내보낼 수 있는 물리 출력만 센다. 드라이버가 보고하는 총
  // 채널 수에는 내부 가상 채널(DAW 리턴 등)이 섞여 있어서, 그 숫자를 그대로
  // 보여주면 현장에서 쓸 수 있는 출력 수를 오해하게 된다.
  final physical = physicalOutputChannelIndices(names);
  if (physical.isEmpty) return '물리 출력 없음';

  final main = physical
      .take(2)
      .map((i) => names[i].trim())
      .where((n) => n.isNotEmpty)
      .join(' / ');

  final head = '물리 출력 ${physical.length}채널';
  if (main.isEmpty) return head;

  final firstTwo = physical.take(2).map((i) => 'Ch-${i + 1}').join('/');
  return '$head · 메인 $firstTwo ($main)';
}

class DashboardScreen extends ConsumerStatefulWidget {
  const DashboardScreen({super.key});

  @override
  ConsumerState<DashboardScreen> createState() => _DashboardScreenState();
}

class _DashboardScreenState extends ConsumerState<DashboardScreen> {
  bool _isRecovering = false;
  bool _isWatchdogActive = false;
  bool _isAutoGuardActive = false;
  final ScrollController _scrollController = ScrollController();
  StreamSubscription<String>? _statusSub;

  @override
  void initState() {
    super.initState();
    _statusSub = rust_api.apiCreateStreamStatusStream().listen((status) {
      if (!mounted) return;
      if (status == 'Failover') {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text(
              '⚠️ ASIO 오디오 인터페이스 연결이 끊어져 기본 출력 장치(WASAPI)로 임시 전환되었습니다. 환경설정에서 오디오 장치를 다시 확인해 주세요.',
              style: TextStyle(fontWeight: FontWeight.bold, color: Colors.white),
            ),
            backgroundColor: Colors.redAccent,
            duration: Duration(seconds: 10),
          ),
        );
      } else if (status == 'WatchdogActive') {
        setState(() => _isWatchdogActive = true);
        Future.delayed(const Duration(seconds: 4), () {
          if (mounted) setState(() => _isWatchdogActive = false);
        });
      } else if (status == 'AutoGuardActive') {
        setState(() => _isAutoGuardActive = true);
        Future.delayed(const Duration(seconds: 4), () {
          if (mounted) setState(() => _isAutoGuardActive = false);
        });
      } else if (status == 'HotReloading') {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text(
              '🔄 오디오 스트림 복구 중... (Hot-Reload)',
              style: TextStyle(color: Colors.black),
            ),
            backgroundColor: Colors.orangeAccent,
            duration: Duration(seconds: 2),
          ),
        );
      }
    });
  }

  @override
  void dispose() {
    _statusSub?.cancel();
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final bodyContent = SafetyAlertBorderWidget(
      isWatchdogActive: _isWatchdogActive,
      isAutoGuardActive: _isAutoGuardActive,
      child: Stack(
        children: [
          Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (!Platform.isMacOS) _buildMaterialMenuBar(context),
              _buildHeader(context),
              Expanded(child: _buildRoomPanels(context)),
            ],
          ),
          _buildErrorModal(),
        ],
      ),
    );

    return Scaffold(
      backgroundColor: AppColors.background,
      body: Platform.isMacOS
          ? PlatformMenuBar(menus: _buildMenus(context), child: bodyContent)
          : bodyContent,
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
                label: 'Load Project',
                onSelected: () async {
                  FilePickerResult? result = await FilePicker.pickFiles(
                    type: FileType.custom,
                    allowedExtensions: ['atmos'],
                  );
                  if (result != null && result.files.single.path != null) {
                    if (context.mounted) {
                      showDialog(
                        context: context,
                        barrierDismissible: false,
                        builder: (context) => const AlertDialog(
                          backgroundColor: AppColors.background,
                          content: Column(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              CircularProgressIndicator(
                                color: AppColors.primaryNeon,
                              ),
                              SizedBox(height: 16),
                              Text(
                                'Loading large audio assets...',
                                style: TextStyle(color: Colors.white),
                              ),
                            ],
                          ),
                        ),
                      );
                    }
                    try {
                      final importedConfig = await rust_api.apiGetConfig(
                        path: result.files.single.path!,
                      );
                      await rust_api.apiStopAll();
                      if (context.mounted) {
                        ref.read(engineStateProvider.notifier).reset();
                        ref
                            .read(configProvider.notifier)
                            .saveConfig(importedConfig);
                        ref
                            .read(tuningStateProvider.notifier)
                            .syncFromBackendConfig(importedConfig);

                        try {
                          await rust_api.apiPreloadAllSounds(
                            config: importedConfig,
                          );
                        } catch (e) {
                          // ignore preload error
                        }

                        if (context.mounted) {
                          Navigator.of(context).pop(); // dismiss dialog
                          ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(
                              content: Text('프리셋이 성공적으로 로드되었습니다.'),
                              backgroundColor: AppColors.success,
                            ),
                          );
                        }
                      }
                    } catch (e) {
                      if (context.mounted) {
                        Navigator.of(context).pop(); // dismiss dialog
                        ref
                            .read(globalErrorProvider.notifier)
                            .showError('설정 불러오기 실패: $e');
                      }
                    }
                  }
                },
              ),
              PlatformMenuItem(
                label: 'Save Project',
                onSelected: () async {
                  final config = ref.read(configProvider);
                  if (config == null) return;
                  String? outputFile = await FilePicker.saveFile(
                    dialogTitle: '프로젝트 저장 (Save Project)',
                    fileName: 'project.atmos',
                    allowedExtensions: ['atmos'],
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
                          const SnackBar(
                            content: Text('설정이 저장되었습니다.'),
                            backgroundColor: AppColors.success,
                          ),
                        );
                      }
                    } catch (e) {
                      if (context.mounted) {
                        ref
                            .read(globalErrorProvider.notifier)
                            .showError('설정 저장 실패: $e');
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
                        const SnackBar(
                          content: Text('바탕화면에 로그가 저장되었습니다.'),
                          backgroundColor: AppColors.success,
                        ),
                      );
                    }
                  } catch (e) {
                    if (context.mounted) {
                      ref
                          .read(globalErrorProvider.notifier)
                          .showError('로그 저장 실패: $e');
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
                final updated = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: config.oscWhitelist,

                  oscPort: config.oscPort,
                  deviceName: config.deviceName,
                  bufferSize: config.bufferSize,
                  themeStartOscAddress: config.themeStartOscAddress,
                  systemResetOscAddress: config.systemResetOscAddress,
                  monoConfigs: config.monoConfigs,
                  stereoConfigs: config.stereoConfigs,
                  multiConfigs: config.multiConfigs,
                  rooms: config.rooms,
                  isExhibitionMode: !config.isExhibitionMode,
                  globalTrajectory: config.globalTrajectory,
                  roomZones: config.roomZones,
                  masterHeadroomDb: config.masterHeadroomDb,
                  peakLimiterEnabled: config.peakLimiterEnabled,
                );
                ref.read(configProvider.notifier).saveConfig(updated);
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: Text(
                      updated.isExhibitionMode
                          ? '전시 모드가 켜졌습니다.'
                          : '전시 모드가 꺼졌습니다.',
                    ),
                    backgroundColor: AppColors.primaryNeon,
                  ),
                );
              }
            },
          ),
          PlatformMenuItem(
            label: 'OSC Packet Monitor',
            onSelected: () {
              if (context.mounted) {
                showDialog(
                  context: context,
                  builder: (context) => const OscMonitorDialog(),
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

  Widget _buildMaterialMenuBar(BuildContext context) {
    return Container(
      color: AppColors.headerBackground,
      child: Row(
        children: [
          MenuBar(
            style: MenuStyle(
              backgroundColor: WidgetStatePropertyAll(
                AppColors.headerBackground,
              ),
              elevation: const WidgetStatePropertyAll(0),
              padding: const WidgetStatePropertyAll(
                EdgeInsets.symmetric(horizontal: 8),
              ),
            ),
            children: [
              SubmenuButton(
                menuChildren: [
                  MenuItemButton(
                    onPressed: () async {
                      FilePickerResult? result = await FilePicker.pickFiles(
                        type: FileType.custom,
                        allowedExtensions: ['atmos'],
                      );
                      if (result != null && result.files.single.path != null) {
                        if (context.mounted) {
                          showDialog(
                            context: context,
                            barrierDismissible: false,
                            builder: (context) => const AlertDialog(
                              backgroundColor: AppColors.background,
                              content: Column(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  CircularProgressIndicator(
                                    color: AppColors.primaryNeon,
                                  ),
                                  SizedBox(height: 16),
                                  Text(
                                    'Loading large audio assets...',
                                    style: TextStyle(color: Colors.white),
                                  ),
                                ],
                              ),
                            ),
                          );
                        }
                        try {
                          final importedConfig = await rust_api.apiGetConfig(
                            path: result.files.single.path!,
                          );
                          await rust_api.apiStopAll();
                          if (context.mounted) {
                            ref.read(engineStateProvider.notifier).reset();
                            ref
                                .read(configProvider.notifier)
                                .saveConfig(importedConfig);
                            ref
                                .read(tuningStateProvider.notifier)
                                .syncFromBackendConfig(importedConfig);

                            try {
                              await rust_api.apiPreloadAllSounds(
                                config: importedConfig,
                              );
                            } catch (e) {
                              // ignore preload error
                            }

                            if (context.mounted) {
                              Navigator.of(context).pop(); // dismiss dialog
                              ScaffoldMessenger.of(context).showSnackBar(
                                const SnackBar(
                                  content: Text('프리셋이 성공적으로 로드되었습니다.'),
                                  backgroundColor: AppColors.success,
                                ),
                              );
                            }
                          }
                        } catch (e) {
                          if (context.mounted) {
                            Navigator.of(context).pop(); // dismiss dialog
                            ref
                                .read(globalErrorProvider.notifier)
                                .showError('설정 불러오기 실패: $e');
                          }
                        }
                      }
                    },
                    child: const Text('Load Project'),
                  ),
                  MenuItemButton(
                    onPressed: () async {
                      final config = ref.read(configProvider);
                      if (config == null) return;
                      String? outputFile = await FilePicker.saveFile(
                        dialogTitle: '프로젝트 저장 (Save Project)',
                        fileName: 'project.atmos',
                        allowedExtensions: ['atmos'],
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
                              const SnackBar(
                                content: Text('설정이 저장되었습니다.'),
                                backgroundColor: AppColors.success,
                              ),
                            );
                          }
                        } catch (e) {
                          if (context.mounted) {
                            ref
                                .read(globalErrorProvider.notifier)
                                .showError('설정 저장 실패: $e');
                          }
                        }
                      }
                    },
                    child: const Text('Save Project'),
                  ),
                  const Divider(),
                  MenuItemButton(
                    onPressed: () async {
                      try {
                        String dest = '';
                        if (Platform.isWindows) {
                          dest =
                              '${Platform.environment['USERPROFILE']}\\Desktop';
                        } else {
                          dest = '${Platform.environment['HOME']}/Desktop';
                        }
                        await rust_api.apiExportLogs(destinationDir: dest);
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(
                              content: Text('바탕화면에 로그가 저장되었습니다.'),
                              backgroundColor: AppColors.success,
                            ),
                          );
                        }
                      } catch (e) {
                        if (context.mounted) {
                          ref
                              .read(globalErrorProvider.notifier)
                              .showError('로그 저장 실패: $e');
                        }
                      }
                    },
                    child: const Text('Export Log'),
                  ),
                ],
                child: const Text('File'),
              ),
              SubmenuButton(
                menuChildren: [
                  MenuItemButton(
                    onPressed: () {
                      final config = ref.read(configProvider);
                      if (config != null) {
                        final updated = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: config.oscWhitelist,

                          oscPort: config.oscPort,
                          deviceName: config.deviceName,
                          bufferSize: config.bufferSize,
                          themeStartOscAddress: config.themeStartOscAddress,
                          systemResetOscAddress: config.systemResetOscAddress,
                          monoConfigs: config.monoConfigs,
                          stereoConfigs: config.stereoConfigs,
                          multiConfigs: config.multiConfigs,
                          rooms: config.rooms,
                          isExhibitionMode: !config.isExhibitionMode,
                          globalTrajectory: config.globalTrajectory,
                          roomZones: config.roomZones,
                          masterHeadroomDb: config.masterHeadroomDb,
                          peakLimiterEnabled: config.peakLimiterEnabled,
                        );
                        ref.read(configProvider.notifier).saveConfig(updated);
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(
                            content: Text(
                              updated.isExhibitionMode
                                  ? '전시 모드가 켜졌습니다.'
                                  : '전시 모드가 꺼졌습니다.',
                            ),
                            backgroundColor: AppColors.primaryNeon,
                          ),
                        );
                      }
                    },
                    child: const Text('Toggle Exhibition Mode'),
                  ),
                  MenuItemButton(
                    onPressed: () {
                      if (context.mounted) {
                        showDialog(
                          context: context,
                          builder: (context) => const OscMonitorDialog(),
                        );
                      }
                    },
                    child: const Text('OSC Packet Monitor'),
                  ),
                ],
                child: const Text('View'),
              ),
              SubmenuButton(
                menuChildren: [
                  MenuItemButton(
                    onPressed: () {
                      if (context.mounted) {
                        showDialog(
                          context: context,
                          builder: (context) => const PreferencesModal(),
                        );
                      }
                    },
                    child: const Text('Preferences'),
                  ),
                ],
                child: const Text('Settings'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildErrorModal() {
    return Consumer(
      builder: (context, ref, child) {
        final error = ref.watch(globalErrorProvider);
        if (error == null) return const SizedBox.shrink();

        return Positioned.fill(
          child: Container(
            color: Colors.black87,
            child: Center(
              child: Container(
                constraints: const BoxConstraints(maxWidth: 500),
                padding: const EdgeInsets.all(32),
                decoration: BoxDecoration(
                  color: AppColors.background,
                  borderRadius: BorderRadius.circular(4),
                  border: Border.all(
                    color: AppColors.danger.withValues(alpha: 0.5),
                    width: 1.0,
                  ),
                  boxShadow: [
                    BoxShadow(
                      color: AppColors.danger.withValues(alpha: 0.15),
                      blurRadius: 12,
                      spreadRadius: 2,
                    ),
                  ],
                ),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(
                      Icons.warning_rounded,
                      color: AppColors.danger,
                      size: 64,
                    ),
                    const SizedBox(height: 24),
                    const Text(
                      '치명적 시스템 오류 발생',
                      style: TextStyle(
                        color: AppColors.danger,
                        fontSize: 24,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                    const SizedBox(height: 16),
                    Text(
                      error,
                      textAlign: TextAlign.center,
                      style: const TextStyle(color: Colors.white, fontSize: 16),
                    ),
                    const SizedBox(height: 24),
                    const Text(
                      '오디오 장치 케이블 연결을 확인하고\n아래의 복구 버튼을 눌러 엔진을 재시작하세요.',
                      textAlign: TextAlign.center,
                      style: TextStyle(color: Colors.white70, fontSize: 14),
                    ),
                    const SizedBox(height: 32),
                    SizedBox(
                      width: double.infinity,
                      height: 56,
                      child: ElevatedButton.icon(
                        onPressed: _isRecovering
                            ? null
                            : () async {
                                final config = ref.read(configProvider);
                                if (config != null) {
                                  setState(() {
                                    _isRecovering = true;
                                  });
                                  try {
                                    await rust_api.apiForceRestartEngine(
                                      deviceName: config.deviceName,
                                    );
                                    if (context.mounted) {
                                      ref
                                          .read(globalErrorProvider.notifier)
                                          .clearError();
                                    }
                                  } finally {
                                    if (context.mounted) {
                                      setState(() {
                                        _isRecovering = false;
                                      });
                                    }
                                  }
                                }
                              },
                        icon: _isRecovering
                            ? const SizedBox(
                                width: 20,
                                height: 20,
                                child: CircularProgressIndicator(
                                  strokeWidth: 2,
                                  color: Colors.white,
                                ),
                              )
                            : const Icon(Icons.refresh, size: 24),
                        label: Text(
                          _isRecovering ? '복구 중...' : '엔진 리셋 (원클릭 복구)',
                          style: const TextStyle(
                            fontSize: 18,
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                        style: ElevatedButton.styleFrom(
                          backgroundColor: AppColors.danger,
                          foregroundColor: Colors.white,
                          disabledBackgroundColor: Colors.white24,
                          shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(4),
                          ),
                        ),
                      ),
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
            'Atmos Mixer Pro',
            style: TextStyle(
              color: AppColors.textPrimary,
              fontSize: 18,
              fontWeight: FontWeight.bold,
            ),
          ),
          Consumer(
            builder: (context, ref, child) {
              final isMasterMuted = ref.watch(
                engineStateProvider.select((state) => state.masterMuteActive),
              );
              if (!isMasterMuted) return const SizedBox.shrink();
              return Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 6,
                ),
                decoration: BoxDecoration(
                  color: Colors.orange.shade800,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.warning_amber_rounded,
                      color: Colors.white,
                      size: 20,
                    ),
                    const SizedBox(width: 8),
                    Text(
                      'MASTER MUTE ACTIVE',
                      style: TextStyle(
                        color: Colors.white,
                        fontWeight: FontWeight.bold,
                        letterSpacing: 1.2,
                      ),
                    ),
                  ],
                ),
              );
            },
          ),
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              const ResamplerStatusBadgeWidget(
                fileSampleRate: 44100,
                deviceSampleRate: 48000,
                forceActive: true,
              ),
              const SizedBox(width: 8),
              MasterLimiterMeterWidget(
                initialGainReductionDb: ref.watch(engineStateProvider).shortTermLufs,
                enableSimulationToggle: true,
              ),
            ],
          ),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            alignment: WrapAlignment.center,
            children: [
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.primaryBlue,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(4),
                  ),
                ),
                onPressed: () async {
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
                  } catch (e) {
                    ref
                        .read(globalErrorProvider.notifier)
                        .showError('테마 시작 오류: $e');
                  }
                },
                child: const Text(
                  'Start',
                  style: TextStyle(color: Colors.white),
                ),
              ),
              ElevatedButton.icon(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.danger,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(4),
                  ),
                ),
                icon: const Icon(Icons.volume_off, color: Colors.white),
                label: const Text(
                  'All Mute',
                  style: TextStyle(color: Colors.white),
                ),
                onPressed: () async {
                  ref.read(engineStateProvider.notifier).toggleMasterMute();
                  final isMuted = ref
                      .read(engineStateProvider)
                      .masterMuteActive;
                  try {
                    await rust_api.apiSetMasterMute(muted: isMuted);
                  } catch (e) {
                    if (context.mounted) {
                      ref
                          .read(globalErrorProvider.notifier)
                          .showError('마스터 음소거 실패: $e');
                    }
                  }
                },
              ),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.danger,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(4),
                  ),
                ),
                onPressed: () async {
                  try {
                    await rust_api.apiStopAll();
                  } catch (e) {
                    ref
                        .read(globalErrorProvider.notifier)
                        .showError('비상 정지 실패: $e');
                  }
                },
                child: const Text(
                  'Emergency',
                  style: TextStyle(color: Colors.white),
                ),
              ),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.darkGrey,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(4),
                  ),
                ),
                onPressed: () async {
                  try {
                    try {
                      await rust_api.apiStopAll();
                    } catch (e) {
                      ref
                          .read(globalErrorProvider.notifier)
                          .showError('시스템 리셋 실패: $e');
                    }
                    ref.read(engineStateProvider.notifier).reset();
                  } catch (e) {
                    ref
                        .read(globalErrorProvider.notifier)
                        .showError('시스템 리셋 오류: $e');
                  }
                },
                child: const Text(
                  'Reset',
                  style: TextStyle(color: Colors.white),
                ),
              ),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.success,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(4),
                  ),
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
                      volumeOscAddress: '/room/volume',
                      tracks: [],
                    );
                    final updated = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: config.oscWhitelist,

                      oscPort: config.oscPort,
                      deviceName: config.deviceName,
                      bufferSize: config.bufferSize,
                      themeStartOscAddress: config.themeStartOscAddress,
                      systemResetOscAddress: config.systemResetOscAddress,
                      monoConfigs: config.monoConfigs,
                      stereoConfigs: config.stereoConfigs,
                      multiConfigs: config.multiConfigs,
                      rooms: [...config.rooms, newRoom],
                      isExhibitionMode: config.isExhibitionMode,
                      globalTrajectory: config.globalTrajectory,
                      roomZones: config.roomZones,
                      masterHeadroomDb: config.masterHeadroomDb,
                      peakLimiterEnabled: config.peakLimiterEnabled,
                    );
                    ref.read(configProvider.notifier).saveConfig(updated);

                    // Sync to exhibition canvas room zones
                    int parsedColor;
                    try {
                      parsedColor = int.parse(colorHex.replaceFirst('#', '0xFF'));
                    } catch (_) {
                      parsedColor = 0xFF3B82F6;
                    }
                    final newRoomZone = exhibition_model.RoomZone(
                      id: newRoom.id,
                      label: newRoom.name,
                      x: 200.0 + (config.rooms.length * 50.0), // staggered spawn
                      y: 200.0,
                      width: 5.0,
                      height: 5.0,
                      physicalWidth: 5.0,
                      physicalHeight: 5.0,
                      color: parsedColor,
                    );
                    ref.read(roomZoneProvider.notifier).addRoomZone(newRoomZone);
                  }
                },
                child: const Text(
                  '+ Room',
                  style: TextStyle(color: Colors.white),
                ),
              ),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.background,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(4),
                    side: const BorderSide(color: Colors.white24, width: 1),
                  ),
                ),
                onPressed: () {
                  if (context.mounted) {
                    showDialog(
                      context: context,
                      builder: (context) => const TuningModal(),
                    );
                  }
                },
                child: const Text(
                  'FX',
                  style: TextStyle(color: Colors.white),
                ),
              ),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.background,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(4),
                    side: const BorderSide(color: Colors.white24, width: 1),
                  ),
                ),
                onPressed: () {
                  Navigator.of(context).push(
                    MaterialPageRoute(
                      builder: (context) =>
                          const atmos_exhibition.SpeakerCanvasScreen(),
                    ),
                  );
                },
                child: const Text(
                  'Speaker Layout',
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
                        '스마트 더킹 작동중',
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
                  // 장치 이름만으로는 실제로 몇 채널이 잡혔는지 알 수 없어서,
                  // 인식된 출력 채널 수와 메인 아웃(Ch-1/Ch-2) 이름을 함께 보여준다.
                  // 인터페이스가 물리 출력보다 많은 채널을 보고하는 경우가 흔하다
                  // (예: Scarlett 6i6은 물리 6개 + 내부 DAW 리턴 6개 = 12채널).
                  const SizedBox(width: 8),
                  Consumer(
                    builder: (context, ref, _) {
                      final channels = ref.watch(outputChannelsProvider);
                      return Text(
                        _outputChannelSummary(channels),
                        style: TextStyle(
                          color: channels.status == OutputChannelsStatus.ready
                              ? Colors.white54
                              : Colors.orangeAccent,
                          fontSize: 12,
                        ),
                      );
                    },
                  ),
                  IconButton(
                    icon: const Icon(Icons.refresh, color: Colors.white),
                    tooltip: '스캔',
                    onPressed: () async {
                      try {
                        final deviceInfos = await rust_api.apiGetOutputDevices();
                        GlobalDeviceCache.devices = deviceInfos.map((d) => d.name).toList();
                        for (final info in deviceInfos) {
                          GlobalDeviceCache.channels[info.name] = info.channelNames;
                        }
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(content: Text('오디오 장치 목록을 새로고침했습니다.')),
                          );
                        }
                      } catch (e) {
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(content: Text('스캔 실패: $e')),
                          );
                        }
                      }
                    },
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
