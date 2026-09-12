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

/// 출력 채널 목록 목. 실제 FFI/CPAL 조회 없이 4채널 장치를 흉내낸다.
/// 드롭다운 항목 개수는 인식된 하드웨어 출력 채널 수를 따르므로 테스트가
/// 채널 수를 명시해야 한다(예전 64채널 폴백은 제거됐다).
class MockOutputChannelsNotifier extends OutputChannelsNotifier {
  static const channels = ['Out 1', 'Out 2', 'Out 3', 'Out 4'];

  @override
  OutputChannelsState build() => const OutputChannelsState(
    status: OutputChannelsStatus.ready,
    channelNames: channels,
    lastKnownGoodChannelNames: channels,
  );
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

    final config = AppConfig(globalReverbMix: 0.0, globalReverbDecay: 1.0, oscWhitelist: const [], 
      oscPort: 8000,
      bufferSize: 512,
      themeStartOscAddress: '',
      systemResetOscAddress: '',
      monoConfigs: {
        1: ChannelSetting(
          enabled: true,
          customName: 'Mono1',
          delayMs: 0.0,
          gainDb: 0.0,
          phaseInvert: false,
          eqBands: [],
        ),
        2: ChannelSetting(
          enabled: true,
          customName: 'Mono2',
          delayMs: 0.0,
          gainDb: 0.0,
          phaseInvert: false,
          eqBands: [],
        ),
      },
      stereoConfigs: {
        3: ChannelSetting(
          enabled: true,
          customName: 'Stereo3',
          delayMs: 0.0,
          gainDb: 0.0,
          phaseInvert: false,
          eqBands: [],
        ),
      },
      multiConfigs: {},
      rooms: [],
      isExhibitionMode: false,
      masterHeadroomDb: 0.0,
      peakLimiterEnabled: true,
      globalTrajectory: null,
      roomZones: [],
    );

    final track = const TrackConfig(
      id: 't1',
      name: 'Track 1',
      filePath: '',
      volume: 1.0,
      isLoop: false,
      isStreaming: false,
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
          // 테스트는 Ch-3/Ch-4 쌍을 선택하므로 4채널 장치를 가정한다.
          outputChannelsProvider.overrideWith(
            () => MockOutputChannelsNotifier(),
          ),
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

    // Output Config에 stereoConfigs[3]이 열려 있으므로 항목은 그 그룹에서
    // 나오고, customName('Stereo3')이 라벨 뒤에 붙는다.
    await tester.tap(find.text('Stereo (Ch-3/Ch-4) (Stereo3)').last);
    await tester.pumpAndSettle();

    expect(parsedChannel, 2); // 3 - 1 = 2
    expect(parsedIsStereo, true);
  });
}
