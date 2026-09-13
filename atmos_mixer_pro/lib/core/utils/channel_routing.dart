import 'package:atmos_mixer_pro/core/utils/channel_dropdown_helper.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';

/// [buildChannelRoutingItems]가 만들어내는 라우팅 드롭다운 항목 1개.
///
/// 환경설정 트랙 매핑(`preferences_modal.dart`)과 메인화면 Ext. Out
/// (`track_card.dart`)이 이 값 하나만 공유하도록 만들어 같은 저장값이 두
/// 화면에서 같은 라벨로 보이는 것을 보장한다.
class ChannelRoutingItem {
  /// [ChannelDropdownValueHelper] 인코딩 값 (예: `mono_0`, `stereo_0`, `multi_0`).
  final String value;

  /// 드롭다운에 표시할 라벨.
  final String label;

  /// 0-based 하드웨어 채널 인덱스 (Multi/Stereo는 시작 채널).
  final int realChannel0;

  /// 파일 채널 수가 하드웨어 채널 수보다 많아 일부 채널이 버려지는 경우 true.
  /// (예: 4채널 파일을 2채널 장치로 보내면 ch3, ch4는 버려진다 — 이는 정상
  /// 동작이며 무음이 아니다. 라벨에 이를 명시해 사용자가 오해하지 않게 한다.)
  final bool isPartialOutput;

  const ChannelRoutingItem({
    required this.value,
    required this.label,
    required this.realChannel0,
    required this.isPartialOutput,
  });
}

/// 드라이버가 보고하지만 **물리적으로 연결할 수 없는** 채널을 걸러내기 위한
/// 판별 규칙. 플랫폼 독립 순수 함수다.
///
/// 인터페이스는 물리 출력보다 많은 채널을 보고하는 경우가 흔하다. 예를 들어
/// Scarlett 6i6은 물리 출력이 6개(Mon 1~2, Line 3~4, S/PDIF L/R)인데
/// CoreAudio에는 드라이버 내부 리턴 6개(DAW 7~12)를 더해 12채널로 보고한다.
/// 그 채널에 스피커를 잡으면 아무 데로도 소리가 나가지 않는다.
///
/// ## 왜 이름으로 판별하는가
/// macOS(CoreAudio)와 Windows(ASIO/WASAPI)에 걸쳐 이식 가능한 공통 신호는
/// 채널 이름뿐이다. CoreAudio의 스트림 terminal type 같은 건 Windows에 대응물이
/// 없어서 플랫폼마다 동작이 갈린다. 규칙을 한 곳에 두어 어느 OS에서든 같은
/// 이름에 같은 판정이 나오게 한다.
///
/// ## 보수적으로, 실패하면 열어둔다
/// 판별은 명백히 가상인 토큰만 본다. 모르는 이름은 **물리로 취급한다**.
/// 실재하는 출력을 숨기는 쪽이 가상 채널을 하나 더 보여주는 쪽보다 훨씬
/// 나쁘기 때문이다(스피커를 연결했는데 목록에 없으면 원인을 찾기 어렵다).
bool isPhysicalOutputChannel(String name) {
  final n = name.trim().toLowerCase();
  if (n.isEmpty) return true; // 이름이 없으면 판단 근거가 없다 -> 물리로 본다

  // 벤더 중립적으로 "드라이버 내부 채널"임이 분명한 토큰만 본다.
  // 'return'이나 'mix'는 물리 단자 이름으로도 흔히 쓰여서 넣지 않는다.
  const virtualTokens = ['daw', 'loopback', 'virtual'];
  for (final t in virtualTokens) {
    if (n.contains(t)) return false;
  }
  return true;
}

/// [channelNames] 중 물리적으로 연결 가능한 채널의 **0-based 하드웨어 인덱스**.
///
/// 목록을 걸러내면서 인덱스를 다시 매기면 안 된다. 'Line 3'은 언제나 하드웨어
/// 인덱스 2이고, 그 값이 그대로 라우팅과 스피커 매핑에 쓰인다. 그래서 이름
/// 배열을 압축하지 않고 살아남은 인덱스만 돌려준다.
List<int> physicalOutputChannelIndices(List<String> channelNames) {
  final out = <int>[];
  for (var i = 0; i < channelNames.length; i++) {
    if (isPhysicalOutputChannel(channelNames[i])) out.add(i);
  }
  return out;
}

