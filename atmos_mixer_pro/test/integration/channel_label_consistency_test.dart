import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/core/utils/channel_routing.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/track_card.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// 채널 라벨 일관성 통합 테스트 (명세 5절 검증항목 4).
///
/// 사용자 요구사항: Output Config, 환경설정 트랙 매핑, 메인화면 Ext. Out,
/// 스피커 레이아웃, FX가 **같은 채널을 같은 이름으로** 보여야 한다.
///
/// 이 테스트는 화면에 실제로 렌더링된 라벨이 공유 생성기가 만든 라벨과
/// 정확히 일치하는지 확인한다. 즉 소비 위젯이 라벨을 제 나름대로 조립하거나
/// 가공하지 않는다는 것을 기계적으로 고정한다. 예전에는 화면마다 라벨 템플릿이
/// 따로 있어서 같은 채널이 'Ch 1-2 (Out 1 / Out 2)', 'Output CH 1',
/// 'Channel 1 (L) • Out 1'처럼 다르게 보였다.

class _MockConfigNotifier extends ConfigNotifier {
  final AppConfig? initial;
  _MockConfigNotifier(this.initial);

  @override
  AppConfig? build() => initial;

  @override
  void saveConfig(
    AppConfig newConfig, {
    bool forceRestart = false,
    bool skipPreload = false,
  }) {
    state = newConfig;
  }
}

class _MockEngineStateNotifier extends EngineStateNotifier {
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

class _MockOutputChannelsNotifier extends OutputChannelsNotifier {
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
  oscWhitelist: const [],
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

const _track = TrackConfig(
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

void main() {
  testWidgets(
    '메인화면 Ext. Out에 보이는 라벨이 공유 생성기 결과와 정확히 일치한다',
    (tester) async {
      final config = _config();

      // 공유 생성기가 만드는 기대 라벨. 두 화면 모두 이 함수만 쓴다.
      // filePath가 비어 있어 파일 채널 수를 모르는 상태(null)를 그대로 전달한다.
      final expected = buildChannelRoutingItems(
        channelNames: _MockOutputChannelsNotifier.channels,
        config: config,
        fileChannels: null,
      );

      expect(
        expected,
        isNotEmpty,
        reason: '기대 항목이 비면 이 테스트는 아무것도 검증하지 못한다',
      );

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            configProvider.overrideWith(() => _MockConfigNotifier(config)),
            engineStateProvider.overrideWith(
              () => _MockEngineStateNotifier(),
            ),
            outputChannelsProvider.overrideWith(
              () => _MockOutputChannelsNotifier(),
            ),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: TrackCard(
                track: _track,
                accentColor: Colors.blue,
                onOutputChanged: (_, _) {},
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.byType(DropdownButton<String>));
      await tester.pumpAndSettle();

      for (final item in expected) {
        expect(
          find.text(item.label),
          findsWidgets,
          reason:
              '공유 생성기가 만든 라벨 "${item.label}"이 화면에 없다. '
              '위젯이 라벨을 따로 조립하고 있다는 뜻이다.',
        );
      }
    },
  );

  test('Output Config를 연 그룹만 항목이 되고, 그 라벨에 customName이 붙는다', () {
    final items = buildChannelRoutingItems(
      channelNames: _MockOutputChannelsNotifier.channels,
      config: _config(),
      fileChannels: 2,
    );

    // 열지 않은 그룹(stereoConfigs key 3 등)은 항목이 되어선 안 된다.
    expect(
      items.any((i) => i.label.contains('Ch-3/Ch-4')),
      isFalse,
      reason: 'Output Config에서 열지 않은 스테레오 쌍이 선택지로 새어나왔다',
    );

    // 연 그룹은 customName과 함께 보여야 한다(e817190에서 유실됐던 항목).
    expect(
      items.any((i) => i.label.contains('Main LR')),
      isTrue,
      reason: 'customName이 라벨에 반영되지 않았다',
    );
  });

  test('같은 입력이면 항상 같은 결과다 (호출부와 무관하게 결정적)', () {
    final config = _config();
    final a = buildChannelRoutingItems(
      channelNames: _MockOutputChannelsNotifier.channels,
      config: config,
      fileChannels: 2,
    );
    final b = buildChannelRoutingItems(
      channelNames: _MockOutputChannelsNotifier.channels,
      config: config,
      fileChannels: 2,
    );

    expect(a.length, b.length);
    for (var i = 0; i < a.length; i++) {
      expect(a[i].value, b[i].value);
      expect(a[i].label, b[i].label);
      expect(a[i].realChannel0, b[i].realChannel0);
      expect(a[i].isPartialOutput, b[i].isPartialOutput);
    }
  });

  test('개별 채널 UI와 트랙 라우팅 UI가 같은 Ch-N 번호 규약을 쓴다', () {
    // 스피커 레이아웃/인스펙터/FX는 channelDisplayName을, 트랙 라우팅은
    // buildChannelRoutingItems를 쓴다. 둘이 가리키는 채널 번호가 어긋나면
    // 사용자가 스피커에 잡은 채널과 트랙이 내보내는 채널이 달라진다.
    const names = _MockOutputChannelsNotifier.channels;

    final items = buildChannelRoutingItems(
      channelNames: names,
      config: _config(),
      fileChannels: 1,
    );

    for (final item in items) {
      final ch0 = item.realChannel0;
      // 라우팅 항목의 라벨에 들어간 채널 번호가 개별 채널 UI의 번호와 같아야 한다.
      expect(
        channelDisplayName(ch0, names),
        startsWith('Ch-${ch0 + 1}'),
        reason: '개별 채널 표기와 라우팅 항목의 채널 번호가 어긋난다',
      );
      expect(
        item.label,
        contains('Ch-${ch0 + 1}'),
        reason: '라우팅 라벨 "${item.label}"이 realChannel0과 다른 번호를 표시한다',
      );
    }
  });
}
