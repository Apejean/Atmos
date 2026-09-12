import 'package:flutter_test/flutter_test.dart';
import 'package:atmos_mixer_pro/core/utils/channel_routing.dart';
import 'package:atmos_mixer_pro/core/utils/channel_dropdown_helper.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// [buildChannelRoutingItems] 단위 테스트.
///
/// 핵심 회귀 방지 대상: 4채널 파일을 2채널 장치에서 재생할 때 환경설정
/// 트랙 매핑과 메인화면 Ext. Out이 서로 다른 개수/라벨의 항목을 만들던 버그
/// (`N-Ch (다채널) Ch 1~2`/`Ch 2~2` vs `N-Ch (Ch-1~4)`). 이 함수 하나로
/// 수렴한 뒤에는 두 화면이 항상 동일한 결과를 받는다.
void main() {
  ChannelSetting setting({bool enabled = true, String customName = ''}) =>
      ChannelSetting(
        enabled: enabled,
        customName: customName,
        delayMs: 0.0,
        eqBands: const [],
        phaseInvert: false,
        gainDb: 0.0,
      );

  AppConfig config({
    Map<int, ChannelSetting> mono = const {},
    Map<int, ChannelSetting> stereo = const {},
    Map<int, ChannelSetting> multi = const {},
  }) => AppConfig(
    oscPort: 9000,
    bufferSize: 512,
    themeStartOscAddress: '',
    systemResetOscAddress: '',
    monoConfigs: mono,
    stereoConfigs: stereo,
    multiConfigs: multi,
    rooms: const [],
    roomZones: const [],
    isExhibitionMode: false,
    masterHeadroomDb: 0.0,
    peakLimiterEnabled: true,
    oscWhitelist: const [],
    globalReverbMix: 0.0,
    globalReverbDecay: 1.0,
  );

  List<String> hw(int n) =>
      List.generate(n, (i) => 'Ch ${i + 1}');

  group('Output Config가 비어있을 때(하위호환 자동 생성)', () {
    test('모노 파일: 하드웨어 채널마다 Mono 항목 1개씩만 생성된다', () {
      final items = buildChannelRoutingItems(
        channelNames: hw(4),
        config: config(),
        fileChannels: 1,
      );

      expect(items.map((e) => e.value).toList(), [
        'mono_0',
        'mono_1',
        'mono_2',
        'mono_3',
      ]);
      expect(items.every((e) => !e.isPartialOutput), isTrue);
      expect(items.first.label, 'Mono (Ch-1)');
    });

    test('스테레오 파일: Mono + 인접 쌍 Stereo 항목이 함께 생성된다', () {
      final items = buildChannelRoutingItems(
        channelNames: hw(3),
        config: config(),
        fileChannels: 2,
      );

      final monoValues = items
          .where((e) => ChannelDropdownValueHelper.isMono(e.value))
          .map((e) => e.value)
          .toList();
      final stereoValues = items
          .where((e) => ChannelDropdownValueHelper.isStereo(e.value))
          .toList();

      expect(monoValues, ['mono_0', 'mono_1', 'mono_2']);
      // 인접 쌍만: (0,1),(1,2) => 2개
      expect(stereoValues.map((e) => e.value).toList(), [
        'stereo_0',
        'stereo_1',
      ]);
      expect(stereoValues.first.label, 'Stereo (Ch-1/Ch-2)');
    });

    test('메타데이터 로딩 전(fileChannels=null)에는 스테레오 파일과 동일하게 취급된다', () {
      final items = buildChannelRoutingItems(
        channelNames: hw(2),
        config: config(),
        fileChannels: null,
      );

      expect(items.map((e) => e.value).toSet(), {
        'mono_0',
        'mono_1',
        'stereo_0',
      });
    });

    test('멀티채널 파일이 하드웨어 안에 다 들어가면 모든 시작 위치에 항목이 생기고 부분 출력이 아니다', () {
      final items = buildChannelRoutingItems(
        channelNames: hw(6),
        config: config(),
        fileChannels: 4,
      );

      // lastStart = 6 - 4 = 2 => i = 0,1,2
      expect(items.map((e) => e.value).toList(), [
        'multi_0',
        'multi_1',
        'multi_2',
      ]);
      expect(items.every((e) => !e.isPartialOutput), isTrue);
      expect(items[0].label, 'N-Ch (Ch-1~4)');
    });

    test('4채널 파일 + 2채널 장치(들어갈 자리 없음): Ch-1 시작 항목 1개가 부분 출력으로 표시된다', () {
      final items = buildChannelRoutingItems(
        channelNames: hw(2),
        config: config(),
        fileChannels: 4,
      );

      expect(items.length, 1);
      expect(items.first.value, 'multi_0');
      expect(items.first.realChannel0, 0);
      expect(items.first.isPartialOutput, isTrue);
      expect(items.first.label, 'N-Ch (Ch-1~2) · 부분 출력 2/4ch (나머지 무시됨)');
    });

    test('하드웨어 채널이 하나도 없으면 항목이 생성되지 않는다', () {
      final items = buildChannelRoutingItems(
        channelNames: const [],
        config: config(),
        fileChannels: 2,
      );
      expect(items, isEmpty);
    });

    test('모든 그룹이 비활성(enabled=false)이어도 비어있는 것과 동일하게 자동 생성으로 폴백한다', () {
      final items = buildChannelRoutingItems(
        channelNames: hw(2),
        config: config(mono: {1: setting(enabled: false)}),
        fileChannels: 1,
      );
      expect(items.map((e) => e.value).toList(), ['mono_0', 'mono_1']);
    });
  });

  group('Output Config가 지배할 때(그룹이 하나라도 열려있음)', () {
    test(
      '재현 케이스: 4채널 파일 / 2채널 장치 / multiConfigs {1,2} -> 두 항목이 서로 다른 부분출력 라벨로 구별된다',
      () {
        final items = buildChannelRoutingItems(
          channelNames: hw(2),
          config: config(
            multi: {1: setting(), 2: setting()},
          ),
          fileChannels: 4,
        );

        expect(items.length, 2);

        expect(items[0].value, 'multi_0');
        expect(items[0].realChannel0, 0);
        expect(items[0].isPartialOutput, isTrue);
        expect(
          items[0].label,
          'N-Ch (Ch-1~2) · 부분 출력 2/4ch (나머지 무시됨)',
        );

        expect(items[1].value, 'multi_1');
        expect(items[1].realChannel0, 1);
        expect(items[1].isPartialOutput, isTrue);
        expect(
          items[1].label,
          'N-Ch (Ch-2~2) · 부분 출력 1/4ch (나머지 무시됨)',
        );
      },
    );

    test('Mono 그룹 키 1개는 L/R 두 항목(Ch-1, Ch-2)을 만든다', () {
      final items = buildChannelRoutingItems(
        channelNames: hw(2),
        config: config(mono: {1: setting(customName: 'Center')}),
        fileChannels: 1,
      );

      expect(items.map((e) => e.value).toList(), ['mono_0', 'mono_1']);
      expect(items[0].label, 'Mono (Ch-1) (Center L)');
      expect(items[1].label, 'Mono (Ch-2) (Center R)');
    });

    test('Mono 그룹의 R 채널이 하드웨어 상한을 넘으면 L만 생성된다', () {
      final items = buildChannelRoutingItems(
        channelNames: hw(1),
        config: config(mono: {1: setting()}),
        fileChannels: 1,
      );
      expect(items.map((e) => e.value).toList(), ['mono_0']);
    });

    test('Stereo 그룹은 짝 채널이 있어야만 생성된다', () {
      final items = buildChannelRoutingItems(
        channelNames: hw(2),
        config: config(stereo: {1: setting(), 2: setting()}),
        fileChannels: 2,
      );

      // key=1 -> realCh=0, realCh+1=1 < 2 => 생성됨
      // key=2 -> realCh=1, realCh+1=2 >= 2(hwCount) => 생성 안 됨
      final stereoValues = items
          .where((e) => ChannelDropdownValueHelper.isStereo(e.value))
          .map((e) => e.value)
          .toList();
      expect(stereoValues, ['stereo_0']);
    });

    test('하드웨어 채널 수를 넘는 Multi 그룹 키는 생성되지 않는다', () {
      final items = buildChannelRoutingItems(
        channelNames: hw(2),
        config: config(multi: {5: setting()}),
        fileChannels: 4,
      );
      expect(items, isEmpty);
    });

    test('한 그룹이라도 설정되면 다른 그룹은 자동 생성 없이 완전히 비활성화된다', () {
      // stereoConfigs만 설정됨 -> 모노 파일이어도 monoConfigs가 비어있으므로
      // Mono 항목은 생성되지 않는다(자동 생성 폴백은 3개 맵이 전부 비어있을
      // 때만 적용됨).
      final items = buildChannelRoutingItems(
        channelNames: hw(4),
        config: config(stereo: {1: setting()}),
        fileChannels: 1,
      );
      expect(items, isEmpty);
    });
  });

  group('결정성(Determinism)', () {
    test('동일 입력으로 두 번 호출하면 항상 동일한 결과를 반환한다', () {
      final cfg = config(
        multi: {1: setting(customName: 'Front'), 2: setting()},
      );
      final channelNames = hw(2);

      List<Map<String, Object?>> asMap(List<ChannelRoutingItem> items) =>
          items
              .map(
                (e) => {
                  'value': e.value,
                  'label': e.label,
                  'realChannel0': e.realChannel0,
                  'isPartialOutput': e.isPartialOutput,
                },
              )
              .toList();

      final first = buildChannelRoutingItems(
        channelNames: channelNames,
        config: cfg,
        fileChannels: 4,
      );
      final second = buildChannelRoutingItems(
        channelNames: channelNames,
        config: cfg,
        fileChannels: 4,
      );

      expect(asMap(first), asMap(second));
    });
  });
}
