import 'package:flutter_riverpod/flutter_riverpod.dart';

class OutputChannelModel {
  final int id; // 1 to 128
  final String name;
  final bool isMuted;
  final bool isSoloed;
  final bool isPhaseInverted;
  final double delayMs;
  final double gainDb;

  OutputChannelModel({
    required this.id,
    required this.name,
    this.isMuted = false,
    this.isSoloed = false,
    this.isPhaseInverted = false,
    this.delayMs = 0.0,
    this.gainDb = 0.0,
  });

  OutputChannelModel copyWith({
    int? id,
    String? name,
    bool? isMuted,
    bool? isSoloed,
    bool? isPhaseInverted,
    double? delayMs,
    double? gainDb,
  }) {
    return OutputChannelModel(
      id: id ?? this.id,
      name: name ?? this.name,
      isMuted: isMuted ?? this.isMuted,
      isSoloed: isSoloed ?? this.isSoloed,
      isPhaseInverted: isPhaseInverted ?? this.isPhaseInverted,
      delayMs: delayMs ?? this.delayMs,
      gainDb: gainDb ?? this.gainDb,
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
      );
    });
  }

  void updateChannel(OutputChannelModel updated) {
    state = state.map((ch) => ch.id == updated.id ? updated : ch).toList();
  }
}

final outputRoutingProvider = NotifierProvider<OutputRoutingNotifier, List<OutputChannelModel>>(() {
  return OutputRoutingNotifier();
});