/// 개별 출력 채널 1개의 표시 이름. **스피커 레이아웃, 스피커 인스펙터,
/// FX(출력 채널 Mixer)가 모두 이 함수 하나만 쓴다.**
///
/// 사용자 요구사항이 "같은 채널이 모든 UI에서 같은 이름으로 보여야 한다"이므로
/// 라벨 템플릿을 각 위젯에 복제하지 않는다. 예전에는 화면마다
/// `Output CH 1`, `Channel 1 (L) • Out 1`, `Ch-1 Out 1`로 달라서 같은 채널이
/// 세 가지 이름을 가졌다.
///
/// [index0]은 0-based 하드웨어 채널 인덱스이고 표시는 1-based(`Ch-1`)다.
/// [channelNames]에 해당 인덱스의 이름이 있으면 괄호로 덧붙인다.
///
/// 트랙 라우팅(Mono/Stereo/N-Ch 그룹)은 이 함수가 아니라
/// [buildChannelRoutingItems]를 쓴다. 스피커와 FX는 그룹핑 없이 개별 채널만
/// 지정하기 때문이다.
String channelDisplayName(int index0, List<String> channelNames) {
  final label = 'Ch-${index0 + 1}';
  if (index0 < 0 || index0 >= channelNames.length) return label;
  final hwName = channelNames[index0].trim();
  return hwName.isEmpty ? label : '$label ($hwName)';
}

/// 저장된 채널 인덱스가 현재 장치의 채널 수를 넘었을 때의 표시 이름.
/// 값을 조용히 0으로 되돌리지 않고 사용자에게 드러내기 위한 라벨이다
/// (더 작은 인터페이스로 교체한 경우 등).
String channelOutOfRangeName(int index0) =>
    'Ch-${index0 + 1} (현재 장치에 없음)';

