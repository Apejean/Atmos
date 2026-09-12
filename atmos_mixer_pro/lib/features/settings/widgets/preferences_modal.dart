import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/core/utils/channel_dropdown_helper.dart';
import 'package:atmos_mixer_pro/core/utils/channel_routing.dart';


class PreferencesModal extends ConsumerStatefulWidget {
  const PreferencesModal({super.key});

  @override
  ConsumerState<PreferencesModal> createState() => _PreferencesModalState();
}

class _PreferencesModalState extends ConsumerState<PreferencesModal>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;
  late AppConfig _tempConfig;

  List<String> _devices = [];
  String _selectedDriverType = 'WASAPI';
  bool _isDeviceManuallyChanged = false;
  bool _isScanning = false;
  final Map<String, int> _trackChannels = {};

  String _getDriverType(String? deviceName) {
    if (deviceName == null) return 'WASAPI';
    deviceName = deviceName.trim();
    if (deviceName.startsWith('[ASIO]')) return 'ASIO';
    if (deviceName.startsWith('[WASAPI]')) return 'WASAPI';
    if (deviceName.startsWith('[CoreAudio]')) return 'CoreAudio';

    if (GlobalDeviceCache.devices != null) {
      final cleanTarget = _getCleanDeviceName(deviceName).trim();
      for (final d in GlobalDeviceCache.devices!) {
        if (_getCleanDeviceName(d).trim() == cleanTarget) {
          if (d.startsWith('[ASIO]')) return 'ASIO';
          if (d.startsWith('[WASAPI]')) return 'WASAPI';
          if (d.startsWith('[CoreAudio]')) return 'CoreAudio';
        }
      }
    }
    return 'WASAPI';
  }

  String _getCleanDeviceName(String? deviceName) {
    if (deviceName == null) return '';
    deviceName = deviceName.trim();
    if (deviceName.startsWith('[ASIO]')) {
      return deviceName.replaceFirst(RegExp(r'^\[ASIO\]\s*'), '');
    }
    if (deviceName.startsWith('[WASAPI]')) {
      return deviceName.replaceFirst(RegExp(r'^\[WASAPI\]\s*'), '');
    }
    if (deviceName.startsWith('[CoreAudio]')) {
      return deviceName.replaceFirst(RegExp(r'^\[CoreAudio\]\s*'), '');
    }
    return deviceName;
  }

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
    // clone config for editing
    final currentConfig = ref.read(configProvider);
    _tempConfig = currentConfig != null
        ? cloneConfig(currentConfig)
        : AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: const [],

            oscPort: 8000,
            bufferSize: 256,
            themeStartOscAddress: '/theme/start',
            systemResetOscAddress: '/system/reset',
            monoConfigs: {},
            stereoConfigs: {},
            multiConfigs: {},
            rooms: [],
            isExhibitionMode: false,
            masterHeadroomDb: 0.0,
            peakLimiterEnabled: true,
            globalTrajectory: null,
            roomZones: [],
          );

    if (_tempConfig.deviceName != null && GlobalDeviceCache.devices == null) {
      _devices = [_tempConfig.deviceName!];
    }

    if (GlobalDeviceCache.devices != null) {
      _devices = GlobalDeviceCache.devices!;
      _applyLoadedDevices(_devices);
    } else {
      if (_tempConfig.deviceName != null) {
        _devices = [_tempConfig.deviceName!];
      }
      _loadDevices();
    }
    _selectedDriverType = _getDriverType(_tempConfig.deviceName);
    _loadAllTrackChannels();
  }

  Future<void> _loadAllTrackChannels() async {
    for (final room in _tempConfig.rooms) {
      for (final track in room.tracks) {
        if (track.filePath.isNotEmpty) {
          try {
            final ch = await rust_api.apiGetAudioFileChannels(
              filePath: track.filePath,
            );
            if (mounted) {
              setState(() {
                _trackChannels[track.id] = ch;
              });
            }
          } catch (e) {
            // ignore
          }
        }
      }
    }
  }

  String _getDropdownValueForTrack(TrackConfig track) {
    final fileChannels = _trackChannels[track.id];
    if (fileChannels != null && fileChannels > 2) {
      return ChannelDropdownValueHelper.getMultiValue(track.outputChannel);
    } else {
      return track.outputStereo
          ? ChannelDropdownValueHelper.getStereoValue(track.outputChannel)
          : ChannelDropdownValueHelper.getMonoValue(track.outputChannel);
    }
  }

  Future<void> _loadDevices() async {
    try {
      final deviceInfos = await rust_api.apiGetOutputDevices();
      final devices = deviceInfos.map((d) => d.name).toList();
      GlobalDeviceCache.devices = devices;
      for (final info in deviceInfos) {
        GlobalDeviceCache.channels[info.name] = info.channelNames;
      }
      _applyLoadedDevices(devices);
    } catch (e) {
      if (mounted) {
        ref.read(globalErrorProvider.notifier).showError('장치 스캔 실패: $e');
      }
    }
  }

  Future<void> _rescanDevices() async {
    if (_isScanning) return;

    setState(() {
      _isScanning = true;
    });

    // 1. Stop engine to release COM lock
    await rust_api.apiStopAudioEngine();

    // 2. Clear cache to force deep scan
    GlobalDeviceCache.devices = null;
    GlobalDeviceCache.channels.clear();

    // 3. (Removed) We no longer clear UI lists while scanning to prevent visual blinking and state loss.

    // 4. Give a tiny delay for backend async stop to process, Rust backend handles retries internally now
    await Future.delayed(const Duration(milliseconds: 50));

    // 5. Deep Scan
    try {
      final deviceInfos = await rust_api.apiGetOutputDevices();
      if (!mounted) return;

      final devices = deviceInfos.map((d) => d.name).toList();
      GlobalDeviceCache.devices = devices;
      for (final info in deviceInfos) {
        GlobalDeviceCache.channels[info.name] = info.channelNames;
      }
      _applyLoadedDevices(devices);

      // 재스캔으로 GlobalDeviceCache가 교체됐으므로 채널 목록 provider도
      // 강제로 다시 조회한다. 그러지 않으면 낡은 채널 목록이 남는다.
      await ref.read(outputChannelsProvider.notifier).refresh();
      // 현재 선택된 장치의 미리보기도 새 스캔 결과로 맞춘다.
      await _loadChannelPreview(_tempConfig.deviceName);

      // 6. Restart engine using the init API
      await rust_api.apiInitAudioSystem(deviceName: _tempConfig.deviceName);
    } catch (e) {
      if (mounted) {
        ref.read(globalErrorProvider.notifier).showError('장치 스캔 실패: $e');
      }
    } finally {
      if (mounted) {
        setState(() {
          _isScanning = false;
        });
      }
    }
  }

  void _applyLoadedDevices(List<String> devices) {
    if (!mounted) return;
    setState(() {
      _devices = devices;
      if (_tempConfig.deviceName != null) {
        final exactMatch = _devices
            .where((d) => d == _tempConfig.deviceName)
            .firstOrNull;
        if (exactMatch == null) {
          final spaceMatch = _devices
              .where((d) => d.trim() == _tempConfig.deviceName!.trim())
              .firstOrNull;
          if (spaceMatch != null) {
            _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

              oscPort: _tempConfig.oscPort,
              deviceName: spaceMatch,
              bufferSize: _tempConfig.bufferSize,
              themeStartOscAddress: _tempConfig.themeStartOscAddress,
              systemResetOscAddress: _tempConfig.systemResetOscAddress,
              monoConfigs: _tempConfig.monoConfigs,
              stereoConfigs: _tempConfig.stereoConfigs,
              multiConfigs: _tempConfig.multiConfigs,
              rooms: _tempConfig.rooms,
              isExhibitionMode: _tempConfig.isExhibitionMode,
              masterHeadroomDb: _tempConfig.masterHeadroomDb,
              peakLimiterEnabled: _tempConfig.peakLimiterEnabled,
              globalTrajectory: _tempConfig.globalTrajectory,
              roomZones: _tempConfig.roomZones,
            );
          } else {
            final cleanTarget = _getCleanDeviceName(
              _tempConfig.deviceName,
            ).trim();
            final prefixMatch = _devices
                .where((d) => _getCleanDeviceName(d).trim() == cleanTarget)
                .firstOrNull;
            if (prefixMatch != null &&
                !_tempConfig.deviceName!.startsWith('[')) {
              _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

                oscPort: _tempConfig.oscPort,
                deviceName: prefixMatch,
                bufferSize: _tempConfig.bufferSize,
                themeStartOscAddress: _tempConfig.themeStartOscAddress,
                systemResetOscAddress: _tempConfig.systemResetOscAddress,
                monoConfigs: _tempConfig.monoConfigs,
                stereoConfigs: _tempConfig.stereoConfigs,
                multiConfigs: _tempConfig.multiConfigs,
                rooms: _tempConfig.rooms,
                isExhibitionMode: _tempConfig.isExhibitionMode,
                masterHeadroomDb: _tempConfig.masterHeadroomDb,
                peakLimiterEnabled: _tempConfig.peakLimiterEnabled,
                globalTrajectory: _tempConfig.globalTrajectory,
                roomZones: _tempConfig.roomZones,
              );
            }
          }
        }
      }
      _selectedDriverType = _getDriverType(_tempConfig.deviceName);
    });
  }

  // Very basic deep clone for editing
  AppConfig cloneConfig(AppConfig config) {
    return AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: config.oscWhitelist,

      oscPort: config.oscPort,
      deviceName: config.deviceName,
      bufferSize: config.bufferSize,
      themeStartOscAddress: config.themeStartOscAddress,
      systemResetOscAddress: config.systemResetOscAddress,
      monoConfigs: Map.from(config.monoConfigs),
      stereoConfigs: Map.from(config.stereoConfigs),
      multiConfigs: Map.from(config.multiConfigs),
      isExhibitionMode: config.isExhibitionMode,
      masterHeadroomDb: config.masterHeadroomDb,
      peakLimiterEnabled: config.peakLimiterEnabled,
      globalTrajectory: config.globalTrajectory,
      roomZones: config.roomZones,
      rooms: config.rooms
          .map(
            (r) => RoomConfig(
              id: r.id,
              name: r.name,
              colorHex: r.colorHex,
              volume: r.volume,
              clearOscAddress: r.clearOscAddress,
              volumeOscAddress: r.volumeOscAddress,
              tracks: r.tracks
                  .map(
                    (t) => TrackConfig(
                      id: t.id,
                      name: t.name,
                      filePath: t.filePath,
                      volume: t.volume,
                      isLoop: t.isLoop,
                      isStreaming: t.isStreaming,
                      outputChannel: t.outputChannel,
                      outputStereo: t.outputStereo,
                      playOscAddress: t.playOscAddress,
                      stopOscAddress: t.stopOscAddress,
                    ),
                  )
                  .toList(),
            ),
          )
          .toList(),
    );
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  void _saveAndClose() {
    final newRooms = _tempConfig.rooms.map((room) {
      final newTracks = room.tracks.map((track) {
        return TrackConfig(
          id: track.id,
          name: track.name,
          filePath: track.filePath,
          volume: track.volume,
          isLoop: track.isLoop,
          isStreaming: track.isStreaming,
          outputChannel: track.outputChannel,
          outputStereo: track.outputStereo,
          playOscAddress: track.playOscAddress,
          stopOscAddress: track.stopOscAddress,
        );
      }).toList();
      return RoomConfig(
        id: room.id,
        name: room.name,
        colorHex: room.colorHex,
        volume: room.volume,
        clearOscAddress: room.clearOscAddress,
              volumeOscAddress: room.volumeOscAddress,
        tracks: newTracks,
      );
    }).toList();

    final currentConfig = ref.read(configProvider);
    final finalConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

      oscPort: _tempConfig.oscPort,
      deviceName: _isDeviceManuallyChanged
          ? _tempConfig.deviceName
          : (currentConfig?.deviceName ?? _tempConfig.deviceName),
      bufferSize: _tempConfig.bufferSize,
      themeStartOscAddress: _tempConfig.themeStartOscAddress,
      systemResetOscAddress: _tempConfig.systemResetOscAddress,
      monoConfigs: _tempConfig.monoConfigs,
      stereoConfigs: _tempConfig.stereoConfigs,
      multiConfigs: _tempConfig.multiConfigs,
      rooms: newRooms,
      isExhibitionMode: _tempConfig.isExhibitionMode,
      masterHeadroomDb: _tempConfig.masterHeadroomDb,
      peakLimiterEnabled: _tempConfig.peakLimiterEnabled,
      globalTrajectory: _tempConfig.globalTrajectory,
      roomZones: _tempConfig.roomZones,
    );
    ref.read(configProvider.notifier).saveConfig(finalConfig);
    Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    ref.listen<AppConfig?>(configProvider, (previous, next) {
      if (next != null) {
        setState(() {
          final updatedRooms = _tempConfig.rooms.map((tempRoom) {
            final nextRoom = next.rooms.firstWhere(
              (r) => r.id == tempRoom.id,
              orElse: () => tempRoom,
            );
            final updatedTracks = tempRoom.tracks.map((tempTrack) {
              final nextTrack = nextRoom.tracks.firstWhere(
                (t) => t.id == tempTrack.id,
                orElse: () => tempTrack,
              );
              return TrackConfig(
                id: tempTrack.id,
                name: nextTrack.name, // Sync name
                filePath: tempTrack.filePath,
                volume: tempTrack.volume,
                isLoop: tempTrack.isLoop,
                isStreaming: tempTrack.isStreaming,
                outputChannel: tempTrack.outputChannel,
                outputStereo: tempTrack.outputStereo,
                playOscAddress: tempTrack.playOscAddress,
                stopOscAddress: tempTrack.stopOscAddress,
              );
            }).toList();
            return RoomConfig(
              id: tempRoom.id,
              name: nextRoom.name, // Sync name
              colorHex: tempRoom.colorHex,
              volume: tempRoom.volume,
              clearOscAddress: tempRoom.clearOscAddress,
              volumeOscAddress: tempRoom.volumeOscAddress,
              tracks: updatedTracks,
            );
          }).toList();

          _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

            oscPort: _tempConfig.oscPort,
            deviceName: _tempConfig.deviceName,
            bufferSize: _tempConfig.bufferSize,
            themeStartOscAddress: _tempConfig.themeStartOscAddress,
            systemResetOscAddress: next.systemResetOscAddress,
            monoConfigs: Map.from(next.monoConfigs),
            stereoConfigs: Map.from(next.stereoConfigs),
            multiConfigs: Map.from(next.multiConfigs),
            rooms: updatedRooms,
            isExhibitionMode: _tempConfig.isExhibitionMode,
            masterHeadroomDb: _tempConfig.masterHeadroomDb,
            peakLimiterEnabled: _tempConfig.peakLimiterEnabled,
            globalTrajectory: _tempConfig.globalTrajectory,
            roomZones: _tempConfig.roomZones,
          );
        });
      }
    });

    return Dialog(
      backgroundColor: AppColors.background,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(4)),
      child: SizedBox(
        width: 800,
        height: 700,
        child: Column(
          children: [
            // Header
            Container(
              padding: const EdgeInsets.all(16),
              decoration: const BoxDecoration(
                color: AppColors.headerBackground,
                borderRadius: BorderRadius.vertical(top: Radius.circular(4)),
              ),
              child: Row(
                children: [
                  const Icon(Icons.settings, color: Colors.white),
                  const SizedBox(width: 8),
                  const Text(
                    '환경설정 (Preferences)',
                    style: TextStyle(
                      color: Colors.white,
                      fontSize: 18,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  const Spacer(),
                  IconButton(
                    icon: const Icon(Icons.close, color: Colors.white),
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                ],
              ),
            ),

            // Tab Bar
            TabBar(
              controller: _tabController,
              indicatorColor: AppColors.primaryBlue,
              labelColor: AppColors.primaryBlue,
              unselectedLabelColor: Colors.white70,
              tabs: const [
                Tab(text: '오디오 출력 설정 (Audio Routing)'),
                Tab(text: '룸별 신호 및 라우팅 설정 (OSC/Arduino)'),
              ],
            ),

            // Tab Views
            Expanded(
              child: TabBarView(
                controller: _tabController,
                children: [_buildAudioTab(), _buildOscTab()],
              ),
            ),

            // Footer
            Container(
              padding: const EdgeInsets.all(16),
              decoration: const BoxDecoration(
                border: Border(top: BorderSide(color: AppColors.darkGrey)),
              ),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  TextButton(
                    onPressed: () => Navigator.of(context).pop(),
                    child: const Text(
                      '취소',
                      style: TextStyle(color: Colors.white70),
                    ),
                  ),
                  const SizedBox(width: 16),
                  ElevatedButton(
                    style: ElevatedButton.styleFrom(
                      backgroundColor: AppColors.primaryBlue,
                    ),
                    onPressed: _saveAndClose,
                    child: const Text(
                      '저장 후 닫기',
                      style: TextStyle(color: Colors.white),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  /// 저장하기 전에 고른 장치의 채널 목록 미리보기.
  ///
  /// `outputChannelsProvider`는 **저장된** `configProvider.deviceName`만
  /// 구독한다(단일 진실 원천). 그래서 이 모달에서 장치를 바꿔도 저장 전에는
  /// 채널 수/트랙 매핑이 이전 장치 기준으로 남는다. 사용자가 장치를 고르는
  /// 즉시 채널 수를 확인해야 하므로, 저장 전 상태에서만 쓰는 미리보기 목록을
  /// 따로 들고 있는다. 저장하면 provider가 진실이 되고 이 값은 버려진다.
  String? _previewDeviceName;
  List<String>? _previewChannelNames;
  bool _isLoadingPreview = false;

  /// 장치 선택이 바뀔 때 그 장치의 채널 목록을 미리 조회한다.
  ///
  /// `_rescanDevices()`처럼 여러 `await`를 거친 뒤 호출되는 경로가 있어서
  /// 진입 시점에도 위젯이 이미 dispose되었을 수 있다. 그래서 첫 `setState`도
  /// `mounted`로 감싼다.
  Future<void> _loadChannelPreview(String? deviceName) async {
    if (!mounted) return;
    setState(() {
      _previewDeviceName = deviceName;
      _previewChannelNames = null;
      _isLoadingPreview = true;
    });
    try {
      final names = await rust_api.apiGetDeviceChannelNames(
        deviceName: deviceName,
      );
      if (!mounted) return;
      setState(() {
        // 그 사이 사용자가 또 다른 장치를 골랐으면 이 응답은 버린다.
        // 로딩 표시도 최신 요청일 때만 내린다. 그러지 않으면 먼저 시작한
        // 요청이 늦게 도착해 스피너를 끄고, 아직 목록이 없는 상태에서
        // 이전 장치 기준 채널 수가 잠깐 노출된다.
        if (_previewDeviceName == deviceName) {
          _previewChannelNames = names;
          _isLoadingPreview = false;
        }
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        // 조회 실패 시 미리보기를 포기하고 provider 값으로 돌아간다.
        if (_previewDeviceName == deviceName) {
          _previewDeviceName = null;
          _previewChannelNames = null;
          _isLoadingPreview = false;
        }
      });
    }
  }

  /// 이 탭이 실제로 써야 할 채널 목록. 저장 전 장치 변경 중이면 미리보기,
  /// 그 밖에는 provider가 보고한 실제 채널 목록.
  List<String> _effectiveChannelNames(OutputChannelsState channels) {
    if (_previewDeviceName == _tempConfig.deviceName &&
        _previewChannelNames != null) {
      return _previewChannelNames!;
    }
    return channels.channelNames;
  }

  Widget _buildAudioTab() {
    // 단일 진실 원천: 자체 로드하던 _channelNames를 제거하고
    // outputChannelsProvider를 구독한다.
    final outputChannels = ref.watch(outputChannelsProvider);
    final channelNames = _effectiveChannelNames(outputChannels);
    final List<DropdownMenuItem<String>> channelItems = [];

    final sortedMono =
        _tempConfig.monoConfigs.entries.where((e) => e.value.enabled).toList()
          ..sort((a, b) => a.key.compareTo(b.key));
    for (final entry in sortedMono) {
      final key = entry.key;
      final setting = entry.value;

      final realCh1 = key - 1;
      if (realCh1 < channelNames.length) {
        final name1 = setting.customName.isNotEmpty
            ? '$key (${setting.customName} L)'
            : '$key';
        channelItems.add(
          DropdownMenuItem<String>(
            value: ChannelDropdownValueHelper.getMonoValue(realCh1),
            child: Text('Mono $name1'),
          ),
        );
      }

      final realCh2 = key;
      final displayCh2 = key + 1;
      if (realCh2 < channelNames.length) {
        final name2 = setting.customName.isNotEmpty
            ? '$displayCh2 (${setting.customName} R)'
            : '$displayCh2';
        channelItems.add(
          DropdownMenuItem<String>(
            value: ChannelDropdownValueHelper.getMonoValue(realCh2),
            child: Text('Mono $name2'),
          ),
        );
      }
    }

    final sortedStereo =
        _tempConfig.stereoConfigs.entries.where((e) => e.value.enabled).toList()
          ..sort((a, b) => a.key.compareTo(b.key));
    for (final entry in sortedStereo) {
      final key = entry.key;
      final setting = entry.value;
      final realCh = key - 1;
      final displayCh2 = key + 1;
      if (realCh + 1 < channelNames.length) {
        final displayName = setting.customName.isNotEmpty
            ? '$key/$displayCh2 (${setting.customName})'
            : '$key/$displayCh2';
        channelItems.add(
          DropdownMenuItem<String>(
            value: ChannelDropdownValueHelper.getStereoValue(realCh),
            child: Text('2-Ch (Stereo) $displayName'),
          ),
        );
      }
    }

    final sortedMulti =
        _tempConfig.multiConfigs.entries.where((e) => e.value.enabled).toList()
          ..sort((a, b) => a.key.compareTo(b.key));
    for (final entry in sortedMulti) {
      final key = entry.key;
      final setting = entry.value;
      final realCh = key - 1;
      if (realCh < channelNames.length) {
        final displayName = setting.customName.isNotEmpty
            ? '$key~ (${setting.customName})'
            : '$key~';
        channelItems.add(
          DropdownMenuItem<String>(
            value: ChannelDropdownValueHelper.getMultiValue(realCh),
            child: Text('N-Ch (다채널) $displayName'),
          ),
        );
      }
    }

    final uniqueDriverTypes = Platform.isMacOS
        ? ['CoreAudio']
        : ['WASAPI', 'ASIO'];
    if (!uniqueDriverTypes.contains(_selectedDriverType)) {
      _selectedDriverType = uniqueDriverTypes.first;
    }

    final filteredDevices = _devices
        .where((d) => _getDriverType(d) == _selectedDriverType)
        .toList();

    return ListView(
      padding: const EdgeInsets.all(16.0),
      children: [
        const Text(
          '하드웨어 오디오 인터페이스',
          style: TextStyle(
            color: AppColors.primaryNeon,
            fontWeight: FontWeight.bold,
          ),
        ),
        const SizedBox(height: 16),
        Row(
          children: [
            const SizedBox(
              width: 120,
              child: Text(
                'Driver type',
                style: TextStyle(
                  color: Colors.white70,
                  fontWeight: FontWeight.bold,
                  fontSize: 14,
                ),
              ),
            ),
            Expanded(
              child: DropdownButtonFormField<String>(
                key: ValueKey(_selectedDriverType),
                isExpanded: true,
                initialValue: _selectedDriverType,
                dropdownColor: AppColors.cardSurfaceSolid,
                decoration: const InputDecoration(
                  filled: true,
                  fillColor: AppColors.cardSurface,
                  border: OutlineInputBorder(),
                  contentPadding: EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 12,
                  ),
                ),
                items: uniqueDriverTypes
                    .map((t) => DropdownMenuItem(value: t, child: Text(t)))
                    .toList(),
                onChanged: (val) {
                  if (val != null) {
                    _isDeviceManuallyChanged = true;
                    setState(() {
                      _selectedDriverType = val;
                      final newFilteredDevices = _devices
                          .where((d) => _getDriverType(d) == val)
                          .toList();
                      String? newDeviceName;
                      if (newFilteredDevices.isNotEmpty) {
                        newDeviceName = newFilteredDevices.first;
                      } else {
                        newDeviceName = null;
                      }
                      // 드라이버 타입이 바뀌면 장치도 바뀌므로 채널 미리보기도
                      // 새 장치 기준으로 다시 조회한다.
                      _loadChannelPreview(newDeviceName);

                      _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

                        oscPort: _tempConfig.oscPort,
                        deviceName: newDeviceName,
                        bufferSize: _tempConfig.bufferSize,
                        themeStartOscAddress: _tempConfig.themeStartOscAddress,
                        systemResetOscAddress:
                            _tempConfig.systemResetOscAddress,
                        monoConfigs: _tempConfig.monoConfigs,
                        stereoConfigs: _tempConfig.stereoConfigs,
                        multiConfigs: _tempConfig.multiConfigs,
                        rooms: _tempConfig.rooms,
                        isExhibitionMode: _tempConfig.isExhibitionMode,
                        masterHeadroomDb: _tempConfig.masterHeadroomDb,
                        peakLimiterEnabled: _tempConfig.peakLimiterEnabled,
                        globalTrajectory: _tempConfig.globalTrajectory,
                        roomZones: _tempConfig.roomZones,
                      );
                    });
                  }
                },
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        Row(
          children: [
            const SizedBox(
              width: 120,
              child: Text(
                'Audio Device',
                style: TextStyle(
                  color: Colors.white70,
                  fontWeight: FontWeight.bold,
                  fontSize: 14,
                ),
              ),
            ),
            Expanded(
              child: DropdownButtonFormField<String>(
                key: ValueKey(_tempConfig.deviceName),
                isExpanded: true,
                initialValue: _tempConfig.deviceName,
                dropdownColor: AppColors.cardSurfaceSolid,
                decoration: const InputDecoration(
                  filled: true,
                  fillColor: AppColors.cardSurface,
                  border: OutlineInputBorder(),
                  contentPadding: EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 12,
                  ),
                ),
                items: [
                  if (filteredDevices.isEmpty)
                    const DropdownMenuItem<String>(
                      value: null,
                      child: Text(
                        '장치 없음',
                        style: TextStyle(color: Colors.white54),
                      ),
                    )
                  else
                    const DropdownMenuItem<String>(
                      value: null,
                      child: Text('기본 오디오 출력 (Default)'),
                    ),
                  ...filteredDevices.map(
                    (d) => DropdownMenuItem(
                      value: d,
                      child: Text(_getCleanDeviceName(d)),
                    ),
                  ),
                  if (_tempConfig.deviceName != null &&
                      !_devices.contains(_tempConfig.deviceName))
                    DropdownMenuItem(
                      value: _tempConfig.deviceName,
                      child: Text(
                        '${_getCleanDeviceName(_tempConfig.deviceName!)} (Disconnected)',
                      ),
                    ),
                ],
                onChanged: (val) {
                  _isDeviceManuallyChanged = true;
                  // 저장 전에도 고른 장치의 채널 수가 보이도록 미리 조회한다.
                  _loadChannelPreview(val);
                  setState(() {
                    _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

                      oscPort: _tempConfig.oscPort,
                      deviceName: val,
                      bufferSize: _tempConfig.bufferSize,
                      themeStartOscAddress: _tempConfig.themeStartOscAddress,
                      systemResetOscAddress: _tempConfig.systemResetOscAddress,
                      monoConfigs: _tempConfig.monoConfigs,
                      stereoConfigs: _tempConfig.stereoConfigs,
                      multiConfigs: _tempConfig.multiConfigs,
                      rooms: _tempConfig.rooms,
                      isExhibitionMode: _tempConfig.isExhibitionMode,
                      masterHeadroomDb: _tempConfig.masterHeadroomDb,
                      peakLimiterEnabled: _tempConfig.peakLimiterEnabled,
                      globalTrajectory: _tempConfig.globalTrajectory,
                      roomZones: _tempConfig.roomZones,
                    );
                  });
                },
              ),
            ),
            const SizedBox(width: 8),
            SizedBox(
              height: 48,
              child: ElevatedButton.icon(
                style: ElevatedButton.styleFrom(
                  backgroundColor: Colors.blueAccent.withValues(alpha: 0.2),
                  foregroundColor: Colors.blueAccent,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(4),
                    side: const BorderSide(color: Colors.blueAccent),
                  ),
                ),
                onPressed: _isScanning ? null : _rescanDevices,
                icon: _isScanning
                    ? const SizedBox(
                        width: 18,
                        height: 18,
                        child: CircularProgressIndicator(
                          strokeWidth: 2,
                          color: Colors.blueAccent,
                        ),
                      )
                    : const Icon(Icons.refresh, size: 18),
                label: Text(
                  _isScanning ? 'Scanning...' : 'Rescan',
                  style: const TextStyle(fontWeight: FontWeight.bold),
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        Row(
          children: [
            const SizedBox(
              width: 120,
              child: Text(
                'Buffer Size',
                style: TextStyle(
                  color: Colors.white70,
                  fontWeight: FontWeight.bold,
                  fontSize: 14,
                ),
              ),
            ),
            Expanded(
              child: DropdownButtonFormField<int>(
                isExpanded: true,
                initialValue: _tempConfig.bufferSize,
                dropdownColor: AppColors.cardSurfaceSolid,
                decoration: const InputDecoration(
                  filled: true,
                  fillColor: AppColors.cardSurface,
                  border: OutlineInputBorder(),
                ),
                items: [64, 128, 256, 512, 1024]
                    .map(
                      (e) =>
                          DropdownMenuItem(value: e, child: Text('$e samples')),
                    )
                    .toList(),
                onChanged: (val) {
                  if (val != null) {
                    setState(() {
                      _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

                        oscPort: _tempConfig.oscPort,
                        deviceName: _tempConfig.deviceName,
                        bufferSize: val,
                        themeStartOscAddress: _tempConfig.themeStartOscAddress,
                        systemResetOscAddress:
                            _tempConfig.systemResetOscAddress,
                        monoConfigs: _tempConfig.monoConfigs,
                        stereoConfigs: _tempConfig.stereoConfigs,
                        multiConfigs: _tempConfig.multiConfigs,
                        rooms: _tempConfig.rooms,
                        isExhibitionMode: _tempConfig.isExhibitionMode,
                        masterHeadroomDb: _tempConfig.masterHeadroomDb,
                        peakLimiterEnabled: _tempConfig.peakLimiterEnabled,
                        globalTrajectory: _tempConfig.globalTrajectory,
                        roomZones: _tempConfig.roomZones,
                      );
                    });
                  }
                },
              ),
            ),
          ],
        ),
        const SizedBox(height: 24),
        const Text(
          '글로벌 DSP 설정 (Global DSP Settings)',
          style: TextStyle(
            color: AppColors.primaryNeon,
            fontWeight: FontWeight.bold,
          ),
        ),
        const SizedBox(height: 16),
        Row(
          children: [
            const SizedBox(
              width: 120,
              child: Text(
                'Master Headroom',
                style: TextStyle(
                  color: Colors.white70,
                  fontWeight: FontWeight.bold,
                  fontSize: 14,
                ),
              ),
            ),
            SegmentedButton<double>(
              segments: const [
                ButtonSegment<double>(value: 0.0, label: Text('0 dB')),
                ButtonSegment<double>(value: -3.0, label: Text('-3 dB')),
                ButtonSegment<double>(value: -6.0, label: Text('-6 dB')),
              ],
              selected: {_tempConfig.masterHeadroomDb},
              onSelectionChanged: (Set<double> newSelection) {
                setState(() {
                  _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

                    oscPort: _tempConfig.oscPort,
                    deviceName: _tempConfig.deviceName,
                    bufferSize: _tempConfig.bufferSize,
                    themeStartOscAddress: _tempConfig.themeStartOscAddress,
                    systemResetOscAddress: _tempConfig.systemResetOscAddress,
                    monoConfigs: _tempConfig.monoConfigs,
                    stereoConfigs: _tempConfig.stereoConfigs,
                    multiConfigs: _tempConfig.multiConfigs,
                    rooms: _tempConfig.rooms,
                    isExhibitionMode: _tempConfig.isExhibitionMode,
                    masterHeadroomDb: newSelection.first,
                    peakLimiterEnabled: _tempConfig.peakLimiterEnabled,
                    globalTrajectory: _tempConfig.globalTrajectory,
                    roomZones: _tempConfig.roomZones,
                  );
                });
              },
              style: ButtonStyle(
                backgroundColor: WidgetStateProperty.resolveWith<Color>((
                  Set<WidgetState> states,
                ) {
                  if (states.contains(WidgetState.selected)) {
                    return AppColors.primaryNeon.withValues(alpha: 0.2);
                  }
                  return AppColors.cardSurfaceSolid;
                }),
                foregroundColor: WidgetStateProperty.resolveWith<Color>((
                  Set<WidgetState> states,
                ) {
                  if (states.contains(WidgetState.selected)) {
                    return AppColors.primaryNeon;
                  }
                  return Colors.white70;
                }),
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        Row(
          children: [
            const SizedBox(
              width: 120,
              child: Text(
                'Peak Limiter Guard',
                style: TextStyle(
                  color: Colors.white70,
                  fontWeight: FontWeight.bold,
                  fontSize: 14,
                ),
              ),
            ),
            Switch(
              value: _tempConfig.peakLimiterEnabled,
              activeTrackColor: AppColors.primaryNeon.withValues(alpha: 0.5),
              activeThumbColor: AppColors.primaryNeon,
              onChanged: (val) {
                setState(() {
                  _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

                    oscPort: _tempConfig.oscPort,
                    deviceName: _tempConfig.deviceName,
                    bufferSize: _tempConfig.bufferSize,
                    themeStartOscAddress: _tempConfig.themeStartOscAddress,
                    systemResetOscAddress: _tempConfig.systemResetOscAddress,
                    monoConfigs: _tempConfig.monoConfigs,
                    stereoConfigs: _tempConfig.stereoConfigs,
                    multiConfigs: _tempConfig.multiConfigs,
                    rooms: _tempConfig.rooms,
                    isExhibitionMode: _tempConfig.isExhibitionMode,
                    masterHeadroomDb: _tempConfig.masterHeadroomDb,
                    peakLimiterEnabled: val,
                    globalTrajectory: _tempConfig.globalTrajectory,
                    roomZones: _tempConfig.roomZones,
                  );
                });
              },
            ),
            const SizedBox(width: 8),
            Text(
              _tempConfig.peakLimiterEnabled ? 'On' : 'Off',
              style: TextStyle(
                color: _tempConfig.peakLimiterEnabled
                    ? AppColors.primaryNeon
                    : Colors.white54,
                fontWeight: FontWeight.bold,
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        Row(
          children: [
            const SizedBox(
              width: 120,
              child: Text(
                'Exhibition Mode',
                style: TextStyle(
                  color: Colors.white70,
                  fontWeight: FontWeight.bold,
                  fontSize: 14,
                ),
              ),
            ),
            Switch(
              value: _tempConfig.isExhibitionMode,
              activeTrackColor: AppColors.primaryNeon.withValues(alpha: 0.5),
              activeThumbColor: AppColors.primaryNeon,
              onChanged: (val) {
                setState(() {
                  _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
                    oscWhitelist: _tempConfig.oscWhitelist,
                    oscPort: _tempConfig.oscPort,
                    deviceName: _tempConfig.deviceName,
                    bufferSize: _tempConfig.bufferSize,
                    themeStartOscAddress: _tempConfig.themeStartOscAddress,
                    systemResetOscAddress: _tempConfig.systemResetOscAddress,
                    monoConfigs: _tempConfig.monoConfigs,
                    stereoConfigs: _tempConfig.stereoConfigs,
                    multiConfigs: _tempConfig.multiConfigs,
                    rooms: _tempConfig.rooms,
                    isExhibitionMode: val,
                    masterHeadroomDb: _tempConfig.masterHeadroomDb,
                    peakLimiterEnabled: _tempConfig.peakLimiterEnabled,
                    globalTrajectory: _tempConfig.globalTrajectory,
                    roomZones: _tempConfig.roomZones,
                  );
                });
              },
            ),
            const SizedBox(width: 8),
            Text(
              _tempConfig.isExhibitionMode ? 'On' : 'Off',
              style: TextStyle(
                color: _tempConfig.isExhibitionMode
                    ? AppColors.primaryNeon
                    : Colors.white54,
                fontWeight: FontWeight.bold,
              ),
            ),
          ],
        ),



        const SizedBox(height: 24),
        const Text(
          '출력 채널 구성 (Channel Config)',
          style: TextStyle(
            color: AppColors.primaryNeon,
            fontWeight: FontWeight.bold,
          ),
        ),
        const SizedBox(height: 8),
        Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: AppColors.cardSurface,
            borderRadius: BorderRadius.circular(4),
            border: Border.all(color: Colors.white.withValues(alpha: 0.05)),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // 저장 전 장치 변경 미리보기 조회가 provider 상태보다 우선한다.
              if (_isLoadingPreview ||
                  outputChannels.status == OutputChannelsStatus.loading)
                const Row(
                  children: [
                    SizedBox(
                      width: 14,
                      height: 14,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                    SizedBox(width: 8),
                    Text(
                      '출력 채널 정보를 불러오는 중...',
                      style: TextStyle(color: Colors.white70),
                    ),
                  ],
                )
              else if (_previewChannelNames != null &&
                  _previewDeviceName == _tempConfig.deviceName)
                Text(
                  '선택한 장치의 아웃풋 채널은 총 ${channelNames.length}개 입니다. '
                  '(저장하면 적용됩니다)',
                  style: const TextStyle(color: Colors.lightBlueAccent),
                )
              else if (outputChannels.status == OutputChannelsStatus.noDevice)
                const Text(
                  '연결된 출력 장치가 없습니다. 오디오 인터페이스를 연결하거나 선택하세요.',
                  style: TextStyle(color: Colors.orangeAccent),
                )
              else if (outputChannels.status == OutputChannelsStatus.error)
                Text(
                  '채널 정보 조회 실패: ${outputChannels.errorMessage} (마지막으로 확인된 목록 표시 중)',
                  style: const TextStyle(color: Colors.redAccent),
                )
              else
                Text(
                  '현재 선택된 오디오 인터페이스의 아웃풋 채널은 총 ${channelNames.length}개 입니다.',
                  style: const TextStyle(color: Colors.white70),
                ),
              const SizedBox(height: 12),
              ElevatedButton(
                onPressed: () async {
                  final result =
                      await showDialog<Map<String, Map<int, ChannelSetting>>>(
                        context: context,
                        builder: (context) => OutputConfigDialog(
                          channelCount: channelNames.length,
                          initialMonoConfigs: _tempConfig.monoConfigs,
                          initialStereoConfigs: _tempConfig.stereoConfigs,
                          initialMultiConfigs: _tempConfig.multiConfigs,
                        ),
                      );
                  if (result != null) {
                    setState(() {
                      _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

                        oscPort: _tempConfig.oscPort,
                        deviceName: _tempConfig.deviceName,
                        bufferSize: _tempConfig.bufferSize,
                        themeStartOscAddress: _tempConfig.themeStartOscAddress,
                        systemResetOscAddress:
                            _tempConfig.systemResetOscAddress,
                        monoConfigs: result['mono']!,
                        stereoConfigs: result['stereo']!,
                        multiConfigs: result['multi']!,
                        rooms: _tempConfig.rooms,
                        isExhibitionMode: _tempConfig.isExhibitionMode,
                        masterHeadroomDb: _tempConfig.masterHeadroomDb,
                        peakLimiterEnabled: _tempConfig.peakLimiterEnabled,
                        globalTrajectory: _tempConfig.globalTrajectory,
                        roomZones: _tempConfig.roomZones,
                      );
                    });
                  }
                },
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.primaryBlue,
                ),
                child: const Text(
                  'Output Config',
                  style: TextStyle(color: Colors.white),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 24),
        const Text(
          '트랙별 출력 채널 매핑 (Track Routing)',
          style: TextStyle(
            color: AppColors.primaryNeon,
            fontWeight: FontWeight.bold,
          ),
        ),
        const SizedBox(height: 8),
        if (outputChannels.status != OutputChannelsStatus.ready)
          Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Text(
              outputChannels.status == OutputChannelsStatus.noDevice
                  ? '연결된 출력 장치가 없어 채널 매핑을 표시할 수 없습니다.'
                  : outputChannels.status == OutputChannelsStatus.loading
                  ? '출력 채널 정보를 불러오는 중입니다...'
                  : '채널 정보 조회 실패: ${outputChannels.errorMessage} (마지막으로 확인된 목록 표시 중)',
              style: const TextStyle(color: Colors.orangeAccent, fontSize: 12),
            ),
          ),
        ..._tempConfig.rooms.asMap().entries.map((rEntry) {
          final rIndex = rEntry.key;
          final room = rEntry.value;
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Padding(
                padding: const EdgeInsets.only(top: 16, bottom: 8),
                child: Text(
                  '■ ${room.name}',
                  style: const TextStyle(
                    color: Colors.white70,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ),
              ...room.tracks.asMap().entries.map((entry) {
                final tIndex = entry.key;
                final track = entry.value;

                // 라우팅 항목 생성은 track_card.dart와 동일한 순수 함수를
                // 공유해 두 화면의 라벨/값이 항상 일치하도록 한다.
                final trackDropdownItems = buildChannelRoutingItems(
                  // 상단 요약과 같은 목록을 써야 한다. provider의 저장된
                  // 값을 직접 쓰면 장치를 바꿨을 때 요약은 2채널인데 트랙
                  // 매핑은 옛 장치의 8채널 옵션을 계속 제시해, 존재하지 않는
                  // 채널이 저장될 수 있다.
                  channelNames: channelNames,
                  config: _tempConfig,
                  fileChannels: _trackChannels[track.id],
                ).map((item) {
                  return DropdownMenuItem<String>(
                    value: item.value,
                    child: Text(
                      item.label,
                      style: const TextStyle(fontSize: 12),
                      overflow: TextOverflow.ellipsis,
                    ),
                  );
                }).toList();

                final currentVal = _getDropdownValueForTrack(track);
                final bool valueExists = trackDropdownItems.any(
                  (item) => item.value == currentVal,
                );

                return Padding(
                  padding: const EdgeInsets.only(left: 16, bottom: 8),
                  child: Row(
                    children: [
                      Expanded(
                        child: Text(
                          track.name,
                          style: const TextStyle(color: Colors.white),
                        ),
                      ),
                      const Text(
                        '👉 Output Ch.',
                        style: TextStyle(color: Colors.white54),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        flex: 1,
                        child: DropdownButtonFormField<String>(
                          initialValue: currentVal,
                          dropdownColor: AppColors.cardSurfaceSolid,
                          isExpanded: true,
                          decoration: const InputDecoration(
                            isDense: true,
                            filled: true,
                            fillColor: AppColors.cardSurface,
                            border: OutlineInputBorder(),
                          ),
                          style: const TextStyle(
                            color: Colors.white,
                            fontSize: 12,
                          ),
                          items: [
                            ...trackDropdownItems,
                            if (!valueExists)
                              DropdownMenuItem(
                                value: currentVal,
                                child: Text(
                                  '${track.outputChannel + 1} (Missing)',
                                  style: const TextStyle(
                                    color: Colors.redAccent,
                                  ),
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                          ],
                          onChanged: (val) {
                            if (val != null) {
                              final parsedChannel =
                                  ChannelDropdownValueHelper.getChannel(val) ??
                                  0;
                              final isStereo =
                                  ChannelDropdownValueHelper.isStereo(val) ||
                                  ChannelDropdownValueHelper.isMulti(val);

                              final newRooms = List<RoomConfig>.from(
                                _tempConfig.rooms,
                              );
                              final newTracks = List<TrackConfig>.from(
                                newRooms[rIndex].tracks,
                              );
                              newTracks[tIndex] = TrackConfig(
                                id: track.id,
                                name: track.name,
                                filePath: track.filePath,
                                volume: track.volume,
                                isLoop: track.isLoop,
                                isStreaming: track.isStreaming,
                                outputChannel: parsedChannel,
                                outputStereo: isStereo,
                                playOscAddress: track.playOscAddress,
                                stopOscAddress: track.stopOscAddress,
                              );
                              newRooms[rIndex] = RoomConfig(
                                id: room.id,
                                name: room.name,
                                colorHex: room.colorHex,
                                volume: room.volume,
                                clearOscAddress: room.clearOscAddress,
              volumeOscAddress: room.volumeOscAddress,
                                tracks: newTracks,
                              );
                              setState(() {
                                _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

                                  oscPort: _tempConfig.oscPort,
                                  deviceName: _tempConfig.deviceName,
                                  bufferSize: _tempConfig.bufferSize,
                                  themeStartOscAddress:
                                      _tempConfig.themeStartOscAddress,
                                  systemResetOscAddress:
                                      _tempConfig.systemResetOscAddress,
                                  monoConfigs: _tempConfig.monoConfigs,
                                  stereoConfigs: _tempConfig.stereoConfigs,
                                  multiConfigs: _tempConfig.multiConfigs,
                                  rooms: newRooms,
                                  isExhibitionMode:
                                      _tempConfig.isExhibitionMode,
                                  masterHeadroomDb:
                                      _tempConfig.masterHeadroomDb,
                                  peakLimiterEnabled:
                                      _tempConfig.peakLimiterEnabled,
                                  globalTrajectory:
                                      _tempConfig.globalTrajectory,
                                  roomZones: _tempConfig.roomZones,
                                );
                              });
                            }
                          },
                        ),
                      ),
                    ],
                  ),
                );
              }),
            ],
          );
        }),
      ],
    );
  }

  Widget _buildOscTab() {
    return ListView(
      padding: const EdgeInsets.all(16.0),
      children: [
        const Text(
          '룸 클리어 및 트랙 트리거 주소 매핑',
          style: TextStyle(
            color: AppColors.primaryNeon,
            fontWeight: FontWeight.bold,
          ),
        ),
        const SizedBox(height: 8),
        Container(
          margin: const EdgeInsets.only(bottom: 16),
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: AppColors.cardSurface,
            borderRadius: BorderRadius.circular(4),
            border: Border.all(color: Colors.white.withValues(alpha: 0.05)),
          ),
          child: Row(
            children: [
              const SizedBox(
                width: 80,
                child: Text(
                  '테마 시작',
                  style: TextStyle(
                    color: AppColors.primaryBlue,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ),
              Expanded(
                child: TextFormField(
                  initialValue: _tempConfig.themeStartOscAddress,
                  style: const TextStyle(color: Colors.white),
                  decoration: const InputDecoration(
                    isDense: true,
                    hintText: '예: /theme/start (전체 리셋 및 1번 룸 시작)',
                  ),
                  onChanged: (val) {
                    setState(() {
                      _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

                        oscPort: _tempConfig.oscPort,
                        deviceName: _tempConfig.deviceName,
                        bufferSize: _tempConfig.bufferSize,
                        themeStartOscAddress: val,
                        systemResetOscAddress:
                            _tempConfig.systemResetOscAddress,
                        monoConfigs: _tempConfig.monoConfigs,
                        stereoConfigs: _tempConfig.stereoConfigs,
                        multiConfigs: _tempConfig.multiConfigs,
                        rooms: _tempConfig.rooms,
                        isExhibitionMode: _tempConfig.isExhibitionMode,
                        masterHeadroomDb: _tempConfig.masterHeadroomDb,
                        peakLimiterEnabled: _tempConfig.peakLimiterEnabled,
                        globalTrajectory: _tempConfig.globalTrajectory,
                        roomZones: _tempConfig.roomZones,
                      );
                    });
                  },
                ),
              ),
            ],
          ),
        ),
        Container(
          margin: const EdgeInsets.only(bottom: 16),
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: AppColors.cardSurface,
            borderRadius: BorderRadius.circular(4),
            border: Border.all(color: Colors.white.withValues(alpha: 0.05)),
          ),
          child: Row(
            children: [
              const SizedBox(
                width: 80,
                child: Text(
                  '시스템 리셋',
                  style: TextStyle(
                    color: AppColors.danger,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ),
              Expanded(
                child: TextFormField(
                  initialValue: _tempConfig.systemResetOscAddress,
                  style: const TextStyle(color: Colors.white),
                  decoration: const InputDecoration(
                    isDense: true,
                    hintText: '예: /system/reset (재생 정지 및 초기화)',
                  ),
                  onChanged: (val) {
                    setState(() {
                      _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

                        oscPort: _tempConfig.oscPort,
                        deviceName: _tempConfig.deviceName,
                        bufferSize: _tempConfig.bufferSize,
                        themeStartOscAddress: _tempConfig.themeStartOscAddress,
                        systemResetOscAddress: val,
                        monoConfigs: _tempConfig.monoConfigs,
                        stereoConfigs: _tempConfig.stereoConfigs,
                        multiConfigs: _tempConfig.multiConfigs,
                        rooms: _tempConfig.rooms,
                        isExhibitionMode: _tempConfig.isExhibitionMode,
                        masterHeadroomDb: _tempConfig.masterHeadroomDb,
                        peakLimiterEnabled: _tempConfig.peakLimiterEnabled,
                        globalTrajectory: _tempConfig.globalTrajectory,
                        roomZones: _tempConfig.roomZones,
                      );
                    });
                  },
                ),
              ),
            ],
          ),
        ),
        ..._tempConfig.rooms.asMap().entries.map((rEntry) {
          final rIndex = rEntry.key;
          final room = rEntry.value;
          return Container(
            margin: const EdgeInsets.only(bottom: 16),
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: AppColors.cardSurface,
              borderRadius: BorderRadius.circular(4),
              border: Border.all(color: Colors.white.withValues(alpha: 0.05)),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '■ ${room.name} 센서 매핑',
                  style: const TextStyle(
                    color: Colors.white,
                    fontWeight: FontWeight.bold,
                  ),
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    const SizedBox(
                      width: 80,
                      child: Text(
                        '룸 클리어',
                        style: TextStyle(
                          color: AppColors.brown,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                    Expanded(
                      child: TextFormField(
                        initialValue: room.clearOscAddress,
                        style: const TextStyle(color: Colors.white),
                        decoration: const InputDecoration(
                          isDense: true,
                          hintText: '예: /room1/clear',
                        ),
                        onChanged: (val) {
                          final newRooms = List<RoomConfig>.from(
                            _tempConfig.rooms,
                          );
                          newRooms[rIndex] = RoomConfig(
                            id: room.id,
                            name: room.name,
                            colorHex: room.colorHex,
                            volume: room.volume,
                            clearOscAddress: val,
                            volumeOscAddress: room.volumeOscAddress,
                            tracks: room.tracks,
                          );
                          setState(() {
                            _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

                              oscPort: _tempConfig.oscPort,
                              deviceName: _tempConfig.deviceName,
                              bufferSize: _tempConfig.bufferSize,
                              themeStartOscAddress:
                                  _tempConfig.themeStartOscAddress,
                              systemResetOscAddress:
                                  _tempConfig.systemResetOscAddress,
                              monoConfigs: _tempConfig.monoConfigs,
                              stereoConfigs: _tempConfig.stereoConfigs,
                              multiConfigs: _tempConfig.multiConfigs,
                              rooms: newRooms,
                              isExhibitionMode: _tempConfig.isExhibitionMode,
                              masterHeadroomDb: _tempConfig.masterHeadroomDb,
                              peakLimiterEnabled:
                                  _tempConfig.peakLimiterEnabled,
                              globalTrajectory: _tempConfig.globalTrajectory,
                              roomZones: _tempConfig.roomZones,
                            );
                          });
                        },
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    const SizedBox(
                      width: 80,
                      child: Text(
                        '볼륨 마스터',
                        style: TextStyle(
                          color: AppColors.primaryNeon,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                    Expanded(
                      child: Tooltip(
                        message: '0.0~1.0 float 값을 수신합니다',
                        child: TextFormField(
                          initialValue: room.volumeOscAddress,
                          style: const TextStyle(color: Colors.white),
                          decoration: const InputDecoration(
                            isDense: true,
                            hintText: '예: /room1/volume',
                          ),
                          onChanged: (val) {
                          final newRooms = List<RoomConfig>.from(
                            _tempConfig.rooms,
                          );
                          newRooms[rIndex] = RoomConfig(
                            id: room.id,
                            name: room.name,
                            colorHex: room.colorHex,
                            volume: room.volume,
                            clearOscAddress: room.clearOscAddress,
                            volumeOscAddress: val,
                            tracks: room.tracks,
                          );
                          setState(() {
                            _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
                              oscWhitelist: _tempConfig.oscWhitelist,
                              oscPort: _tempConfig.oscPort,
                              deviceName: _tempConfig.deviceName,
                              bufferSize: _tempConfig.bufferSize,
                              themeStartOscAddress:
                                  _tempConfig.themeStartOscAddress,
                              systemResetOscAddress:
                                  _tempConfig.systemResetOscAddress,
                              monoConfigs: _tempConfig.monoConfigs,
                              stereoConfigs: _tempConfig.stereoConfigs,
                              multiConfigs: _tempConfig.multiConfigs,
                              rooms: newRooms,
                              isExhibitionMode: _tempConfig.isExhibitionMode,
                              masterHeadroomDb: _tempConfig.masterHeadroomDb,
                              peakLimiterEnabled:
                                  _tempConfig.peakLimiterEnabled,
                              globalTrajectory: _tempConfig.globalTrajectory,
                              roomZones: _tempConfig.roomZones,
                            );
                          });
                        },
                      ),
                    ),
                  ),
                ],
              ),
                const Divider(color: AppColors.darkGrey),
                ...room.tracks.asMap().entries.map((entry) {
                  final tIndex = entry.key;
                  final track = entry.value;
                  return Padding(
                    padding: const EdgeInsets.only(bottom: 8),
                    child: Row(
                      children: [
                        SizedBox(
                          width: 80,
                          child: Text(
                            track.name,
                            style: const TextStyle(color: Colors.white70),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: TextFormField(
                            initialValue: track.playOscAddress,
                            style: const TextStyle(color: Colors.white),
                            decoration: const InputDecoration(
                              isDense: true,
                              prefixIcon: Icon(
                                Icons.play_arrow,
                                color: Colors.green,
                                size: 16,
                              ),
                              hintText: 'Play OSC',
                            ),
                            onChanged: (val) {
                              final newRooms = List<RoomConfig>.from(
                                _tempConfig.rooms,
                              );
                              final newTracks = List<TrackConfig>.from(
                                newRooms[rIndex].tracks,
                              );
                              newTracks[tIndex] = TrackConfig(
                                id: track.id,
                                name: track.name,
                                filePath: track.filePath,
                                volume: track.volume,
                                isLoop: track.isLoop,
                                isStreaming: track.isStreaming,
                                outputChannel: track.outputChannel,
                                outputStereo: track.outputStereo,
                                playOscAddress: val,
                                stopOscAddress: track.stopOscAddress,
                              );
                              newRooms[rIndex] = RoomConfig(
                                id: room.id,
                                name: room.name,
                                colorHex: room.colorHex,
                                volume: room.volume,
                                clearOscAddress: room.clearOscAddress,
              volumeOscAddress: room.volumeOscAddress,
                                tracks: newTracks,
                              );
                              setState(() {
                                _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

                                  oscPort: _tempConfig.oscPort,
                                  deviceName: _tempConfig.deviceName,
                                  bufferSize: _tempConfig.bufferSize,
                                  themeStartOscAddress:
                                      _tempConfig.themeStartOscAddress,
                                  systemResetOscAddress:
                                      _tempConfig.systemResetOscAddress,
                                  monoConfigs: _tempConfig.monoConfigs,
                                  stereoConfigs: _tempConfig.stereoConfigs,
                                  multiConfigs: _tempConfig.multiConfigs,
                                  rooms: newRooms,
                                  isExhibitionMode:
                                      _tempConfig.isExhibitionMode,
                                  masterHeadroomDb:
                                      _tempConfig.masterHeadroomDb,
                                  peakLimiterEnabled:
                                      _tempConfig.peakLimiterEnabled,
                                  globalTrajectory:
                                      _tempConfig.globalTrajectory,
                                  roomZones: _tempConfig.roomZones,
                                );
                              });
                            },
                          ),
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: TextFormField(
                            initialValue: track.stopOscAddress,
                            style: const TextStyle(color: Colors.white),
                            decoration: const InputDecoration(
                              isDense: true,
                              prefixIcon: Icon(
                                Icons.stop,
                                color: Colors.red,
                                size: 16,
                              ),
                              hintText: 'Stop OSC',
                            ),
                            onChanged: (val) {
                              final newRooms = List<RoomConfig>.from(
                                _tempConfig.rooms,
                              );
                              final newTracks = List<TrackConfig>.from(
                                newRooms[rIndex].tracks,
                              );
                              newTracks[tIndex] = TrackConfig(
                                id: track.id,
                                name: track.name,
                                filePath: track.filePath,
                                volume: track.volume,
                                isLoop: track.isLoop,
                                isStreaming: track.isStreaming,
                                outputChannel: track.outputChannel,
                                outputStereo: track.outputStereo,
                                playOscAddress: track.playOscAddress,
                                stopOscAddress: val,
                              );
                              newRooms[rIndex] = RoomConfig(
                                id: room.id,
                                name: room.name,
                                colorHex: room.colorHex,
                                volume: room.volume,
                                clearOscAddress: room.clearOscAddress,
              volumeOscAddress: room.volumeOscAddress,
                                tracks: newTracks,
                              );
                              setState(() {
                                _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

                                  oscPort: _tempConfig.oscPort,
                                  deviceName: _tempConfig.deviceName,
                                  bufferSize: _tempConfig.bufferSize,
                                  themeStartOscAddress:
                                      _tempConfig.themeStartOscAddress,
                                  systemResetOscAddress:
                                      _tempConfig.systemResetOscAddress,
                                  monoConfigs: _tempConfig.monoConfigs,
                                  stereoConfigs: _tempConfig.stereoConfigs,
                                  multiConfigs: _tempConfig.multiConfigs,
                                  rooms: newRooms,
                                  isExhibitionMode:
                                      _tempConfig.isExhibitionMode,
                                  masterHeadroomDb:
                                      _tempConfig.masterHeadroomDb,
                                  peakLimiterEnabled:
                                      _tempConfig.peakLimiterEnabled,
                                  globalTrajectory:
                                      _tempConfig.globalTrajectory,
                                  roomZones: _tempConfig.roomZones,
                                );
                              });
                            },
                          ),
                        ),
                      ],
                    ),
                  );
                }),
              ],
            ),
          );
        }),
        const SizedBox(height: 24),
        const Divider(color: AppColors.darkGrey),
        const SizedBox(height: 16),
        const Text(
          'OSC 네트워크 서버 설정',
          style: TextStyle(
            color: AppColors.primaryNeon,
            fontWeight: FontWeight.bold,
          ),
        ),
        const SizedBox(height: 8),
        Row(
          children: [
            const Text('수신 포트 (Port): ', style: TextStyle(color: Colors.white)),
            SizedBox(
              width: 100,
              child: TextFormField(
                initialValue: _tempConfig.oscPort.toString(),
                style: const TextStyle(color: Colors.white),
                decoration: const InputDecoration(
                  isDense: true,
                  filled: true,
                  fillColor: AppColors.cardSurface,
                ),
                keyboardType: TextInputType.number,
                onChanged: (val) {
                  final p = int.tryParse(val);
                  if (p != null) {
                    setState(() {
                      _tempConfig = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, 
oscWhitelist: _tempConfig.oscWhitelist,

                        oscPort: int.tryParse(val) ?? _tempConfig.oscPort,
                        deviceName: _tempConfig.deviceName,
                        bufferSize: _tempConfig.bufferSize,
                        themeStartOscAddress: _tempConfig.themeStartOscAddress,
                        systemResetOscAddress:
                            _tempConfig.systemResetOscAddress,
                        monoConfigs: _tempConfig.monoConfigs,
                        stereoConfigs: _tempConfig.stereoConfigs,
                        multiConfigs: _tempConfig.multiConfigs,
                        rooms: _tempConfig.rooms,
                        isExhibitionMode: _tempConfig.isExhibitionMode,
                        masterHeadroomDb: _tempConfig.masterHeadroomDb,
                        peakLimiterEnabled: _tempConfig.peakLimiterEnabled,
                        globalTrajectory: _tempConfig.globalTrajectory,
                        roomZones: _tempConfig.roomZones,
                      );
                    });
                  }
                },
              ),
            ),
          ],
        ),
      ],
    );
  }
}

class OutputConfigDialog extends ConsumerStatefulWidget {
  final int channelCount;
  final Map<int, ChannelSetting> initialMonoConfigs;
  final Map<int, ChannelSetting> initialStereoConfigs;
  final Map<int, ChannelSetting> initialMultiConfigs;

  const OutputConfigDialog({
    super.key,
    required this.channelCount,
    required this.initialMonoConfigs,
    required this.initialStereoConfigs,
    required this.initialMultiConfigs,
  });

  @override
  ConsumerState<OutputConfigDialog> createState() => _OutputConfigDialogState();
}

class _OutputConfigDialogState extends ConsumerState<OutputConfigDialog> {
  late Map<int, ChannelSetting> monoConfigs;
  late Map<int, ChannelSetting> stereoConfigs;
  late Map<int, ChannelSetting> multiConfigs;

  @override
  void initState() {
    super.initState();
    monoConfigs = Map.from(widget.initialMonoConfigs);
    stereoConfigs = Map.from(widget.initialStereoConfigs);
    multiConfigs = Map.from(widget.initialMultiConfigs);
  }

  Widget _buildChannelRow(
    String label,
    ChannelSetting setting,
    ValueChanged<ChannelSetting> onChanged,
  ) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4.0, horizontal: 8.0),
      child: Row(
        children: [
          InkWell(
            onTap: () {
              onChanged(
                ChannelSetting(phaseInvert: false, gainDb: 0.0, enabled: !setting.enabled,
                  customName: setting.customName,
                  delayMs: setting.delayMs,
                  eqBands: setting.eqBands,
                ),
              );
            },
            child: Container(
              width: 80,
              height: 32,
              alignment: Alignment.center,
              decoration: BoxDecoration(
                color: setting.enabled ? Colors.orange : AppColors.cardSurface,
                border: Border.all(color: Colors.black54),
                borderRadius: BorderRadius.circular(4),
              ),
              child: Text(
                label,
                style: TextStyle(
                  color: setting.enabled ? Colors.black : Colors.white,
                  fontWeight: FontWeight.bold,
                ),
              ),
            ),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: SizedBox(
              height: 32,
              child: TextFormField(
                initialValue: setting.customName,
                style: const TextStyle(color: Colors.white, fontSize: 14),
                decoration: InputDecoration(
                  contentPadding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 0,
                  ),
                  filled: true,
                  fillColor: AppColors.cardSurface,
                  border: OutlineInputBorder(
                    borderSide: const BorderSide(color: Colors.black54),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  focusedBorder: OutlineInputBorder(
                    borderSide: const BorderSide(color: Colors.orange),
                    borderRadius: BorderRadius.circular(4),
                  ),
                ),
                onChanged: (val) {
                  onChanged(
                    ChannelSetting(phaseInvert: false, gainDb: 0.0, enabled: setting.enabled,
                      customName: val,
                      delayMs: setting.delayMs,
                      eqBands: setting.eqBands,
                    ),
                  );
                },
              ),
            ),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      backgroundColor: AppColors.background,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(4)),
      child: SizedBox(
        width: 700,
        height: 600,
        child: Column(
          children: [
            Container(
              padding: const EdgeInsets.all(16),
              decoration: const BoxDecoration(
                color: AppColors.headerBackground,
                borderRadius: BorderRadius.vertical(top: Radius.circular(4)),
              ),
              child: Row(
                children: [
                  const Text(
                    'Output Config',
                    style: TextStyle(
                      color: Colors.white,
                      fontSize: 18,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  const Spacer(),
                  IconButton(
                    icon: const Icon(Icons.close, color: Colors.white),
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                ],
              ),
            ),
            const Padding(
              padding: EdgeInsets.all(16.0),
              child: Text(
                'Choose which audio hardware outputs to make available. Each channel can be used as a 1-Ch (Mono) output or the start of an N-Ch (Multi-Channel) output array.',
                style: TextStyle(color: Colors.white70, fontSize: 13),
              ),
            ),
            Expanded(
              child: Row(
                children: [
                  // Mono Column
                  Expanded(
                    child: Column(
                      children: [
                        const Padding(
                          padding: EdgeInsets.all(8.0),
                          child: Align(
                            alignment: Alignment.centerLeft,
                            child: Text(
                              '1-Ch (Mono) Outputs',
                              style: TextStyle(
                                color: Colors.white,
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                          ),
                        ),
                        Expanded(
                          child: ListView.builder(
                            itemCount: widget.channelCount,
                            itemBuilder: (context, index) {
                              final chStart = index;
                              final displayCh1 = chStart + 1;
                              final key = displayCh1;
                              final setting =
                                  monoConfigs[key] ??
                                  const ChannelSetting(phaseInvert: false, gainDb: 0.0, enabled: false,
                                    customName: '',
                                    delayMs: 0.0,
                                    eqBands: [],
                                  );

                              return _buildChannelRow('$displayCh1', setting, (
                                newSetting,
                              ) {
                                setState(() {
                                  monoConfigs[key] = newSetting;
                                });
                              });
                            },
                          ),
                        ),
                      ],
                    ),
                  ),
                  const VerticalDivider(color: AppColors.darkGrey, width: 1),
                  // Stereo Column
                  Expanded(
                    child: Column(
                      children: [
                        const Padding(
                          padding: EdgeInsets.all(8.0),
                          child: Align(
                            alignment: Alignment.centerLeft,
                            child: Text(
                              '2-Ch (Stereo) Outputs',
                              style: TextStyle(
                                color: Colors.white,
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                          ),
                        ),
                        Expanded(
                          child: ListView.builder(
                            itemCount: (widget.channelCount / 2).ceil(),
                            itemBuilder: (context, index) {
                              final chStart = index * 2;
                              if (chStart >= widget.channelCount) {
                                return const SizedBox.shrink();
                              }
                              final displayCh1 = chStart + 1;
                              final displayCh2 = chStart + 2;
                              final key = displayCh1;
                              final setting =
                                  stereoConfigs[key] ??
                                  const ChannelSetting(phaseInvert: false, gainDb: 0.0, enabled: false,
                                    customName: '',
                                    delayMs: 0.0,
                                    eqBands: [],
                                  );

                              return _buildChannelRow(
                                '$displayCh1/$displayCh2',
                                setting,
                                (newSetting) {
                                  setState(() {
                                    stereoConfigs[key] = newSetting;
                                  });
                                },
                              );
                            },
                          ),
                        ),
                      ],
                    ),
                  ),
                  const VerticalDivider(color: AppColors.darkGrey, width: 1),
                  // Multi-Ch Column
                  Expanded(
                    child: Column(
                      children: [
                        const Padding(
                          padding: EdgeInsets.all(8.0),
                          child: Align(
                            alignment: Alignment.centerLeft,
                            child: Text(
                              'Multi-Ch (N-Ch) 시작',
                              style: TextStyle(
                                color: Colors.white,
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                          ),
                        ),
                        Expanded(
                          child: ListView.builder(
                            itemCount: widget.channelCount,
                            itemBuilder: (context, index) {
                              final chStart = index;
                              final displayCh1 = chStart + 1;
                              final key = displayCh1;
                              final setting =
                                  multiConfigs[key] ??
                                  const ChannelSetting(phaseInvert: false, gainDb: 0.0, enabled: false,
                                    customName: '',
                                    delayMs: 0.0,
                                    eqBands: [],
                                  );

                              return _buildChannelRow(
                                'Ch $displayCh1 시작',
                                setting,
                                (newSetting) {
                                  setState(() {
                                    multiConfigs[key] = newSetting;
                                  });
                                },
                              );
                            },
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
            Container(
              padding: const EdgeInsets.all(16),
              decoration: const BoxDecoration(
                border: Border(top: BorderSide(color: AppColors.darkGrey)),
              ),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  TextButton(
                    onPressed: () => Navigator.of(context).pop(),
                    child: const Text(
                      'Cancel',
                      style: TextStyle(color: Colors.white70),
                    ),
                  ),
                  const SizedBox(width: 16),
                  ElevatedButton(
                    style: ElevatedButton.styleFrom(
                      backgroundColor: AppColors.primaryBlue,
                    ),
                    onPressed: () {
                      Navigator.of(context).pop({
                        'mono': monoConfigs,
                        'stereo': stereoConfigs,
                        'multi': multiConfigs,
                      });
                    },
                    child: const Text(
                      'OK',
                      style: TextStyle(color: Colors.white),
                    ),
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
