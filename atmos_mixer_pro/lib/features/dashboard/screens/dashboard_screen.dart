import 'dart:io';
import 'package:atmos_mixer_pro/core/state/engine_resync.dart';
import 'package:atmos_mixer_pro/core/state/project_file.dart';
import 'package:atmos_mixer_pro/core/state/project_media.dart';
import 'package:atmos_mixer_pro/core/state/project_export.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/export_project_dialog.dart';
import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/core/utils/channel_routing.dart';
import 'package:atmos_mixer_pro/core/utils/log_export_dir.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/room_card.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/preferences_modal.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/osc_monitor_dialog.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/missing_media_dialog.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/master_limiter_meter.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/resampler_status_badge.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/binaural_toggle_badge.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/acoustic_sync_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/spatial_sync_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/screens/speaker_canvas_screen.dart'
    as atmos_exhibition;
import 'package:atmos_mixer_pro/features/dashboard/widgets/safety_alert_border.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/features/dashboard/widgets/track_play_failure.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/global_error_overlay.dart';
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
              // ASIO/WASAPI는 Windows 전용 개념이라 macOS에서는 맞지 않는
              // 문구였다(실기 보고: "나는 맥인데 왜 저런 게 뜨냐").
              // 폴백 동작 자체는 플랫폼 공통이므로 문구도 중립으로 쓴다.
              '⚠️ 오디오 장치 연결이 끊어져 시스템 기본 출력 장치로 임시 전환되었습니다. 환경설정에서 오디오 장치를 다시 확인해 주세요.',
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
    // 스피커 위치/룸/환경이 바뀌면 채널별 FX(딜레이, 게인, EQ, 위상)를 자동
    // 재계산하는 동기화 provider를 여기서 활성화한다.
    //
    // Riverpod NotifierProvider는 지연 생성이라 아무도 읽지 않으면 build()가
    // 실행되지 않고, 따라서 내부의 ref.listen(speakerLayoutProvider, ...)도
    // 등록되지 않는다. 예전에는 이 provider를 앱 어디에서도 읽지 않아
    // (테스트에서만 읽었다) 기능 전체가 죽어 있었다 — 스피커를 옮겨도 FX가
    // 전혀 따라오지 않던 원인이다. 메인 화면은 앱 수명 내내 살아 있으므로
    // 여기서 watch해 동기화가 항상 돌게 한다.
    ref.watch(acousticSyncProvider);
    // 공간 설정(스피커 좌표·방·궤적·청취 지점)을 엔진에 보내는 단일 소유자.
    // 같은 이유로 여기서 watch해야 한다 — 아무도 읽지 않으면 전송이 멈춘다.
    ref.watch(spatialSyncProvider);

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
          const GlobalErrorOverlay(),
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

  /// 프로젝트 파일(.atmos)을 연다. macOS 메뉴와 Windows 메뉴가 같이 쓴다.
  /// 다른 PC에서 저장한 프로젝트면 트랙 오디오·도면 경로를 이 PC의 파일로 다시 연결한다
  /// (core/state/project_media.dart). 프로젝트 폴더에서 못 찾은 파일이 있으면 폴더를 묻는다.
  Future<void> _loadProject(BuildContext context) async {
    final FilePickerResult? result = await FilePicker.pickFiles(
      type: FileType.custom,
      allowedExtensions: ['atmos'],
    );
    final path = result?.files.single.path;
    if (path == null || !context.mounted) return;
    _showLoading(context);
    try {
      final imported = await rust_api.apiGetConfig(path: path);
      final rawBlueprint = readExhibitionSection(await File(path).readAsString())?['blueprint_image_path'];
      final media = await relinkProjectMedia(
        config: imported,
        blueprintPath: rawBlueprint is String && rawBlueprint.isNotEmpty ? rawBlueprint : null,
        projectDir: File(path).parent.path,
        askFolder: (missing) async {
          // 로딩 창을 닫고 묻는다. 답을 받으면 다시 띄운다.
          if (!context.mounted) return null;
          Navigator.of(context).pop();
          final folder = await askMediaFolder(context, missing);
          if (context.mounted) _showLoading(context);
          return folder;
        },
      );
      // 프로젝트의 오디오 장치가 이 PC에 없으면(다른 OS·다른 PC에서 저장) 지금 장치를 그대로 쓴다
      // (project_media.dart deviceForImportedProject 참고).
      List<String>? available = GlobalDeviceCache.devices;
      if (available == null) {
        try {
          available = (await rust_api.apiGetOutputDevices()).map((d) => d.name).toList();
        } catch (_) {
          available = null; // 목록을 모르면 프로젝트 값을 그대로 쓴다(예전 동작).
        }
      }
      final device = deviceForImportedProject(
        imported: media.config,
        current: ref.read(configProvider),
        available: available,
      );
      final importedConfig = device.config;
      await rust_api.apiStopAll();
      if (context.mounted) {
        ref.read(engineStateProvider.notifier).reset();
        ref.read(configProvider.notifier).saveConfig(importedConfig);
        ref
            .read(tuningStateProvider.notifier)
            .syncFromBackendConfig(importedConfig, treatAsManual: true);

        // 엔진 설정에 없는 설계 데이터(스피커 배치·방·리버브·베이스
        // 매니지먼트·궤적·청사진·채널 튜닝)를 같은 파일에서 복원한다.
        // 설계 데이터가 없는 예전 파일이면 건너뛴다.
        if (await restoreExhibitionDataFromFile(path)) {
          // 도면 경로는 복원한 값 대신 이 PC에서 찾은 경로를 쓴다.
          await storeBlueprintPath(media.blueprintPath);
          await reloadExhibitionProvidersFromWidgetRef(ref);
          resyncEngineStateFromWidgetRef(ref);
        }

        try {
          await rust_api.apiPreloadAllSounds(config: importedConfig);
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
          if (media.missing.isNotEmpty) await showMissingMedia(context, media.missing);
          if (device.keptCurrent && context.mounted) {
            final shortfall = channelShortfallNotice(
              projectHighest: highestProjectChannel(importedConfig),
              deviceChannels: ref.read(engineStateProvider).outputChannelCount,
            );
            await showDialog<void>(
              context: context,
              builder: (context) => AlertDialog(
                backgroundColor: AppColors.background,
                title: const Text('오디오 장치', style: TextStyle(color: Colors.white)),
                content: Text(
                  '프로젝트의 오디오 장치 "${device.projectDevice}"는 이 PC에 없어서 '
                  '지금 장치 "${importedConfig.deviceName ?? '시스템 기본 장치'}"를 그대로 씁니다. '
                  '채널은 번호 그대로 이 장치의 같은 채널로 나갑니다.\n'
                  '${shortfall == null ? '' : '$shortfall\n'}'
                  '다른 장치를 쓰려면 환경설정에서 고르세요.',
                  style: const TextStyle(color: Colors.white70),
                ),
                actions: [
                  TextButton(onPressed: () => Navigator.of(context).pop(), child: const Text('확인')),
                ],
              ),
            );
          }
        }
      }
    } catch (e) {
      if (context.mounted) {
        Navigator.of(context).pop(); // dismiss dialog
        ref.read(globalErrorProvider.notifier).showOperationError('설정 불러오기 실패: ${errorText(e)}');
      }
    }
  }

  /// File > Export Project: 정한 이름의 폴더에 project.atmos + audio/ + drawing/을 모은다
  /// (core/state/project_export.dart, 사용자 요청 2026-10-10).
  Future<void> _exportProject(BuildContext context) async {
    final config = ref.read(configProvider);
    if (config == null) return;
    final parent = await FilePicker.getDirectoryPath(dialogTitle: '내보낼 위치(이 안에 새 폴더를 만듭니다)');
    if (parent == null || !context.mounted) return;
    final now = DateTime.now();
    String two(int v) => v.toString().padLeft(2, '0');
    final name = await askExportFolderName(
      context,
      parentDir: parent,
      initial: 'Atmos_Project_${now.year}${two(now.month)}${two(now.day)}',
    );
    if (name == null || !context.mounted) return;
    final target = Directory('$parent${Platform.pathSeparator}$name');
    if (await target.exists() && !await target.list().isEmpty) {
      if (!context.mounted || !await confirmExportIntoExisting(context, target.path)) return;
    }
    if (!context.mounted) return;
    final status = ValueNotifier<String>('준비 중...');
    showExportProgress(context, status);
    try {
      // 드래그·노브 조작은 저장이 300ms 미뤄져 있다. 먼저 끝낸다(Save Project와 같다).
      await flushExhibitionSavesFromWidgetRef(ref);
      final result = await exportProject(
        parentDir: parent,
        folderName: name,
        config: config,
        design: await collectExhibitionData(),
        saveConfig: (path, cfg) => rust_api.apiSaveConfig(path: path, config: cfg),
        onProgress: (done, total, file) => status.value =
            file.isEmpty ? '프로젝트 파일을 쓰는 중...' : '파일 복사 ${done + 1}/$total: $file',
      );
      if (context.mounted) Navigator.of(context).pop(); // 진행 창
      if (context.mounted) await showExportResult(context, result);
    } catch (e) {
      if (context.mounted) {
        Navigator.of(context).pop(); // 진행 창
        ref.read(globalErrorProvider.notifier).showOperationError('프로젝트 내보내기 실패: ${errorText(e)}');
      }
    } finally {
      status.dispose();
    }
  }

  void _showLoading(BuildContext context) {
    showDialog(
      context: context,
      barrierDismissible: false,
      builder: (context) => const AlertDialog(
        backgroundColor: AppColors.background,
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            CircularProgressIndicator(color: AppColors.primaryNeon),
            SizedBox(height: 16),
            Text('Loading large audio assets...', style: TextStyle(color: Colors.white)),
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
                label: 'Load Project',
                onSelected: () => _loadProject(context),
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
                      // 엔진이 파일을 쓴 뒤에 설계 데이터를 같은 파일에 얹는다.
                      // 이게 없으면 새 컴퓨터에서 스피커 배치와 방 설계가 사라진다.
                      // 드래그·노브 조작은 저장이 300ms 미뤄져 있다. 먼저 끝낸다.
                      await flushExhibitionSavesFromWidgetRef(ref);
                      await appendExhibitionDataToFile(outputFile);
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
                            .showOperationError('설정 저장 실패: ${errorText(e)}');
                      }
                    }
                  }
                },
              ),
              PlatformMenuItem(
                label: 'Export Project',
                onSelected: () => _exportProject(context),
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'Export Log',
                onSelected: () async {
                  try {
                    // 바탕화면을 못 찾으면(OneDrive로 옮겨진 경우 등) 폴더를 묻는다.
                    final dest = await desktopDirForLogExport() ??
                        await FilePicker.getDirectoryPath(dialogTitle: '로그를 저장할 폴더');
                    if (dest == null) return;
                    await rust_api.apiExportLogs(destinationDir: dest);
                    if (context.mounted) {
                      ScaffoldMessenger.of(context).showSnackBar(
                        SnackBar(
                          content: Text('로그를 저장했습니다: $dest'),
                          backgroundColor: AppColors.success,
                        ),
                      );
                    }
                  } catch (e) {
                    if (context.mounted) {
                      ref
                          .read(globalErrorProvider.notifier)
                          .showOperationError('로그 저장 실패: ${errorText(e)}');
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
                    onPressed: () => _loadProject(context),
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
                          // 엔진이 파일을 쓴 뒤 설계 데이터를 같은 파일에 얹는다
                          // (macOS 메뉴 쪽과 같은 처리).
                          // 드래그·노브 조작은 저장이 300ms 미뤄져 있다. 먼저 끝낸다.
                          await flushExhibitionSavesFromWidgetRef(ref);
                          await appendExhibitionDataToFile(outputFile);
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
                                .showOperationError('설정 저장 실패: ${errorText(e)}');
                          }
                        }
                      }
                    },
                    child: const Text('Save Project'),
                  ),
                  MenuItemButton(
                    onPressed: () => _exportProject(context),
                    child: const Text('Export Project'),
                  ),
                  const Divider(),
                  MenuItemButton(
                    onPressed: () async {
                      try {
                        // 바탕화면을 못 찾으면(OneDrive로 옮겨진 경우 등) 폴더를 묻는다.
                        final dest = await desktopDirForLogExport() ??
                            await FilePicker.getDirectoryPath(dialogTitle: '로그를 저장할 폴더');
                        if (dest == null) return;
                        await rust_api.apiExportLogs(destinationDir: dest);
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(
                              content: Text('로그를 저장했습니다: $dest'),
                              backgroundColor: AppColors.success,
                            ),
                          );
                        }
                      } catch (e) {
                        if (context.mounted) {
                          ref
                              .read(globalErrorProvider.notifier)
                              .showOperationError('로그 저장 실패: ${errorText(e)}');
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
                // 실측 게인 리덕션(양수 dB)을 위젯 규약(음수)으로 맞춰 넘긴다.
                initialGainReductionDb:
                    -ref.watch(engineStateProvider).gainReductionDb.abs(),
                // 상용 전시 현장에서 시뮬레이션이 켜진 채 남으면 미터를
                // 오독하게 되므로 기본적으로 끈다.
                enableSimulationToggle: false,
              ),
              const SizedBox(width: 8),
              const BinauralToggleBadge(),
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
                          .showError('정지 실패: ${errorText(e)}');
                    }
                    final config = ref.read(configProvider);
                    if (config != null && config.rooms.isNotEmpty) {
                      if (config.isExhibitionMode) {
                        try {
                          await rust_api.apiPlayAllLoopTracks();
                        } catch (e) {
                          if (context.mounted) {
                            showTrackPlayFailure(context, e, prefix: '전시 모드 트랙 재생 실패');
                          }
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
                              if (context.mounted) {
                                showTrackPlayFailure(context, e);
                              }
                            }
                          }
                        }
                      }
                    }
                  } catch (e) {
                    ref
                        .read(globalErrorProvider.notifier)
                        .showError('테마 시작 오류: ${errorText(e)}');
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
                          .showError('마스터 음소거 실패: ${errorText(e)}');
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
                        .showError('비상 정지 실패: ${errorText(e)}');
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
                          .showError('시스템 리셋 실패: ${errorText(e)}');
                    }
                    ref.read(engineStateProvider.notifier).reset();
                  } catch (e) {
                    ref
                        .read(globalErrorProvider.notifier)
                        .showError('시스템 리셋 오류: ${errorText(e)}');
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
