import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;

class PreferencesModal extends ConsumerStatefulWidget {
  const PreferencesModal({super.key});

  @override
  ConsumerState<PreferencesModal> createState() => _PreferencesModalState();
}

class _PreferencesModalState extends ConsumerState<PreferencesModal>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;
  late AppConfig _tempConfig;

  static List<rust_api.OutputDeviceInfo>? _cachedDeviceInfos;
  static List<String>? _cachedDevices;

  List<rust_api.OutputDeviceInfo> _deviceInfos = [];
  List<String> _devices = [];
  List<String> _channelNames = [];
  String _selectedDriverType = 'WASAPI';

  String _getDriverType(String? deviceName) {
    if (deviceName == null) return 'WASAPI';
    if (deviceName.startsWith('[ASIO]')) return 'ASIO';
    if (deviceName.startsWith('[WASAPI]')) return 'WASAPI';
    if (deviceName.startsWith('[CoreAudio]')) return 'CoreAudio';
    return 'WASAPI';
  }

  String _getCleanDeviceName(String? deviceName) {
    if (deviceName == null) return '';
    if (deviceName.startsWith('[ASIO] ')) return deviceName.substring(7);
    if (deviceName.startsWith('[WASAPI] ')) return deviceName.substring(9);
    if (deviceName.startsWith('[CoreAudio] ')) return deviceName.substring(12);
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
        : AppConfig(
            oscPort: 8000,
            bufferSize: 256,
            themeStartOscAddress: '/theme/start',
            systemResetOscAddress: '/system/reset',
            monoConfigs: {},
            stereoConfigs: {},
            rooms: [],
          );
    _selectedDriverType = _getDriverType(_tempConfig.deviceName);
    _loadDevices();
  }

  Future<void> _loadDevices() async {
    if (_cachedDeviceInfos != null && _cachedDevices != null) {
      setState(() {
        _deviceInfos = _cachedDeviceInfos!;
        _devices = _cachedDevices!;
      });
      _loadDeviceChannels(_tempConfig.deviceName);
      return;
    }
    try {
      final deviceInfos = await rust_api.apiGetOutputDevices();
      final devices = deviceInfos.map((d) => d.name).toList();
      _cachedDeviceInfos = deviceInfos;
      _cachedDevices = devices;
      setState(() {
        _deviceInfos = deviceInfos;
        _devices = devices;
      });
      _loadDeviceChannels(_tempConfig.deviceName);
    } catch (e) {
      if (mounted) {
        ref.read(globalErrorProvider.notifier).showError('장치 스캔 실패: $e');
      }
    }
  }

  void _loadDeviceChannels(String? deviceName) {
    if (deviceName == null) {
      setState(() {
        _channelNames = [];
      });
      return;
    }
    try {
      final info = _deviceInfos.firstWhere((d) => d.name == deviceName);
      setState(() {
        _channelNames = info.channelNames;
      });
    } catch (e) {
      // Device not found in the cached list (e.g. disconnected)
      setState(() {
        _channelNames = [];
      });
    }
  }

  // Very basic deep clone for editing
  AppConfig cloneConfig(AppConfig config) {
    return AppConfig(
      oscPort: config.oscPort,
      deviceName: config.deviceName,
      bufferSize: config.bufferSize,
      themeStartOscAddress: config.themeStartOscAddress,
      systemResetOscAddress: config.systemResetOscAddress,
      monoConfigs: Map.from(config.monoConfigs),
      stereoConfigs: Map.from(config.stereoConfigs),
      rooms: config.rooms
          .map(
            (r) => RoomConfig(
              id: r.id,
              name: r.name,
              colorHex: r.colorHex,
              volume: r.volume,
              clearOscAddress: r.clearOscAddress,
              tracks: r.tracks
                  .map(
                    (t) => TrackConfig(
                      id: t.id,
                      name: t.name,
                      filePath: t.filePath,
                      volume: t.volume,
                      isLoop: t.isLoop,
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
        tracks: newTracks,
      );
    }).toList();

    final finalConfig = AppConfig(
      oscPort: _tempConfig.oscPort,
      deviceName: _tempConfig.deviceName,
      bufferSize: _tempConfig.bufferSize,
      themeStartOscAddress: _tempConfig.themeStartOscAddress,
      systemResetOscAddress: _tempConfig.systemResetOscAddress,
      monoConfigs: _tempConfig.monoConfigs,
      stereoConfigs: _tempConfig.stereoConfigs,
      rooms: newRooms,
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
              tracks: updatedTracks,
            );
          }).toList();

          _tempConfig = AppConfig(
            oscPort: _tempConfig.oscPort,
            deviceName: _tempConfig.deviceName,
            bufferSize: _tempConfig.bufferSize,
            themeStartOscAddress: _tempConfig.themeStartOscAddress,
            systemResetOscAddress: next.systemResetOscAddress,
            monoConfigs: Map.from(next.monoConfigs),
            stereoConfigs: Map.from(next.stereoConfigs),
            rooms: updatedRooms,
          );
        });
      }
    });

    return Dialog(
      backgroundColor: AppColors.background,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
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
                borderRadius: BorderRadius.vertical(top: Radius.circular(12)),
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

  Widget _buildAudioTab() {
    final List<DropdownMenuItem<String>> channelItems = [];

    final sortedMono =
        _tempConfig.monoConfigs.entries.where((e) => e.value.enabled).toList()
          ..sort((a, b) => a.key.compareTo(b.key));
    for (final entry in sortedMono) {
      final key = entry.key;
      final setting = entry.value;
      
      final realCh1 = key - 1;
      if (realCh1 < _channelNames.length) {
        final name1 = setting.customName.isNotEmpty ? '$key (${setting.customName} L)' : '$key';
        channelItems.add(DropdownMenuItem<String>(value: '${realCh1}_mono', child: Text('Mono $name1')));
      }
      
      final realCh2 = key;
      final displayCh2 = key + 1;
      if (realCh2 < _channelNames.length) {
        final name2 = setting.customName.isNotEmpty ? '$displayCh2 (${setting.customName} R)' : '$displayCh2';
        channelItems.add(DropdownMenuItem<String>(value: '${realCh2}_mono', child: Text('Mono $name2')));
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
      if (realCh < _channelNames.length) {
        final displayName = setting.customName.isNotEmpty
            ? '$key/$displayCh2 (${setting.customName})'
            : '$key/$displayCh2';
        channelItems.add(
          DropdownMenuItem<String>(
            value: '${realCh}_stereo',
            child: Text('Stereo $displayName'),
          ),
        );
      }
    }

    String getDropdownValue(int channelIndex, bool isStereo) {
      return isStereo ? '${channelIndex}_stereo' : '${channelIndex}_mono';
    }

    final uniqueDriverTypes = Platform.isMacOS ? ['CoreAudio'] : ['WASAPI', 'ASIO'];
    if (!uniqueDriverTypes.contains(_selectedDriverType)) {
      _selectedDriverType = uniqueDriverTypes.first;
    }
    
    final filteredDevices = _devices.where((d) => _getDriverType(d) == _selectedDriverType).toList();

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
                  contentPadding: EdgeInsets.symmetric(horizontal: 12, vertical: 12),
                ),
                items: uniqueDriverTypes.map((t) => DropdownMenuItem(value: t, child: Text(t))).toList(),
                onChanged: (val) {
                  if (val != null) {
                    setState(() {
                      _selectedDriverType = val;
                      final newFilteredDevices = _devices.where((d) => _getDriverType(d) == val).toList();
                      String? newDeviceName;
                      if (newFilteredDevices.isNotEmpty) {
                        newDeviceName = newFilteredDevices.first;
                      } else {
                        newDeviceName = null;
                      }
                      
                      _tempConfig = AppConfig(
                        oscPort: _tempConfig.oscPort,
                        deviceName: newDeviceName,
                        bufferSize: _tempConfig.bufferSize,
                        themeStartOscAddress: _tempConfig.themeStartOscAddress,
                        systemResetOscAddress: _tempConfig.systemResetOscAddress,
                        monoConfigs: _tempConfig.monoConfigs,
                        stereoConfigs: _tempConfig.stereoConfigs,
                        rooms: _tempConfig.rooms,
                      );
                    });
                    _loadDeviceChannels(_tempConfig.deviceName);
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
                  contentPadding: EdgeInsets.symmetric(horizontal: 12, vertical: 12),
                ),
                items: [
                  if (filteredDevices.isEmpty)
                    const DropdownMenuItem<String>(
                      value: null,
                      child: Text('장치 없음', style: TextStyle(color: Colors.white54)),
                    )
                  else
                    const DropdownMenuItem<String>(
                      value: null,
                      child: Text('기본 오디오 출력 (Default)'),
                    ),
                  ...filteredDevices.map(
                    (d) => DropdownMenuItem(value: d, child: Text(_getCleanDeviceName(d))),
                  ),
                  if (_tempConfig.deviceName != null &&
                      !_devices.contains(_tempConfig.deviceName))
                    DropdownMenuItem(
                      value: _tempConfig.deviceName,
                      child: Text('${_getCleanDeviceName(_tempConfig.deviceName!)} (Disconnected)'),
                    ),
                ],
                onChanged: (val) {
                  setState(() {
                    _tempConfig = AppConfig(
                      oscPort: _tempConfig.oscPort,
                      deviceName: val,
                      bufferSize: _tempConfig.bufferSize,
                      themeStartOscAddress: _tempConfig.themeStartOscAddress,
                      systemResetOscAddress: _tempConfig.systemResetOscAddress,
                      monoConfigs: _tempConfig.monoConfigs,
                      stereoConfigs: _tempConfig.stereoConfigs,
                      rooms: _tempConfig.rooms,
                    );
                  });
                  _loadDeviceChannels(val);
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
                      _tempConfig = AppConfig(
                        oscPort: _tempConfig.oscPort,
                        deviceName: _tempConfig.deviceName,
                        bufferSize: val,
                        themeStartOscAddress: _tempConfig.themeStartOscAddress,
                        systemResetOscAddress:
                            _tempConfig.systemResetOscAddress,
                        monoConfigs: _tempConfig.monoConfigs,
                        stereoConfigs: _tempConfig.stereoConfigs,
                        rooms: _tempConfig.rooms,
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
            borderRadius: BorderRadius.circular(8),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                '현재 선택된 오디오 인터페이스의 아웃풋 채널은 총 ${_channelNames.length}개 입니다.',
                style: const TextStyle(color: Colors.white70),
              ),
              const SizedBox(height: 12),
              ElevatedButton(
                onPressed: () async {
                  final result =
                      await showDialog<Map<String, Map<int, ChannelSetting>>>(
                        context: context,
                        builder: (context) => OutputConfigDialog(
                          channelCount: _channelNames.length,
                          initialMonoConfigs: _tempConfig.monoConfigs,
                          initialStereoConfigs: _tempConfig.stereoConfigs,
                        ),
                      );
                  if (result != null) {
                    setState(() {
                      _tempConfig = AppConfig(
                        oscPort: _tempConfig.oscPort,
                        deviceName: _tempConfig.deviceName,
                        bufferSize: _tempConfig.bufferSize,
                        themeStartOscAddress: _tempConfig.themeStartOscAddress,
                        systemResetOscAddress:
                            _tempConfig.systemResetOscAddress,
                        monoConfigs: result['mono']!,
                        stereoConfigs: result['stereo']!,
                        rooms: _tempConfig.rooms,
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
                          initialValue: getDropdownValue(
                            track.outputChannel,
                            track.outputStereo,
                          ),
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
                            ...channelItems,
                            if (!channelItems.any(
                              (item) =>
                                  item.value ==
                                  getDropdownValue(
                                    track.outputChannel,
                                    track.outputStereo,
                                  ),
                            ))
                              DropdownMenuItem(
                                value: getDropdownValue(
                                  track.outputChannel,
                                  track.outputStereo,
                                ),
                                child: Text(
                                  '${track.outputChannel + 1} (Missing)',
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                          ],
                          onChanged: (val) {
                            if (val != null) {
                              final isStereo = val.endsWith('_stereo');
                              final parsedChannel = int.parse(
                                val.split('_').first,
                              );
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
                                tracks: newTracks,
                              );
                              setState(() {
                                _tempConfig = AppConfig(
                                  oscPort: _tempConfig.oscPort,
                                  deviceName: _tempConfig.deviceName,
                                  bufferSize: _tempConfig.bufferSize,
                                  themeStartOscAddress:
                                      _tempConfig.themeStartOscAddress,
                                  systemResetOscAddress:
                                      _tempConfig.systemResetOscAddress,
                                  monoConfigs: _tempConfig.monoConfigs,
                                  stereoConfigs: _tempConfig.stereoConfigs,
                                  rooms: newRooms,
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
            borderRadius: BorderRadius.circular(8),
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
                      _tempConfig = AppConfig(
                        oscPort: _tempConfig.oscPort,
                        deviceName: _tempConfig.deviceName,
                        bufferSize: _tempConfig.bufferSize,
                        themeStartOscAddress: val,
                        systemResetOscAddress:
                            _tempConfig.systemResetOscAddress,
                        monoConfigs: _tempConfig.monoConfigs,
                        stereoConfigs: _tempConfig.stereoConfigs,
                        rooms: _tempConfig.rooms,
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
            borderRadius: BorderRadius.circular(8),
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
                      _tempConfig = AppConfig(
                        oscPort: _tempConfig.oscPort,
                        deviceName: _tempConfig.deviceName,
                        bufferSize: _tempConfig.bufferSize,
                        themeStartOscAddress: _tempConfig.themeStartOscAddress,
                        systemResetOscAddress: val,
                        monoConfigs: _tempConfig.monoConfigs,
                        stereoConfigs: _tempConfig.stereoConfigs,
                        rooms: _tempConfig.rooms,
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
              borderRadius: BorderRadius.circular(8),
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
                            tracks: room.tracks,
                          );
                          setState(() {
                            _tempConfig = AppConfig(
                              oscPort: _tempConfig.oscPort,
                              deviceName: _tempConfig.deviceName,
                              bufferSize: _tempConfig.bufferSize,
                              themeStartOscAddress:
                                  _tempConfig.themeStartOscAddress,
                              systemResetOscAddress:
                                  _tempConfig.systemResetOscAddress,
                              monoConfigs: _tempConfig.monoConfigs,
                              stereoConfigs: _tempConfig.stereoConfigs,
                              rooms: newRooms,
                            );
                          });
                        },
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
                                tracks: newTracks,
                              );
                              setState(() {
                                _tempConfig = AppConfig(
                                  oscPort: _tempConfig.oscPort,
                                  deviceName: _tempConfig.deviceName,
                                  bufferSize: _tempConfig.bufferSize,
                                  themeStartOscAddress:
                                      _tempConfig.themeStartOscAddress,
                                  systemResetOscAddress:
                                      _tempConfig.systemResetOscAddress,
                                  monoConfigs: _tempConfig.monoConfigs,
                                  stereoConfigs: _tempConfig.stereoConfigs,
                                  rooms: newRooms,
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
                                tracks: newTracks,
                              );
                              setState(() {
                                _tempConfig = AppConfig(
                                  oscPort: _tempConfig.oscPort,
                                  deviceName: _tempConfig.deviceName,
                                  bufferSize: _tempConfig.bufferSize,
                                  themeStartOscAddress:
                                      _tempConfig.themeStartOscAddress,
                                  systemResetOscAddress:
                                      _tempConfig.systemResetOscAddress,
                                  monoConfigs: _tempConfig.monoConfigs,
                                  stereoConfigs: _tempConfig.stereoConfigs,
                                  rooms: newRooms,
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
                      _tempConfig = AppConfig(
                        oscPort: int.tryParse(val) ?? _tempConfig.oscPort,
                        deviceName: _tempConfig.deviceName,
                        bufferSize: _tempConfig.bufferSize,
                        themeStartOscAddress: _tempConfig.themeStartOscAddress,
                        systemResetOscAddress:
                            _tempConfig.systemResetOscAddress,
                        monoConfigs: _tempConfig.monoConfigs,
                        stereoConfigs: _tempConfig.stereoConfigs,
                        rooms: _tempConfig.rooms,
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

  const OutputConfigDialog({
    super.key,
    required this.channelCount,
    required this.initialMonoConfigs,
    required this.initialStereoConfigs,
  });

  @override
  ConsumerState<OutputConfigDialog> createState() => _OutputConfigDialogState();
}

class _OutputConfigDialogState extends ConsumerState<OutputConfigDialog> {
  late Map<int, ChannelSetting> monoConfigs;
  late Map<int, ChannelSetting> stereoConfigs;

  @override
  void initState() {
    super.initState();
    monoConfigs = Map.from(widget.initialMonoConfigs);
    stereoConfigs = Map.from(widget.initialStereoConfigs);
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
                ChannelSetting(
                  enabled: !setting.enabled,
                  customName: setting.customName,
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
                    ChannelSetting(enabled: setting.enabled, customName: val),
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
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      child: SizedBox(
        width: 700,
        height: 600,
        child: Column(
          children: [
            Container(
              padding: const EdgeInsets.all(16),
              decoration: const BoxDecoration(
                color: AppColors.headerBackground,
                borderRadius: BorderRadius.vertical(top: Radius.circular(12)),
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
                'Choose which audio hardware outputs to make available. Every output pair can be used as one stereo out and/or two mono outs.',
                style: TextStyle(color: Colors.white70),
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
                              'Mono Outputs',
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
                                  monoConfigs[key] ??
                                  const ChannelSetting(
                                    enabled: false,
                                    customName: '',
                                  );

                              return _buildChannelRow(
                                '$displayCh1 & $displayCh2',
                                setting,
                                (newSetting) {
                                  setState(() {
                                    monoConfigs[key] = newSetting;
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
                  // Stereo Column
                  Expanded(
                    child: Column(
                      children: [
                        const Padding(
                          padding: EdgeInsets.all(8.0),
                          child: Align(
                            alignment: Alignment.centerLeft,
                            child: Text(
                              'Stereo Outputs',
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
                                  const ChannelSetting(
                                    enabled: false,
                                    customName: '',
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
                      Navigator.of(
                        context,
                      ).pop({'mono': monoConfigs, 'stereo': stereoConfigs});
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
