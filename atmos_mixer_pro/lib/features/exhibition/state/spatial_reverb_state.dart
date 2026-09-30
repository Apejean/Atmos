import 'dart:async';
import 'dart:convert';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

const _kSpatialReverbPrefsKey = 'spatial_reverb_state';

enum ReverbType {
  room(
    label: 'Room',
    description: 'Natural small/medium room acoustics',
    defaultSize: 180.0,
    defaultDecay: 1.20,
    defaultPreDelay: 10.0,
    defaultDamp: 50.0,
    defaultDensity: 65.0,
  ),
  hall(
    label: 'Hall',
    description: 'Spacious and lush concert hall',
    defaultSize: 800.0,
    defaultDecay: 3.20,
    defaultPreDelay: 30.0,
    defaultDamp: 40.0,
    defaultDensity: 80.0,
  ),
  plate(
    label: 'Plate',
    description: 'Classic bright and dense metallic plate',
    defaultSize: 350.0,
    defaultDecay: 2.50,
    defaultPreDelay: 5.0,
    defaultDamp: 15.0,
    defaultDensity: 90.0,
  ),
  chamber(
    label: 'Chamber',
    description: 'Dense studio acoustic echo chamber',
    defaultSize: 450.0,
    defaultDecay: 2.20,
    defaultPreDelay: 18.0,
    defaultDamp: 35.0,
    defaultDensity: 75.0,
  ),
  cathedral(
    label: 'Cathedral',
    description: 'Massive stone space with majestic long tail',
    defaultSize: 1500.0,
    defaultDecay: 5.50,
    defaultPreDelay: 45.0,
    defaultDamp: 30.0,
    defaultDensity: 95.0,
  ),
  ambience(
    label: 'Ambience',
    description: 'Transparent short early reflections without mud',
    defaultSize: 100.0,
    defaultDecay: 0.60,
    defaultPreDelay: 0.0,
    defaultDamp: 60.0,
    defaultDensity: 50.0,
  );

  final String label;
  final String description;
  final double defaultSize;
  final double defaultDecay;
  final double defaultPreDelay;
  final double defaultDamp;
  final double defaultDensity;

  const ReverbType({
    required this.label,
    required this.description,
    required this.defaultSize,
    required this.defaultDecay,
    required this.defaultPreDelay,
    required this.defaultDamp,
    required this.defaultDensity,
  });
}

class SpatialReverbSettings {
  // Master Power
  final bool isEnabled;

  // 1. Input Processing
  final bool loCutEnabled;
  final double loCutFreq; // Hz (20 ~ 2000)
  final bool hiCutEnabled;
  final double hiCutFreq; // Hz (500 ~ 20000)
  final double preDelayMs; // ms (0.5 ~ 100)

  // 2. Early Reflections
  final bool spinEnabled;
  final double spinRate; // Hz (0.05 ~ 5.0)
  final double spinAmount; // 0.0 ~ 100.0
  final double shape; // 0.0 ~ 1.0 (Diffusion room shape)

  // 3. Global Reverb Type & Size
  final ReverbType reverbType;
  final double roomSize; // m3 (50 ~ 2000)
  final double stereoWidth; // % (0 ~ 100)

  // 4. Diffusion Network & Decay
  final double decayTime; // seconds (0.2 ~ 20.0)
  final double diffLowFreq; // Hz (50 ~ 2000)
  final double diffLowDecay; // Ratio (0.2 ~ 2.0)
  final double diffHighFreq; // Hz (1000 ~ 16000)
  final double diffHighDecay; // Ratio (0.2 ~ 2.0)
  final bool isFrozen;
  final bool isFreezeCut;
  final double density; // % (0 ~ 100)
  final double damp; // % (0 ~ 100)
  final double chorusRate; // Hz
  final double chorusAmount;

  // 5. Output Stage
  final double reflectGainDb; // dB (-30 ~ +6)
  final double diffuseGainDb; // dB (-30 ~ +6)
  final double dryWetPercent; // % (0 ~ 100)

  const SpatialReverbSettings({
    this.isEnabled = true,
    this.loCutEnabled = true,
    this.loCutFreq = 150.0,
    this.hiCutEnabled = true,
    this.hiCutFreq = 10000.0,
    this.preDelayMs = 25.0,
    this.spinEnabled = true,
    this.spinRate = 0.30,
    this.spinAmount = 17.5,
    this.shape = 0.50,
    this.reverbType = ReverbType.hall,
    this.roomSize = 800.0,
    this.stereoWidth = 100.0,
    this.decayTime = 3.20,
    this.diffLowFreq = 90.0,
    this.diffLowDecay = 0.75,
    this.diffHighFreq = 4500.0,
    this.diffHighDecay = 0.70,
    this.isFrozen = false,
    this.isFreezeCut = false,
    this.density = 80.0,
    this.damp = 40.0,
    this.chorusRate = 0.02,
    this.chorusAmount = 0.02,
    this.reflectGainDb = 0.0,
    this.diffuseGainDb = 0.0,
    this.dryWetPercent = 80.0,
  });