/// `channelNames`(실제 하드웨어 출력 채널 목록)와 Output Config
/// (`monoConfigs`/`stereoConfigs`/`multiConfigs`)를 입력받아 라우팅 드롭다운
/// 항목을 생성하는 순수 함수. 부작용이 없으며 동일 입력에는 항상 동일 결과를
/// 반환한다.
///
/// ## 0/1-based 규약 (Dart 측 유일한 1→0 변환 지점)
/// `config.json`의 `mono_configs`/`stereo_configs`/`multi_configs` 맵의
/// **키(key)만 1-based**(레거시 설계, 변경하지 않음)이고, 그 외 전부
/// 0-based다: [ChannelRoutingItem.realChannel0], [ChannelDropdownValueHelper]
/// 인코딩 값, `channelNames`의 배열 인덱스, `TrackConfig.outputChannel`,
/// `SpeakerNode.channel` 전부 0-based hw 채널을 가리킨다. 이 함수 내부에서
/// `*Configs` 맵의 1-based 키를 읽어 0-based로 변환하는 것 외에는 Dart 코드
/// 어디에서도 1-based 값을 만들거나 소비하지 않는다.
///
/// ## Output Config 수렴 규칙
/// - `monoConfigs`/`stereoConfigs`/`multiConfigs` 중 하나라도 `enabled` 항목이
///   있으면(=사용자가 Output Config에서 명시적으로 그룹을 열었으면), 이후로는
///   그 그룹만 후보가 된다(Output Config가 지배).
/// - 셋 다 `enabled` 항목이 하나도 없으면(신규 설치 등) 하위호환을 위해 모든
///   하드웨어 채널에 대해 자동으로 Mono/Stereo/Multi 항목을 생성한다.
///
/// ## 부분 재생 규칙
/// Multi 항목은 파일 채널 수가 하드웨어 채널 수를 넘어서면 실제로 열리는
/// 채널까지만 라벨에 표시하고 `· 부분 출력 X/Yc h (나머지 무시됨)`을 덧붙인다
/// (예: 4채널 파일 + 2채널 장치 -> ch1,2만 나가고 ch3,4는 버려짐). 이는 정상
/// 동작이며 무음이 아니므로 라벨로 명시한다.
List<ChannelRoutingItem> buildChannelRoutingItems({
  required List<String> channelNames,
  required AppConfig config,
  required int? fileChannels,
}) {
  // 하드웨어 채널이 하나도 없으면(장치 인식 실패/미연결) 어떤 항목도
  // 만들 수 없다. 이 경우 소비 위젯은 outputChannelsProvider의 상태
  // (noDevice/error)를 보고 별도의 안내를 표시해야 한다.
  if (channelNames.isEmpty) return const [];

  // 드라이버 내부 가상 채널(DAW 리턴 등)은 물리적으로 연결할 수 없으므로
  // 라우팅 후보에서 뺀다. 인덱스는 원래 하드웨어 인덱스를 그대로 유지한다.
  final physical = physicalOutputChannelIndices(channelNames).toSet();
  if (physical.isEmpty) return const [];

  final int hwCount = channelNames.length;
  final bool isMulti = fileChannels != null && fileChannels > 2;
  final bool isMono = fileChannels == 1;

  final bool hasConfiguredGroups =
      config.monoConfigs.values.any((s) => s.enabled) ||
      config.stereoConfigs.values.any((s) => s.enabled) ||
      config.multiConfigs.values.any((s) => s.enabled);

  final items = <ChannelRoutingItem>[];

  // 멀티채널 파일이 아니면(모노/스테레오 파일) 각 하드웨어 채널을 모노
  // 목적지로 제공한다.
  if (!isMulti) {
    if (hasConfiguredGroups) {
      final sortedMono = config.monoConfigs.entries
          .where((e) => e.value.enabled)
          .toList()
        ..sort((a, b) => a.key.compareTo(b.key));
      // Mono 그룹 하나는 **채널 하나**를 연다. 예전에는 한 슬롯이 L/R 페어를
      // 연다고 보고 key-1과 key 두 채널을 모두 항목으로 만들었다. 그러면 연속한
      // 키를 열었을 때(예: Mono 1, Mono 2) Ch-2가 두 번 나온다. 게다가 두 항목의
      // 드롭다운 값이 'mono_1'로 완전히 같아서, 그 채널을 선택하면
      // DropdownButton이 "값에 해당하는 항목은 정확히 하나여야 한다"는 assert로
      // 죽는다(잠복 크래시). Ableton식 Output Config에서 Mono는 모노 채널 하나를
      // 여닫는 것이므로 1:1이 맞다. 페어가 필요하면 Stereo 그룹을 쓴다.
      for (final e in sortedMono) {
        final key = e.key; // 1-based (레거시)
        final setting = e.value;

        final realCh = key - 1; // 0-based 변환
        if (realCh < hwCount && physical.contains(realCh)) {
          items.add(
            ChannelRoutingItem(
              value: ChannelDropdownValueHelper.getMonoValue(realCh),
              label: _withCustomSuffix(
                'Mono (Ch-${realCh + 1})',
                setting.customName,
              ),
              realChannel0: realCh,
              isPartialOutput: false,
            ),
          );
        }
      }
    } else {
      for (int i = 0; i < hwCount; i++) {
        if (!physical.contains(i)) continue;
        items.add(
          ChannelRoutingItem(
            value: ChannelDropdownValueHelper.getMonoValue(i),
            label: 'Mono (Ch-${i + 1})',
            realChannel0: i,
            isPartialOutput: false,
          ),
        );
      }
    }
  }

  // 스테레오 파일은 위 모노 선택지에 더해 인접 채널 쌍의 스테레오 선택지도
  // 제공한다.
  if (!isMono && !isMulti) {
    if (hasConfiguredGroups) {
      final sortedStereo = config.stereoConfigs.entries
          .where((e) => e.value.enabled)
          .toList()
        ..sort((a, b) => a.key.compareTo(b.key));
      for (final e in sortedStereo) {
        final key = e.key; // 1-based
        final setting = e.value;
        final realCh = key - 1; // 0-based
        // 짝 채널(realCh+1, 0-based)이 하드웨어에 없거나 가상 채널이면
        // 만들지 않는다. 스테레오 쌍은 두 채널 모두 물리여야 한다.
        if (realCh + 1 < hwCount &&
            physical.contains(realCh) &&
            physical.contains(realCh + 1)) {
          items.add(
            ChannelRoutingItem(
              value: ChannelDropdownValueHelper.getStereoValue(realCh),
              label: _withCustomSuffix(
                'Stereo (Ch-$key/Ch-${key + 1})',
                setting.customName,
              ),
              realChannel0: realCh,
              isPartialOutput: false,
            ),
          );
        }
      }
    } else {
      for (int i = 0; i + 1 < hwCount; i++) {
        if (!physical.contains(i) || !physical.contains(i + 1)) continue;
        items.add(
          ChannelRoutingItem(
            value: ChannelDropdownValueHelper.getStereoValue(i),
            label: 'Stereo (Ch-${i + 1}/Ch-${i + 2})',
            realChannel0: i,
            isPartialOutput: false,
          ),
        );
      }
    }
  }

  // 멀티채널 파일은 파일 채널 수가 시작 위치부터 들어갈 수 있는 후보만
  // 만든다. 하드웨어가 부족하면(부분 재생) 실제로 열리는 범위만 라벨에
  // 표시하고 부분 출력임을 명시한다.
  if (isMulti) {
    final fileCh = fileChannels; // isMulti == true 이므로 항상 non-null

    if (hasConfiguredGroups) {
      final sortedMulti = config.multiConfigs.entries
          .where((e) => e.value.enabled)
          .toList()
        ..sort((a, b) => a.key.compareTo(b.key));
      for (final e in sortedMulti) {
        final key = e.key; // 1-based
        final setting = e.value;
        final startCh = key - 1; // 0-based
        // 시작 채널이 하드웨어 상한을 넘거나 가상 채널이면 만들지 않는다.
        if (startCh < hwCount && physical.contains(startCh)) {
          items.add(
            _buildMultiItem(
              startCh: startCh,
              fileCh: fileCh,
              hwCount: hwCount,
              physical: physical,
              customName: setting.customName,
            ),
          );
        }
      }
    } else {
      final lastStart = hwCount - fileCh;
      if (lastStart >= 0) {
        for (int i = 0; i <= lastStart; i++) {
          if (!physical.contains(i)) continue;
          items.add(_buildMultiItem(
            startCh: i, fileCh: fileCh, hwCount: hwCount, physical: physical));
        }
      }
      if (items.isEmpty) {
        // 파일 채널 수가 물리 채널 수보다 많아 완전히 들어갈 자리가 없어도,
        // 트랙이 선택 불가가 되지 않도록 첫 물리 채널에서 시작하는 항목
        // 하나는 남긴다(부분 출력으로 표시).
        final first = physical.reduce((a, b) => a < b ? a : b);
        items.add(_buildMultiItem(
          startCh: first, fileCh: fileCh, hwCount: hwCount, physical: physical));
      }
    }
  }

  return items;
}

