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

AppConfig _config({OutputRoutingMode mode = OutputRoutingMode.trackMapping}) => AppConfig(
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
  outputRoutingMode: mode,
);

TrackConfig _track({
  bool isLoop = false,
  bool isStreaming = false,
  int outputChannel = 0,
  List<TrackRoute> routes = const [],
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
  routes: routes,
);

Future<void> _pump(
  WidgetTester tester,
  TrackConfig track, {
  ValueChanged<bool>? onStreamChanged,
  void Function(int channel, bool isStereo)? onOutputChanged,
  OutputRoutingMode mode = OutputRoutingMode.trackMapping,
}) async {
  final config = _config(mode: mode);
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
  group('출력 방식(매트릭스 라우팅 7.4)', () {
    testWidgets('매핑 방식이면 지금처럼 드롭다운만 있고 표 단추가 없다(표가 저장돼 있어도)', (tester) async {
      await _pump(tester, _track(routes: const [TrackRoute(fileChannel: 0, outputChannel: 2, gain: 1.0)]));
      expect(find.byType(DropdownButton<String>), findsOneWidget);
      expect(find.byKey(const Key('track_routes_button')), findsNothing);
    });

    testWidgets('매트릭스 방식이면 요약과 표 단추가 있고 드롭다운이 없다', (tester) async {
      await _pump(
        tester,
        _track(routes: const [
          TrackRoute(fileChannel: 0, outputChannel: 0, gain: 1.0),
          TrackRoute(fileChannel: 0, outputChannel: 2, gain: 0.5),
        ]),
        mode: OutputRoutingMode.matrix,
      );
      expect(find.byType(DropdownButton<String>), findsNothing);
      expect(find.byKey(const Key('track_routes_button')), findsOneWidget);
      expect(find.text('표 · Ch-1, 3'), findsOneWidget);
    });

    testWidgets('매트릭스 방식인데 표가 비어 있으면 매핑대로 나간다고 알린다', (tester) async {
      await _pump(tester, _track(), mode: OutputRoutingMode.matrix);
      expect(find.text('매핑대로 출력 중 — 표 열기'), findsOneWidget);
    });
  });

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

  // 2026-11-21 사용자 결정: 저장된 출력이 켠 그룹 목록에 없어도 다른 출력으로 몰래 바꾸지 않는다(그 트랙은 엔진이
  // 무음으로 둔다). 예전에는 첫 항목으로 바꿔, 그룹을 켜고 끄기만 해도 트랙 출력이 바뀌었다.
  testWidgets('저장된 출력이 켠 그룹에 없으면 바꾸지 않고 주황으로 알린다', (tester) async {
    final moved = <int>[];

    // 열린 채널은 CH1·CH2·CH3(모노 1·3, 스테레오 1/2)이다. 모노 CH4(3번)는 열려 있지 않다.
    await _pump(
      tester,
      _track(outputChannel: 3),
      onOutputChanged: (channel, _) => moved.add(channel),
    );

    expect(moved, isEmpty, reason: '저장된 출력을 첫 항목으로 바꿨다');
    expect(find.text('CH4 (출력 설정에 없음)'), findsOneWidget);
  });
}