  SpatialReverbSettings copyWith({
    bool? isEnabled,
    bool? loCutEnabled,
    double? loCutFreq,
    bool? hiCutEnabled,
    double? hiCutFreq,
    double? preDelayMs,
    bool? spinEnabled,
    double? spinRate,
    double? spinAmount,
    double? shape,
    ReverbType? reverbType,
    double? roomSize,
    double? stereoWidth,
    double? decayTime,
    double? diffLowFreq,
    double? diffLowDecay,
    double? diffHighFreq,
    double? diffHighDecay,
    bool? isFrozen,
    bool? isFreezeCut,
    double? density,
    double? damp,
    double? chorusRate,
    double? chorusAmount,
    double? reflectGainDb,
    double? diffuseGainDb,
    double? dryWetPercent,
  }) {
    return SpatialReverbSettings(
      isEnabled: isEnabled ?? this.isEnabled,
      loCutEnabled: loCutEnabled ?? this.loCutEnabled,
      loCutFreq: loCutFreq ?? this.loCutFreq,
      hiCutEnabled: hiCutEnabled ?? this.hiCutEnabled,
      hiCutFreq: hiCutFreq ?? this.hiCutFreq,
      preDelayMs: preDelayMs ?? this.preDelayMs,
      spinEnabled: spinEnabled ?? this.spinEnabled,
      spinRate: spinRate ?? this.spinRate,
      spinAmount: spinAmount ?? this.spinAmount,
      shape: shape ?? this.shape,
      reverbType: reverbType ?? this.reverbType,
      roomSize: roomSize ?? this.roomSize,
      stereoWidth: stereoWidth ?? this.stereoWidth,
      decayTime: decayTime ?? this.decayTime,
      diffLowFreq: diffLowFreq ?? this.diffLowFreq,
      diffLowDecay: diffLowDecay ?? this.diffLowDecay,
      diffHighFreq: diffHighFreq ?? this.diffHighFreq,
      diffHighDecay: diffHighDecay ?? this.diffHighDecay,
      isFrozen: isFrozen ?? this.isFrozen,
      isFreezeCut: isFreezeCut ?? this.isFreezeCut,
      density: density ?? this.density,
      damp: damp ?? this.damp,
      chorusRate: chorusRate ?? this.chorusRate,
      chorusAmount: chorusAmount ?? this.chorusAmount,
      reflectGainDb: reflectGainDb ?? this.reflectGainDb,
      diffuseGainDb: diffuseGainDb ?? this.diffuseGainDb,
      dryWetPercent: dryWetPercent ?? this.dryWetPercent,
    );
  }

  Map<String, dynamic> toJson() => {
        'isEnabled': isEnabled,
        'loCutEnabled': loCutEnabled,
        'loCutFreq': loCutFreq,
        'hiCutEnabled': hiCutEnabled,
        'hiCutFreq': hiCutFreq,
        'preDelayMs': preDelayMs,
        'spinEnabled': spinEnabled,
        'spinRate': spinRate,
        'spinAmount': spinAmount,
        'shape': shape,
        'reverbType': reverbType.name,
        'roomSize': roomSize,
        'stereoWidth': stereoWidth,
        'decayTime': decayTime,
        'diffLowFreq': diffLowFreq,
        'diffLowDecay': diffLowDecay,
        'diffHighFreq': diffHighFreq,
        'diffHighDecay': diffHighDecay,
        'isFrozen': isFrozen,
        'isFreezeCut': isFreezeCut,
        'density': density,
        'damp': damp,
        'chorusRate': chorusRate,
        'chorusAmount': chorusAmount,
        'reflectGainDb': reflectGainDb,
        'diffuseGainDb': diffuseGainDb,
        'dryWetPercent': dryWetPercent,
      };

