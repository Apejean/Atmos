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

    test('Mono 그룹 키 1개는 채널 하나만 만든다', () {
      // 이 테스트는 원래 "Mono 슬롯 하나가 L/R 페어를 연다"고 고정하고 있었다.
      // 그 전제가 틀렸다. Ableton식 Output Config에서 Mono는 모노 채널 하나를
      // 여닫는 것이고, 페어가 필요하면 Stereo 그룹을 쓴다. 페어로 만들면
      // 연속한 키를 열었을 때 같은 채널이 두 번 나오고 드롭다운 값까지 겹쳐
      // DropdownButton이 assert로 죽었다.
      final items = buildChannelRoutingItems(
        channelNames: hw(2),
        config: config(mono: {1: setting(customName: 'Center')}),
        fileChannels: 1,
      );

      expect(items.map((e) => e.value).toList(), ['mono_0']);
      expect(items[0].label, 'Mono (Ch-1) (Center)');
    });

    test('Mono 그룹 키가 하드웨어 상한을 넘으면 생성되지 않는다', () {
      final items = buildChannelRoutingItems(
        channelNames: hw(1),
        config: config(mono: {2: setting()}),
        fileChannels: 1,
      );
      expect(items.map((e) => e.value).toList(), isEmpty);
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

  group('channelDisplayName (개별 채널 이름 단일화)', () {
    // 사용자 요구사항: 같은 채널이 스피커 레이아웃, 스피커 인스펙터,
    // FX(출력 채널 Mixer)에서 모두 같은 이름으로 보여야 한다. 예전에는 화면마다
    // 'Output CH 1' / 'Channel 1 (L) • Out 1' / 'Ch-1 Out 1'로 달랐다.
    // 이 테스트가 그 규약을 고정한다.
    test('하드웨어 이름이 있으면 Ch-N (이름) 형식', () {
      expect(channelDisplayName(0, ['Out 1', 'Out 2']), 'Ch-1 (Out 1)');
      expect(channelDisplayName(1, ['Out 1', 'Out 2']), 'Ch-2 (Out 2)');
    });

    test('하드웨어 이름이 비어 있거나 공백뿐이면 Ch-N만', () {
      expect(channelDisplayName(0, ['']), 'Ch-1');
      expect(channelDisplayName(0, ['   ']), 'Ch-1');
    });

    test('입력 인덱스는 0-based, 표시는 1-based', () {
      expect(channelDisplayName(7, List.filled(8, '')), 'Ch-8');
    });

    test('범위를 벗어난 인덱스도 크래시하지 않고 번호만 보여준다', () {
      expect(channelDisplayName(5, ['Out 1', 'Out 2']), 'Ch-6');
      expect(channelDisplayName(-1, ['Out 1']), 'Ch-0');
    });

    test('저장값이 현재 장치 범위를 넘으면 조용히 0으로 바뀌지 않고 드러난다', () {
      expect(channelOutOfRangeName(63), 'Ch-64 (현재 장치에 없음)');
      expect(channelOutOfRangeName(0), 'Ch-1 (현재 장치에 없음)');
    });
  });

  group('Mono 그룹은 채널 하나만 연다 (중복/크래시 회귀)', () {
    // 사용자 실제 설정 재현: monoConfigs {1: true, 2: true}, 12채널 장치.
    // 예전에는 Mono 슬롯 하나가 L/R 페어를 연다고 보고 key-1과 key 두 채널을
    // 모두 만들어서, 연속한 키를 열면 Ch-2가 두 번 나왔다. 더 심각한 건 두
    // 항목의 값이 'mono_1'로 같아서 그 채널을 고르면 DropdownButton이
    // assert로 죽는다는 점이었다.
    AppConfig configWithMono(Map<int, bool> mono) => AppConfig(
      globalReverbMix: 0.0,
      globalReverbDecay: 1.0,
      oscWhitelist: const [],
      oscPort: 8000,
      bufferSize: 1024,
      themeStartOscAddress: '',
      systemResetOscAddress: '',
      monoConfigs: {
        for (final e in mono.entries)
          e.key: ChannelSetting(
            enabled: e.value,
            customName: '',
            delayMs: 0.0,
            gainDb: 0.0,
            phaseInvert: false,
            eqBands: const [],
          ),
      },
      stereoConfigs: const {},
      multiConfigs: const {},
      rooms: const [],
      isExhibitionMode: false,
      masterHeadroomDb: 0.0,
      peakLimiterEnabled: true,
      globalTrajectory: null,
      roomZones: const [],
    );

    test('연속한 Mono 키를 열어도 같은 채널이 두 번 나오지 않는다', () {
      final items = buildChannelRoutingItems(
        channelNames: List.generate(12, (i) => 'Out ${i + 1}'),
        config: configWithMono({1: true, 2: true}),
        fileChannels: 1,
      );

      final labels = items.map((i) => i.label).toList();
      expect(
        labels.toSet().length,
        labels.length,
        reason: '중복 라벨이 있다: $labels',
      );
    });

    test('드롭다운 값이 유일하다 (DropdownButton assert 크래시 방지)', () {
      final items = buildChannelRoutingItems(
        channelNames: List.generate(12, (i) => 'Out ${i + 1}'),
        config: configWithMono({1: true, 2: true}),
        fileChannels: 1,
      );

      final values = items.map((i) => i.value).toList();
      expect(
        values.toSet().length,
        values.length,
        reason:
            'DropdownButton은 값에 해당하는 항목이 정확히 하나여야 한다. '
            '중복 값: $values',
      );
    });

    test('Mono 키 N은 정확히 채널 N 하나에 대응한다', () {
      final items = buildChannelRoutingItems(
        channelNames: List.generate(12, (i) => 'Out ${i + 1}'),
        config: configWithMono({1: true, 2: true}),
        fileChannels: 1,
      );

      expect(items.length, 2);
      expect(items[0].realChannel0, 0);
      expect(items[0].label, 'Mono (Ch-1)');
      expect(items[1].realChannel0, 1);
      expect(items[1].label, 'Mono (Ch-2)');
    });
  });
}
