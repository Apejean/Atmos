import 'package:flutter/foundation.dart' show visibleForTesting;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/src/rust/api/error.dart';
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

/// [outputChannelsProvider]가 노출하는 채널 목록 조회 상태.
///
/// `hardwareChannelsProvider`(FutureProvider)는 실패 시 조용히 `[]`을 반환해서
/// 소비 위젯이 "채널이 0개"와 "조회 실패"를 구분할 수 없었다. 이 열거형은 그
/// 둘을 분리한다.
enum OutputChannelsStatus {
  /// 최초 조회 중이거나 장치 전환으로 재조회 중.
  loading,

  /// 정상 조회 완료. [OutputChannelsState.channelNames]가 진실.
  ready,

  /// 선택된 장치가 시스템에서 사라졌거나(예: "Device not found"), `deviceName`이
  /// null(기본 장치 사용)인데 시스템에 기본 출력 장치 자체가 없는 경우
  /// (예: "No default output device"). 두 경우 모두 "실제로 라우팅할 출력이
  /// 없다"는 동일한 의미이므로 같은 상태로 취급한다.
  noDevice,

  /// 위 두 경우를 제외한 예외(권한 문제, 드라이버 오류 등). 무음 실패를 막기
  /// 위해 [OutputChannelsState.errorMessage]를 채우고
  /// [OutputChannelsState.lastKnownGoodChannelNames]를 유지한다.
  error,
}

class OutputChannelsState {
  final OutputChannelsStatus status;
  final String? deviceName;

  /// 0-based 인덱스 = 실제 하드웨어 채널. `status`에 따라 다음처럼 채워진다:
  /// - loading: 이전에 성공한 목록([lastKnownGoodChannelNames])을 그대로 유지
  /// - ready: 방금 조회한 목록
  /// - noDevice: 항상 빈 목록
  /// - error: 이전에 성공한 목록을 유지(있다면)
  final List<String> channelNames;
  final String? errorMessage;

  /// 마지막으로 조회에 성공했을 때의 채널 목록. `status`와 무관하게 유지되어
  /// 순간적인 재조회 실패로 UI가 텅 비어버리지 않게 한다.
  final List<String>? lastKnownGoodChannelNames;

  const OutputChannelsState({
    required this.status,
    this.deviceName,
    this.channelNames = const [],
    this.errorMessage,
    this.lastKnownGoodChannelNames,
  });
}

/// `hardwareChannelsProvider`를 대체할 신규 단일 진실 원천 provider.
///
/// B3에서 각 소비 위젯(환경설정 트랙 매핑, 메인화면 Ext. Out, FX, 스피커
/// 인스펙터 등)이 이 provider로 옮겨갈 때까지 `hardwareChannelsProvider`는
/// 삭제하지 않는다(동시에 깨지는 것을 방지).
class OutputChannelsNotifier extends Notifier<OutputChannelsState> {
  @override
  OutputChannelsState build() {
    // configProvider의 deviceName이 바뀔 때만 재조회한다. ref.watch 대신
    // ref.listen을 쓰는 이유: watch를 쓰면 의존성이 바뀔 때마다 이 Notifier
    // 인스턴스 자체가 재생성되어 lastKnownGoodChannelNames가 매번 사라진다.
    ref.listen<String?>(configProvider.select((c) => c?.deviceName), (
      previous,
      next,
    ) {
      if (previous != next) _refresh();
    });

    // 핫플러그 이벤트(장치 연결/해제) 수신 시 재조회.
    ref.listen(deviceEventStreamProvider, (previous, next) {
      _refresh();
    });

    // build() 도중에는 state를 변경할 수 없으므로 최초 조회는 build()가 끝난
    // 다음 이벤트 루프 턴으로 미룬다.
    Future.microtask(_refresh);

    return const OutputChannelsState(status: OutputChannelsStatus.loading);
  }

  Future<void> _refresh() async {
    final deviceName = ref.read(configProvider)?.deviceName;
    final previousGood = state.lastKnownGoodChannelNames;

    state = OutputChannelsState(
      status: OutputChannelsStatus.loading,
      deviceName: deviceName,
      channelNames: previousGood ?? const [],
      lastKnownGoodChannelNames: previousGood,
    );

    if (deviceName != null &&
        GlobalDeviceCache.channels.containsKey(deviceName)) {
      final cached = GlobalDeviceCache.channels[deviceName]!;
      state = OutputChannelsState(
        status: OutputChannelsStatus.ready,
        deviceName: deviceName,
        channelNames: cached,
        lastKnownGoodChannelNames: cached,
      );
      return;
    }

    try {
      final names = await fetchChannelNames(deviceName);
      if (names.isEmpty) {
        state = OutputChannelsState(
          status: OutputChannelsStatus.noDevice,
          deviceName: deviceName,
          channelNames: const [],
          lastKnownGoodChannelNames: previousGood,
        );
        return;
      }
      if (deviceName != null) {
        GlobalDeviceCache.channels[deviceName] = names;
      }
      state = OutputChannelsState(
        status: OutputChannelsStatus.ready,
        deviceName: deviceName,
        channelNames: names,
        lastKnownGoodChannelNames: names,
      );
    } catch (e) {
      final message = e is AtmosError ? e.message : e.toString();
      // "No default output device": deviceName이 null인데 시스템 기본 출력
      // 장치 자체가 없음. "Device not found": 저장된 장치가 시스템에서
      // 사라짐. 둘 다 "실제로 라우팅할 출력이 없다"는 동일한 의미다.
      final isNoDevice =
          message.contains('No default output device') ||
          message.contains('Device not found');
      state = OutputChannelsState(
        status: isNoDevice
            ? OutputChannelsStatus.noDevice
            : OutputChannelsStatus.error,
        deviceName: deviceName,
        channelNames: isNoDevice ? const [] : (previousGood ?? const []),
        errorMessage: isNoDevice ? null : message,
        lastKnownGoodChannelNames: previousGood,
      );
    }
  }

  /// 실제 FFI 호출. 이 Notifier를 상속한 테스트용 Mock이 오버라이드할 수
  /// 있도록 별도 메서드로 분리했다(`MockConfigNotifier`/
  /// `MockEngineStateNotifier`와 동일한 테스트 패턴).
  @visibleForTesting
  Future<List<String>> fetchChannelNames(String? deviceName) =>
      rust_api.apiGetDeviceChannelNames(deviceName: deviceName);
}

final outputChannelsProvider =
    NotifierProvider<OutputChannelsNotifier, OutputChannelsState>(
      OutputChannelsNotifier.new,
    );

class EngineState {
  final String? activeRoomId;
  final Set<String> clearedRoomIds;
  final bool duckingActive;
  final bool themeStarted;
  final List<String> playingTrackIds;
  final bool masterMuteActive;
  final int outputChannelCount;
  final double shortTermLufs;

  EngineState({
    this.activeRoomId,
    this.clearedRoomIds = const {},
    this.duckingActive = false,
    this.themeStarted = false,
    this.playingTrackIds = const [],
    this.masterMuteActive = false,
    this.outputChannelCount = 2,
    this.shortTermLufs = -70.0,
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
    double? shortTermLufs,
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
      shortTermLufs: shortTermLufs ?? this.shortTermLufs,
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
        shortTermLufs: update.shortTermLufs,
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