  factory SpatialReverbSettings.fromJson(Map<String, dynamic> json) {
    ReverbType parseType(dynamic name) {
      for (final t in ReverbType.values) {
        if (t.name == name) return t;
      }
      return ReverbType.hall;
    }

    double asDouble(dynamic v, double fallback) =>
        v is num ? v.toDouble() : fallback;

    const d = SpatialReverbSettings();
    return SpatialReverbSettings(
      isEnabled: json['isEnabled'] as bool? ?? d.isEnabled,
      loCutEnabled: json['loCutEnabled'] as bool? ?? d.loCutEnabled,
      loCutFreq: asDouble(json['loCutFreq'], d.loCutFreq),
      hiCutEnabled: json['hiCutEnabled'] as bool? ?? d.hiCutEnabled,
      hiCutFreq: asDouble(json['hiCutFreq'], d.hiCutFreq),
      preDelayMs: asDouble(json['preDelayMs'], d.preDelayMs),
      spinEnabled: json['spinEnabled'] as bool? ?? d.spinEnabled,
      spinRate: asDouble(json['spinRate'], d.spinRate),
      spinAmount: asDouble(json['spinAmount'], d.spinAmount),
      shape: asDouble(json['shape'], d.shape),
      reverbType: parseType(json['reverbType']),
      roomSize: asDouble(json['roomSize'], d.roomSize),
      stereoWidth: asDouble(json['stereoWidth'], d.stereoWidth),
      decayTime: asDouble(json['decayTime'], d.decayTime),
      diffLowFreq: asDouble(json['diffLowFreq'], d.diffLowFreq),
      diffLowDecay: asDouble(json['diffLowDecay'], d.diffLowDecay),
      diffHighFreq: asDouble(json['diffHighFreq'], d.diffHighFreq),
      diffHighDecay: asDouble(json['diffHighDecay'], d.diffHighDecay),
      isFrozen: json['isFrozen'] as bool? ?? d.isFrozen,
      isFreezeCut: json['isFreezeCut'] as bool? ?? d.isFreezeCut,
      density: asDouble(json['density'], d.density),
      damp: asDouble(json['damp'], d.damp),
      chorusRate: asDouble(json['chorusRate'], d.chorusRate),
      chorusAmount: asDouble(json['chorusAmount'], d.chorusAmount),
      reflectGainDb: asDouble(json['reflectGainDb'], d.reflectGainDb),
      diffuseGainDb: asDouble(json['diffuseGainDb'], d.diffuseGainDb),
      dryWetPercent: asDouble(json['dryWetPercent'], d.dryWetPercent),
    );
  }
}

class SpatialReverbState {
  final int selectedChannel; // 0 = ALL, 1 = Ch 1, 2 = Ch 2, etc.
  final Map<int, SpatialReverbSettings> channelSettings;

  const SpatialReverbState({
    this.selectedChannel = 0,
    this.channelSettings = const {0: SpatialReverbSettings()},
  });

  SpatialReverbSettings get currentSettings =>
      channelSettings[selectedChannel] ?? channelSettings[0] ?? const SpatialReverbSettings();

  SpatialReverbSettings getSettingsForChannel(int ch) =>
      channelSettings[ch] ?? channelSettings[0] ?? const SpatialReverbSettings();

  SpatialReverbState copyWith({
    int? selectedChannel,
    Map<int, SpatialReverbSettings>? channelSettings,
  }) {
    return SpatialReverbState(
      selectedChannel: selectedChannel ?? this.selectedChannel,
      channelSettings: channelSettings ?? this.channelSettings,
    );
  }
}

class SpatialReverbNotifier extends Notifier<SpatialReverbState> {
  Timer? _saveDebounceTimer;

  void _syncWithRust(int channel, SpatialReverbSettings settings) {
    try {
      rust_api.apiSetChannelSpatialReverb(
        channel: BigInt.from(channel),
        isEnabled: settings.isEnabled,
        roomSize: settings.roomSize,
        decayTime: settings.decayTime,
        preDelayMs: settings.preDelayMs,
        damp: settings.damp,
        density: settings.density,
        dryWet: settings.dryWetPercent / 100.0,
      );
    } catch (_) {}

    // 마스터 버스 리버브(mixer.reverb, 하드웨어 ch0/ch1에 적용)는 언제나
    // "ALL OUTPUTS"(채널 0) 설정을 따라야 한다.
    //
    // 예전에는 편집 중인 채널이 0일 때만 이 명령을 보냈다. 그래서 랙에서
    // 특정 채널(예: CH 1)을 선택한 채 MIX를 0으로 내리면 그 채널의 리버브만
    // 꺼지고 마스터 버스는 이전 값(기본 Hall 80%)에 그대로 남았다. 사용자
    // 입장에서는 "믹스를 0으로 했는데도 리버브가 엄청 걸려있는" 상태가 된다
    // (실기 확인: selectedChannel=1, ALL=80%, 편집한 CH1=0%).
    final allSettings = state.channelSettings[0] ?? settings;
    try {
      rust_api.apiSetSpatialReverb(
        isEnabled: allSettings.isEnabled,
        roomSize: allSettings.roomSize,
        decayTime: allSettings.decayTime,
        preDelayMs: allSettings.preDelayMs,
        damp: allSettings.damp,
        density: allSettings.density,
        dryWet: allSettings.dryWetPercent / 100.0,
      );
    } catch (_) {}
  }

