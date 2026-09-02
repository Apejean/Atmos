import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:flutter_riverpod/flutter_riverpod.dart';

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

    if (channel == 0) {
      try {
        rust_api.apiSetSpatialReverb(
          isEnabled: settings.isEnabled,
          roomSize: settings.roomSize,
          decayTime: settings.decayTime,
          preDelayMs: settings.preDelayMs,
          damp: settings.damp,
          density: settings.density,
          dryWet: settings.dryWetPercent / 100.0,
        );
      } catch (_) {}
    }
  }

  @override
  SpatialReverbState build() => const SpatialReverbState();

  void selectChannel(int channel) {
    final updatedMap = Map<int, SpatialReverbSettings>.from(state.channelSettings);
    if (!updatedMap.containsKey(channel)) {
      // Copy from channel 0 or current default
      updatedMap[channel] = state.channelSettings[0] ?? const SpatialReverbSettings();
    }
    state = state.copyWith(selectedChannel: channel, channelSettings: updatedMap);
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