/// Multi 항목 1개를 만든다. 요청한 범위(`startCh`~`startCh+fileCh-1`)가
/// 실제로 열리는 범위를 넘으면 열리는 데까지만 라벨에 담고
/// `isPartialOutput`을 true로 표시한다.
///
/// 열리는 범위는 하드웨어 상한뿐 아니라 **물리 채널의 연속 구간**으로도
/// 제한된다. 예를 들어 물리 출력이 Ch-1~6이고 Ch-7 이상이 드라이버 내부
/// 가상 채널이면, Ch-5에서 시작하는 4채널 파일은 Ch-5~6까지만 실제로 나간다.
ChannelRoutingItem _buildMultiItem({
  required int startCh,
  required int fileCh,
  required int hwCount,
  required Set<int> physical,
  String customName = '',
}) {
  final requestedEnd = startCh + fileCh - 1;

  // startCh에서 시작하는 연속 물리 구간의 마지막 인덱스.
  var contiguousEnd = startCh;
  while (contiguousEnd + 1 < hwCount && physical.contains(contiguousEnd + 1)) {
    contiguousEnd++;
  }
  final availableEnd = contiguousEnd;
  final actualEnd = requestedEnd < availableEnd ? requestedEnd : availableEnd;
  final isPartial = requestedEnd > availableEnd;

  var label = 'N-Ch (Ch-${startCh + 1}~${actualEnd + 1})';
  if (customName.isNotEmpty) {
    label += ' ($customName)';
  }
  if (isPartial) {
    final actualCount = actualEnd - startCh + 1;
    label += ' · 부분 출력 $actualCount/${fileCh}ch (나머지 무시됨)';
  }

  return ChannelRoutingItem(
    value: ChannelDropdownValueHelper.getMultiValue(startCh),
    label: label,
    realChannel0: startCh,
    isPartialOutput: isPartial,
  );
}

/// `customName`이 있으면 라벨 뒤에 `(name)` 또는(페어일 때) `(name L)`/`(name R)`을
/// 붙인다.
String _withCustomSuffix(String base, String customName, {String? pairSuffix}) {
  if (customName.isEmpty) return base;
  if (pairSuffix != null) return '$base ($customName $pairSuffix)';
  return '$base ($customName)';
}