  @override
  SpatialReverbState build() {
    _loadFromPrefs();
    ref.onDispose(() {
      _saveDebounceTimer?.cancel();
    });
    return const SpatialReverbState();
  }

  /// 재동기화 호출 횟수(테스트 검증용).
  int resyncCount = 0;

  /// 엔진이 (재)기동된 뒤 모든 채널의 공간 리버브 설정을 다시 보낸다.
  ///
  /// 리버브 파라미터는 예전엔 어디에도 저장되지 않아서, 엔진을 다시 켜면
  /// 믹서가 기본값(Hall, 80% 웻)으로 시작했다. 사용자가 믹스를 0으로
  /// 내려도 다음 엔진 재기동(재스캔, 워치독 복구, 앱 재시작) 때 이 함수가
  /// 그 시점의 메모리 상태를 다시 보내는데, _loadFromPrefs가 아직 끝나기
  /// 전이거나 애초에 저장이 없으면 기본값이 다시 걸렸다(실기 증상: "리버브
  /// 믹스값을 뺐는데도 엄청 걸려있는 상태"). 이제 아래 _loadFromPrefs로
  /// 값을 영속화하므로, 재기동 후에도 사용자가 마지막으로 설정한 값이
  /// 여기서 다시 나간다.
  void resyncToBackend() {
    resyncCount++;
    for (final entry in state.channelSettings.entries) {
      _syncWithRust(entry.key, entry.value);
    }
  }

  /// 프로젝트 파일을 불러온 뒤 저장소를 다시 읽는다(core/state/project_file.dart).
  Future<void> reloadFromPrefs() => _loadFromPrefs(resetWhenAbsent: true);

  Future<void> _loadFromPrefs({bool resetWhenAbsent = false}) async {
    final prefs = await SharedPreferences.getInstance();
    final jsonString = prefs.getString(_kSpatialReverbPrefsKey);
    if (jsonString == null) {
      // 프로젝트 전환으로 다시 읽는 경우에만 기본 상태로 되돌린다. 앱 시작
      // 경로에서 되돌리면 이미 화면에서 조작한 값을 지울 수 있다.
      if (resetWhenAbsent) {
        state = const SpatialReverbState();
        resyncToBackend();
      }
      return;
    }
    try {
      final decoded = jsonDecode(jsonString) as Map<String, dynamic>;
      final selectedChannel = decoded['selectedChannel'] as int? ?? 0;
      final rawSettings = decoded['channelSettings'] as Map<String, dynamic>?;
      if (rawSettings == null || rawSettings.isEmpty) return;
      final channelSettings = <int, SpatialReverbSettings>{
        for (final entry in rawSettings.entries)
          int.parse(entry.key):
              SpatialReverbSettings.fromJson(entry.value as Map<String, dynamic>),
      };
      state = state.copyWith(
        selectedChannel: selectedChannel,
        channelSettings: channelSettings,
      );
      // 로드된 값을 즉시 엔진에 반영한다. 앱 부팅 흐름의 resyncEngineState*
      // 호출이 이 async 로드보다 먼저 끝날 수 있어(경쟁), 그때는 여전히
      // 기본값이 나가므로 로드 완료 시점에 한 번 더 확실히 보낸다.
      resyncToBackend();
    } catch (_) {
      // 손상된 저장값은 기본 상태로 둔다.
    }
  }

  /// 프로젝트 파일을 저장하기 전에 미뤄둔 저장을 즉시 끝낸다
  /// (core/state/project_file.dart). 이게 없으면 방금 옮긴 스피커나 방금 돌린
  /// 노브가 파일에 빠진다.
  Future<void> flushPendingSave() {
    _saveDebounceTimer?.cancel();
    return _saveToPrefsImmediate();
  }

  void _saveToPrefsDebounced() {
    _saveDebounceTimer?.cancel();
    _saveDebounceTimer = Timer(const Duration(milliseconds: 300), () {
      _saveToPrefsImmediate();
    });
  }

  Future<void> _saveToPrefsImmediate() async {
    final prefs = await SharedPreferences.getInstance();
    final payload = {
      'selectedChannel': state.selectedChannel,
      'channelSettings': {
        for (final entry in state.channelSettings.entries)
          entry.key.toString(): entry.value.toJson(),
      },
    };
    await prefs.setString(_kSpatialReverbPrefsKey, jsonEncode(payload));
  }

