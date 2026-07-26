import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

Future<String> _getConfigPath() async {
  final dir = await getApplicationSupportDirectory();
  return '${dir.path}/config.json';
}

class GlobalDeviceCache {
  static List<String>? devices;
  static Map<String, List<String>> channels = {};
}

class _SaveTask {
  final AppConfig config;
  final bool forceRestart;
  final bool skipPreload;
  _SaveTask(this.config, this.forceRestart, this.skipPreload);
}

class ConfigNotifier extends Notifier<AppConfig?> {
  bool _isSaving = false;
  final List<_SaveTask> _saveQueue = [];
  AppConfig? _lastProcessedConfig;

  @override
  AppConfig? build() {
    // Initial load will be handled by the splash screen
    return null;
  }

  Future<void> loadConfigAsync() async {
    try {
      final path = await _getConfigPath();
      final config = await rust_api.apiGetConfig(path: path);
      try {
        await rust_api.apiPreloadAllSounds(config: config);
      } catch (e) {
        // Ignore initial preload errors
      }

      _lastProcessedConfig = config;
      state = config;
      ref.read(tuningStateProvider.notifier).syncFromBackendConfig(config);
      await rust_api.apiInitAudioSystem(deviceName: config.deviceName);
      await rust_api.apiStartOscListener(port: config.oscPort);

      try {
        final deviceInfos = await rust_api.apiGetOutputDevices();
        GlobalDeviceCache.devices = deviceInfos.map((d) => d.name).toList();
        for (final info in deviceInfos) {
          GlobalDeviceCache.channels[info.name] = info.channelNames;
        }
      } catch (e) {
        // Ignore background scan errors
      }
    } catch (e) {
      ref.read(globalErrorProvider.notifier).showError('설정 로드 실패: $e');
    }
  }

  void saveConfig(
    AppConfig newConfig, {
    bool forceRestart = false,
    bool skipPreload = false,
  }) {
    // Update config before save so UI reacts quickly
    state = newConfig;

    _saveQueue.add(_SaveTask(newConfig, forceRestart, skipPreload));
    _processQueue();
  }

  Future<void> _processQueue() async {
    if (_isSaving) return;
    _isSaving = true;

    while (_saveQueue.isNotEmpty) {
      final task = _saveQueue.removeAt(0);
      final configToSave = task.config;
      final oldConfig = _lastProcessedConfig;

      try {
        final path = await _getConfigPath();
        await rust_api.apiSaveConfig(path: path, config: configToSave);
        if (!task.skipPreload) {
          try {
            await rust_api.apiPreloadAllSounds(config: configToSave);
          } catch (e) {
            // Ignore preload errors, keep UI responsive
          }
        }

        bool engineNeedsRestart =
            oldConfig == null ||
            oldConfig.deviceName != configToSave.deviceName ||
            oldConfig.bufferSize != configToSave.bufferSize ||
            task.forceRestart;

        if (!task.forceRestart &&
            engineNeedsRestart &&
            oldConfig != null &&
            oldConfig.deviceName != null &&
            configToSave.deviceName != null) {
          final oldName = oldConfig.deviceName!.trim();
          final newName = configToSave.deviceName!.trim();

          if (oldName != newName) {
            String stripPrefix(String name) {
              return name.replaceFirst(RegExp(r'^\[.*?\]\s*'), '');
            }

            if (stripPrefix(oldName) == stripPrefix(newName)) {
              final oldHasPrefix = oldName.startsWith('[');
              final newHasPrefix = newName.startsWith('[');

              if (!oldHasPrefix ||
                  !newHasPrefix ||
                  oldName.split(']').first == newName.split(']').first) {
                engineNeedsRestart =
                    oldConfig.bufferSize != configToSave.bufferSize;
              }
            }
          }
        }

        if (engineNeedsRestart) {
          await rust_api.apiInitAudioSystem(
            deviceName: configToSave.deviceName,
          );

          if (!(await rust_api.apiIsEngineReady())) {
            try {
              await rust_api
                  .apiCreateDeviceEventStream()
                  .firstWhere((e) => e == 'EngineReady')
                  .timeout(const Duration(milliseconds: 5000));
            } catch (e) {
              ref.read(globalErrorProvider.notifier).showError('오디오 엔진 연결 시간 초과 (Timeout). 오디오 장치 연결 상태를 확인해주세요.');
            }
          }
          ref.read(tuningStateProvider.notifier).applyAllToBackend();
        }

        if (oldConfig == null || oldConfig.oscPort != configToSave.oscPort) {
          await rust_api.apiStartOscListener(port: configToSave.oscPort);
        }

        // Apply Global DSP Settings dynamically
        if (oldConfig == null ||
            oldConfig.masterHeadroomDb != configToSave.masterHeadroomDb ||
            oldConfig.peakLimiterEnabled != configToSave.peakLimiterEnabled) {
          await rust_api.apiApplyGlobalTuning(
            masterHeadroomDb: configToSave.masterHeadroomDb,
            peakLimiterEnabled: configToSave.peakLimiterEnabled,
          );
        }

        _lastProcessedConfig = configToSave;
      } catch (e) {
        ref.read(globalErrorProvider.notifier).showError('설정 저장 실패: $e');
      }
    }

    _isSaving = false;
  }
}

