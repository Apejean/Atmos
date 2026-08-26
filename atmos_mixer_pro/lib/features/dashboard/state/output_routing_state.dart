import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/src/rust/common/config.dart' as rust_config;

class OutputChannelModel {
  final int id; // 1 to 128
  final String name;
  final bool isMuted;
  final bool isSoloed;
  final bool isPhaseInverted;
  final double delayMs;
  final double gainDb;
  final List<rust_config.EqBand> eqBands;

  OutputChannelModel({
    required this.id,
    required this.name,
    this.isMuted = false,
    this.isSoloed = false,
    this.isPhaseInverted = false,
    this.delayMs = 0.0,
    this.gainDb = 0.0,
    this.eqBands = const [],
  });

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'name': name,
      'isMuted': isMuted,
      'isSoloed': isSoloed,
      'isPhaseInverted': isPhaseInverted,
      'delayMs': delayMs,
      'gainDb': gainDb,
      'eqBands': eqBands.map((e) => {
        'enabled': e.enabled,
        'freq': e.freq,
        'gain': e.gain,
        'qFactor': e.qFactor,
        'filterType': e.filterType.name,
      }).toList(),
    };
  }

  OutputChannelModel copyWith({
    int? id,
    String? name,
    bool? isMuted,
    bool? isSoloed,
    bool? isPhaseInverted,
    double? delayMs,
    double? gainDb,
    List<rust_config.EqBand>? eqBands,
  }) {
    return OutputChannelModel(
      id: id ?? this.id,
      name: name ?? this.name,
      isMuted: isMuted ?? this.isMuted,
      isSoloed: isSoloed ?? this.isSoloed,
      isPhaseInverted: isPhaseInverted ?? this.isPhaseInverted,
      delayMs: delayMs ?? this.delayMs,
      gainDb: gainDb ?? this.gainDb,
      eqBands: eqBands ?? this.eqBands,
    );
  }
}

class OutputRoutingNotifier extends Notifier<List<OutputChannelModel>> {
  @override
  List<OutputChannelModel> build() {
    return List.generate(128, (index) {
      return OutputChannelModel(
        id: index + 1,
        name: 'Speaker ${index + 1}',
        eqBands: List.generate(8, (i) => rust_config.EqBand(
          enabled: false,
          freq: 1000.0,
          gain: 0.0,
          qFactor: 0.707,
          filterType: rust_config.EqType.bell,
        )),
      );
    });
  }

  void updateChannel(OutputChannelModel updated) {
    state = state.map((ch) => ch.id == updated.id ? updated : ch).toList();
    _syncToBackend();
  }
  
  void importCalibration(List<OutputChannelModel> imported) {
    state = imported;
    _syncToBackend();
  }

  void _syncToBackend() {
    final payload = {
      'channels': state.map((e) => e.toJson()).toList(),
    };
    final jsonString = jsonEncode(payload);
    rust_api.apiUpdateOutputRouting(jsonPayload: jsonString).catchError((e) {
      debugPrint('Failed to sync output routing: $e');
    });
  }
}

final outputRoutingProvider = NotifierProvider<OutputRoutingNotifier, List<OutputChannelModel>>(() {
  return OutputRoutingNotifier();
});
