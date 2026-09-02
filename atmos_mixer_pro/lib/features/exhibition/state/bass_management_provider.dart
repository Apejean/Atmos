import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;

class BassManagementState {
  final bool isEnabled;
  final int? lfeChannel; // which channel is the subwoofer
  final double crossoverFreq;

  BassManagementState({
    required this.isEnabled,
    this.lfeChannel,
    required this.crossoverFreq,
  });

  BassManagementState copyWith({
    bool? isEnabled,
    int? lfeChannel,
    double? crossoverFreq,
    bool clearLfe = false,
  }) {
    return BassManagementState(
      isEnabled: isEnabled ?? this.isEnabled,
      lfeChannel: clearLfe ? null : (lfeChannel ?? this.lfeChannel),
      crossoverFreq: crossoverFreq ?? this.crossoverFreq,
    );
  }
}

class BassManagementNotifier extends Notifier<BassManagementState> {
  @override
  BassManagementState build() {
    return BassManagementState(isEnabled: false, crossoverFreq: 80.0);
  }

  void setEnabled(bool enabled) {
    state = state.copyWith(isEnabled: enabled);
    rust_api.apiSetBassManagementEnabled(enabled: enabled);
  }

  void setLfeChannel(int? channel) {
    bool shouldEnable = channel != null ? true : state.isEnabled;
    state = state.copyWith(lfeChannel: channel, clearLfe: channel == null, isEnabled: shouldEnable);
    rust_api.apiSetLfeChannel(channel: channel != null ? BigInt.from(channel) : null);
    if (channel != null) {
      rust_api.apiSetBassManagementEnabled(enabled: true);
    }
  }

  void setCrossoverFreq(double freq) {
    state = state.copyWith(crossoverFreq: freq);
    rust_api.apiSetCrossoverFrequency(freq: freq.toDouble());
  }
}

final bassManagementProvider =
    NotifierProvider<BassManagementNotifier, BassManagementState>(
        BassManagementNotifier.new);