final configProvider = NotifierProvider<ConfigNotifier, AppConfig?>(
  ConfigNotifier.new,
);

final vuStreamProvider = StreamProvider<List<double>>((ref) {
  return rust_api.apiCreateVuStream().asBroadcastStream();
});

final deviceEventStreamProvider = StreamProvider<String>((ref) {
  return rust_api.apiCreateDeviceEventStream().asBroadcastStream();
});

final hardwareChannelsProvider = FutureProvider<List<String>>((ref) async {
  final deviceName = ref.watch(configProvider.select((c) => c?.deviceName));
  if (deviceName != null &&
      GlobalDeviceCache.channels.containsKey(deviceName)) {
    return GlobalDeviceCache.channels[deviceName]!;
  }
  try {
    return await rust_api.apiGetDeviceChannelNames(deviceName: deviceName);
  } catch (e) {
    return [];
  }
});

class EngineState {
  final String? activeRoomId;
  final Set<String> clearedRoomIds;
  final bool duckingActive;
  final bool themeStarted;
  final List<String> playingTrackIds;
  final bool masterMuteActive;
  final int outputChannelCount;

  EngineState({
    this.activeRoomId,
    this.clearedRoomIds = const {},
    this.duckingActive = false,
    this.themeStarted = false,
    this.playingTrackIds = const [],
    this.masterMuteActive = false,
    this.outputChannelCount = 2,
  });

  EngineState copyWith({
    String? activeRoomId,
    bool forceNullActiveRoom = false,
    Set<String>? clearedRoomIds,
    bool? duckingActive,
    bool? themeStarted,
    List<String>? playingTrackIds,
    bool? masterMuteActive,
    int? outputChannelCount,
  }) {
    return EngineState(
      activeRoomId: forceNullActiveRoom
          ? null
          : (activeRoomId ?? this.activeRoomId),
      clearedRoomIds: clearedRoomIds ?? this.clearedRoomIds,
      duckingActive: duckingActive ?? this.duckingActive,
      themeStarted: themeStarted ?? this.themeStarted,
      playingTrackIds: playingTrackIds ?? this.playingTrackIds,
      masterMuteActive: masterMuteActive ?? this.masterMuteActive,
      outputChannelCount: outputChannelCount ?? this.outputChannelCount,
    );
  }
}

