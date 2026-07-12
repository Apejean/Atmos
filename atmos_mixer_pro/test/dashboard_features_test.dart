import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/track_card.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

class MockConfigNotifier extends ConfigNotifier {
  final AppConfig? initial;
  MockConfigNotifier(this.initial);

  @override
  AppConfig? build() => initial;

  @override
  void loadConfig() {}

  @override
  void saveConfig(
    AppConfig newConfig, {
    bool forceRestart = false,
    bool skipPreload = false,
  }) {
    state = newConfig;
  }
}

class MockEngineStateNotifier extends EngineStateNotifier {
  @override
  EngineState build() => EngineState();

  @override
  Future<void> setActiveRoom(String roomId) async {}

  @override
  Future<void> clearActiveRoom() async {}

  @override
  void clearRoom(String roomId) {}

  @override
  Future<void> startTheme(String firstRoomId) async {}

  @override
  void reset() {}
}

class MockLogNotifier extends Notifier<List<String>> {
  @override
  List<String> build() => [];
}

void main() {
  testWidgets('TrackCard dropdown parses output properly', (
    WidgetTester tester,
  ) async {
    int? parsedChannel;
    bool? parsedIsStereo;

    final config = const AppConfig(
      oscPort: 8000,
      bufferSize: 512,
      themeStartOscAddress: '',
      systemResetOscAddress: '',
      monoConfigs: {
        1: ChannelSetting(enabled: true, customName: 'Mono1', delayMs: 0.0, eqBands: []),
        2: ChannelSetting(enabled: true, customName: 'Mono2', delayMs: 0.0, eqBands: []),
      },
      stereoConfigs: {3: ChannelSetting(enabled: true, customName: 'Stereo3', delayMs: 0.0, eqBands: [])},
      multiConfigs: {},
      rooms: [],
      isExhibitionMode: false,
    );

    final track = const TrackConfig(
      id: 't1',
      name: 'Track 1',
      filePath: '',
      volume: 1.0,
      isLoop: false,
      outputChannel: 0,
      outputStereo: false,
      playOscAddress: '',
      stopOscAddress: '',
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          configProvider.overrideWith(() => MockConfigNotifier(config)),
          engineStateProvider.overrideWith(() => MockEngineStateNotifier()),
        ],
        child: MaterialApp(
          home: Scaffold(
            body: TrackCard(
              track: track,
              accentColor: Colors.blue,
              onOutputChanged: (ch, isStereo) {
                parsedChannel = ch;
                parsedIsStereo = isStereo;
              },
            ),
          ),
        ),
      ),
    );

    await tester.pumpAndSettle();

    // Tap the dropdown
    await tester.tap(find.byType(DropdownButton<String>));
    await tester.pumpAndSettle();

    // The stereo option should be "Stereo 3/4 (Stereo3)"
    await tester.tap(find.text('Stereo 3/4 (Stereo3)').last);
    await tester.pumpAndSettle();

    expect(parsedChannel, 2); // Because 3 - 1 = 2
    expect(parsedIsStereo, true);
  });
}
