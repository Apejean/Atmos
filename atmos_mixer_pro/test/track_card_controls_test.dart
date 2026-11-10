import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/core/utils/channel_dropdown_helper.dart';
import 'package:atmos_mixer_pro/core/utils/channel_routing.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/track_card.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// 메인화면 트랙 카드 조작부(2026-11-10 사용자 결정): Bed·3D Object 선택과 3D Trajectory 버튼을 없애고,
/// 무한루프 옆 아이콘에 이름(메모리/디스크)을 붙였다.

class _ConfigNotifier extends ConfigNotifier {
  final AppConfig initial;
  _ConfigNotifier(this.initial);

  @override
  AppConfig? build() => initial;
}

class _EngineStateNotifier extends EngineStateNotifier {
  @override
  EngineState build() => EngineState();
}

class _OutputChannelsNotifier extends OutputChannelsNotifier {
  static const channels = ['Out 1', 'Out 2', 'Out 3', 'Out 4'];

  @override
  OutputChannelsState build() => const OutputChannelsState(
    status: OutputChannelsStatus.ready,
    channelNames: channels,
    lastKnownGoodChannelNames: channels,
  );
}

ChannelSetting _setting(String name) => ChannelSetting(
  enabled: true,
  customName: name,
  delayMs: 0.0,
  gainDb: 0.0,
  phaseInvert: false,
  eqBands: const [],
);

AppConfig _config() => AppConfig(
  globalReverbMix: 0.0,
  globalReverbDecay: 1.0,
  oscPort: 8000,
  bufferSize: 512,
  themeStartOscAddress: '',
  systemResetOscAddress: '',
  monoConfigs: {1: _setting('Front'), 3: _setting('Rear')},
  stereoConfigs: {1: _setting('Main LR')},
  multiConfigs: const {},
  rooms: const [],
  isExhibitionMode: false,
  masterHeadroomDb: 0.0,
  peakLimiterEnabled: true,
  globalTrajectory: null,
  roomZones: const [],
);

TrackConfig _track({
  bool isLoop = false,
  bool isStreaming = false,
  int outputChannel = 0,
}) => TrackConfig(
  id: 't1',
  name: 'Track 1',
  filePath: '',
  volume: 1.0,
  isLoop: isLoop,
  isStreaming: isStreaming,
  outputChannel: outputChannel,
  outputStereo: false,
  playOscAddress: '',
  stopOscAddress: '',
);

Future<void> _pump(
  WidgetTester tester,
  TrackConfig track, {
  ValueChanged<bool>? onStreamChanged,
  void Function(int channel, bool isStereo)? onOutputChanged,
}) async {
  final config = _config();
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        configProvider.overrideWith(() => _ConfigNotifier(config)),
        engineStateProvider.overrideWith(() => _EngineStateNotifier()),
        outputChannelsProvider.overrideWith(() => _OutputChannelsNotifier()),
      ],
      child: MaterialApp(
        home: Scaffold(
          body: TrackCard(
            track: track,
            accentColor: Colors.blue,
            onStreamChanged: onStreamChanged,
            onOutputChanged: onOutputChanged ?? (_, _) {},
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('Bed·3D Object 선택과 3D Trajectory 버튼이 없고 Ext. Out만 보인다', (tester) async {
    await _pump(tester, _track());

    expect(find.text('BED'), findsNothing);
    expect(find.text('3D OBJECT'), findsNothing);
    expect(find.text('3D Trajectory'), findsNothing);
    expect(find.text('Ext. Out: '), findsOneWidget);
  });

  testWidgets('무한루프가 꺼진 트랙은 저장된 대로 메모리 또는 디스크로 보인다', (tester) async {
    await _pump(tester, _track(isStreaming: false));
    expect(find.text('메모리'), findsOneWidget);
    expect(find.text('디스크'), findsNothing);

    await _pump(tester, _track(isStreaming: true));
    expect(find.text('디스크'), findsOneWidget);
    expect(find.text('메모리'), findsNothing);
  });

  testWidgets('이름을 누르면 메모리와 디스크가 서로 바뀐다', (tester) async {
    final changes = <bool>[];

    await _pump(tester, _track(isStreaming: false), onStreamChanged: changes.add);
    await tester.tap(find.text('메모리'));
    await tester.pump();
    expect(changes, [true]);

    await _pump(tester, _track(isStreaming: true), onStreamChanged: changes.add);
    await tester.tap(find.text('디스크'));
    await tester.pump();
    expect(changes, [true, false]);
  });

  testWidgets('무한루프를 켠 트랙은 엔진이 항상 디스크에서 읽으므로 디스크로 보이고 바꿀 수 없다', (tester) async {
    final changes = <bool>[];

    await _pump(tester, _track(isLoop: true, isStreaming: false), onStreamChanged: changes.add);

    expect(find.text('디스크'), findsOneWidget);
    expect(find.text('메모리'), findsNothing);
    expect(find.byTooltip('무한루프 트랙은 항상 디스크에서 읽습니다'), findsOneWidget);

    await tester.tap(find.text('디스크'));
    await tester.pump();
    expect(changes, isEmpty);
  });

  testWidgets('예전에 3D OBJECT로 저장한 트랙은 첫 출력으로 옮겨진다', (tester) async {
    final moved = <int>[];
    final first = buildChannelRoutingItems(
      channelNames: _OutputChannelsNotifier.channels,
      config: _config(),
      fileChannels: null,
    ).first;

    await _pump(
      tester,
      _track(outputChannel: 4294967295),
      onOutputChanged: (channel, _) => moved.add(channel),
    );

    expect(moved, [ChannelDropdownValueHelper.getChannel(first.value)]);
  });
}