class EngineStateNotifier extends Notifier<EngineState> {
  @override
  EngineState build() {
    // Subscribe to rust_api.apiCreateEngineStateStream()
    final stream = rust_api.apiCreateEngineStateStream();
    final sub = stream.listen((update) {
      state = state.copyWith(
        activeRoomId: update.activeRoomId,
        forceNullActiveRoom: update.activeRoomId == null,
        duckingActive: update.duckingActive,
        playingTrackIds: update.playingTrackIds,
        outputChannelCount: update.outputChannelCount,
      );
    });
    ref.onDispose(() => sub.cancel());

    return EngineState();
  }

  Future<void> setActiveRoom(String roomId) async {
    try {
      await rust_api.apiSetActiveRoom(roomId: roomId);
    } catch (e) {
      // Ignored or handled elsewhere
    }
  }

  Future<void> clearActiveRoom() async {
    try {
      await rust_api.apiSetActiveRoom(roomId: null);
    } catch (e) {
      // ignored
    }
  }

  void clearRoom(String roomId) {
    final newCleared = Set<String>.from(state.clearedRoomIds)..add(roomId);
    state = state.copyWith(clearedRoomIds: newCleared);
  }

  Future<void> startTheme(String firstRoomId) async {
    state = state.copyWith(themeStarted: true, clearedRoomIds: {});
    try {
      await rust_api.apiSetActiveRoom(roomId: firstRoomId);
    } catch (e) {
      // ignored
    }
  }

  void toggleMasterMute() {
    state = state.copyWith(masterMuteActive: !state.masterMuteActive);
  }

  void reset() {
    state = state.copyWith(
      themeStarted: false,
      clearedRoomIds: {},
      masterMuteActive: false,
    );
  }
}

final engineStateProvider = NotifierProvider<EngineStateNotifier, EngineState>(
  EngineStateNotifier.new,
);

class GlobalErrorNotifier extends Notifier<String?> {
  @override
  String? build() {
    try {
      ref.listen(deviceEventStreamProvider, (previous, next) {
        final event = next.value;
        if (event != null && (event.contains("DeviceNotAvailable") || event.contains("Disconnected"))) {
          state = "오디오 장치와 연결이 끊어졌습니다. 설정에서 오디오 장치를 다시 확인해 주세요.";
        }
      });
    } catch (_) {}
    return null;
  }

  void showError(String message) {
    state = message;
  }

  void clearError() {
    state = null;
  }
}

final globalErrorProvider = NotifierProvider<GlobalErrorNotifier, String?>(
  GlobalErrorNotifier.new,
);

class OutputConfigState {
  final Set<int> monoChannels;
  final Set<int> stereoChannels; // Storing the first channel index of the pair

  OutputConfigState({
    this.monoChannels = const {},
    this.stereoChannels = const {},
  });

  OutputConfigState copyWith({
    Set<int>? monoChannels,
    Set<int>? stereoChannels,
  }) {
    return OutputConfigState(
      monoChannels: monoChannels ?? this.monoChannels,
      stereoChannels: stereoChannels ?? this.stereoChannels,
    );
  }
}

class OutputConfigNotifier extends Notifier<OutputConfigState> {
  @override
  OutputConfigState build() {
    _load();
    return OutputConfigState();
  }

  Future<void> _load() async {
    final prefs = await SharedPreferences.getInstance();
    final monoList = prefs.getStringList('output_config_mono') ?? [];
    final stereoList = prefs.getStringList('output_config_stereo') ?? [];

    state = OutputConfigState(
      monoChannels: monoList.map(int.parse).toSet(),
      stereoChannels: stereoList.map(int.parse).toSet(),
    );
  }

  Future<void> save(Set<int> mono, Set<int> stereo) async {
    state = OutputConfigState(monoChannels: mono, stereoChannels: stereo);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setStringList(
      'output_config_mono',
      mono.map((e) => e.toString()).toList(),
    );
    await prefs.setStringList(
      'output_config_stereo',
      stereo.map((e) => e.toString()).toList(),
    );
  }
}

final outputConfigProvider =
    NotifierProvider<OutputConfigNotifier, OutputConfigState>(
      OutputConfigNotifier.new,
    );
