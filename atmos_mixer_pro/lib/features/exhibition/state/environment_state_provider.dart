import 'package:flutter_riverpod/flutter_riverpod.dart';

class EnvironmentState {
  final double temperature; // in Celsius
  final double humidity; // in percentage 0-100

  const EnvironmentState({
    this.temperature = 20.0,
    this.humidity = 50.0,
  });

  EnvironmentState copyWith({
    double? temperature,
    double? humidity,
  }) {
    return EnvironmentState(
      temperature: temperature ?? this.temperature,
      humidity: humidity ?? this.humidity,
    );
  }

  // Calculate speed of sound in m/s based on temperature
  double get speedOfSound => 331.3 + (0.606 * temperature);
}

class EnvironmentStateNotifier extends Notifier<EnvironmentState> {
  @override
  EnvironmentState build() {
    return const EnvironmentState();
  }

  void setTemperature(double temp) {
    state = state.copyWith(temperature: temp);
  }

  void setHumidity(double hum) {
    state = state.copyWith(humidity: hum);
  }
}

final environmentStateProvider =
    NotifierProvider<EnvironmentStateNotifier, EnvironmentState>(
        EnvironmentStateNotifier.new);