  void selectChannel(int channel) {
    final updatedMap = Map<int, SpatialReverbSettings>.from(state.channelSettings);
    if (!updatedMap.containsKey(channel)) {
      // Copy from channel 0 or current default
      updatedMap[channel] = state.channelSettings[0] ?? const SpatialReverbSettings();
    }
    state = state.copyWith(selectedChannel: channel, channelSettings: updatedMap);
    _saveToPrefsDebounced();
  }

  void copyToAll() {
    final cur = state.currentSettings;
    final updatedMap = <int, SpatialReverbSettings>{0: cur};
    for (int ch = 1; ch <= 64; ch++) {
      if (state.channelSettings.containsKey(ch)) {
        updatedMap[ch] = cur;
      }
    }
    state = state.copyWith(channelSettings: updatedMap);
    _syncWithRust(0, cur);
    _saveToPrefsDebounced();
  }

  void _updateCurrentSettings(SpatialReverbSettings Function(SpatialReverbSettings current) updateFn) {
    final ch = state.selectedChannel;
    final cur = state.currentSettings;
    final updated = updateFn(cur);
    final updatedMap = Map<int, SpatialReverbSettings>.from(state.channelSettings);
    updatedMap[ch] = updated;

    if (ch == 0) {
      // Update all channels
      for (final key in updatedMap.keys.toList()) {
        updatedMap[key] = updated;
      }
    }

    state = state.copyWith(channelSettings: updatedMap);
    _syncWithRust(ch, updated);
    _saveToPrefsDebounced();
  }

  void setReverbType(ReverbType type) {
    _updateCurrentSettings((s) => s.copyWith(
      reverbType: type,
      roomSize: type.defaultSize,
      decayTime: type.defaultDecay,
      preDelayMs: type.defaultPreDelay,
      damp: type.defaultDamp,
      density: type.defaultDensity,
    ));
  }

  void toggleEnabled() => _updateCurrentSettings((s) => s.copyWith(isEnabled: !s.isEnabled));
  void toggleLoCut() => _updateCurrentSettings((s) => s.copyWith(loCutEnabled: !s.loCutEnabled));
  void toggleHiCut() => _updateCurrentSettings((s) => s.copyWith(hiCutEnabled: !s.hiCutEnabled));
  void toggleSpin() => _updateCurrentSettings((s) => s.copyWith(spinEnabled: !s.spinEnabled));
  void toggleFreeze() => _updateCurrentSettings((s) => s.copyWith(isFrozen: !s.isFrozen));

  void updateLoCutFreq(double val) => _updateCurrentSettings((s) => s.copyWith(loCutFreq: val));
  void updateHiCutFreq(double val) => _updateCurrentSettings((s) => s.copyWith(hiCutFreq: val));
  void updatePreDelay(double val) => _updateCurrentSettings((s) => s.copyWith(preDelayMs: val));
  void updateSpin(double rate, double amount) => _updateCurrentSettings((s) => s.copyWith(spinRate: rate, spinAmount: amount));
  void updateShape(double val) => _updateCurrentSettings((s) => s.copyWith(shape: val));
  void updateRoomSize(double val) => _updateCurrentSettings((s) => s.copyWith(roomSize: val));
  void updateStereoWidth(double val) => _updateCurrentSettings((s) => s.copyWith(stereoWidth: val));
  void updateDecayTime(double val) => _updateCurrentSettings((s) => s.copyWith(decayTime: val));
  void updateDiffLow(double freq, double decay) => _updateCurrentSettings((s) => s.copyWith(diffLowFreq: freq, diffLowDecay: decay));
  void updateDiffHigh(double freq, double decay) => _updateCurrentSettings((s) => s.copyWith(diffHighFreq: freq, diffHighDecay: decay));
  void updateDensity(double val) => _updateCurrentSettings((s) => s.copyWith(density: val));
  void updateDamp(double val) => _updateCurrentSettings((s) => s.copyWith(damp: val));
  void updateReflectGain(double val) => _updateCurrentSettings((s) => s.copyWith(reflectGainDb: val));
  void updateDiffuseGain(double val) => _updateCurrentSettings((s) => s.copyWith(diffuseGainDb: val));
  void updateDryWet(double val) => _updateCurrentSettings((s) => s.copyWith(dryWetPercent: val));
}

final spatialReverbProvider = NotifierProvider<SpatialReverbNotifier, SpatialReverbState>(SpatialReverbNotifier.new);
