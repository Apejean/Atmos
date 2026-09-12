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
      for (final e in sortedMono) {
        final key = e.key; // 1-based (레거시)
        final setting = e.value;

        final realCh1 = key - 1; // 0-based 변환
        if (realCh1 < hwCount) {
          items.add(
            ChannelRoutingItem(
              value: ChannelDropdownValueHelper.getMonoValue(realCh1),
              label: _withCustomSuffix(
                'Mono (Ch-${realCh1 + 1})',
                setting.customName,
                pairSuffix: 'L',
              ),
              realChannel0: realCh1,
              isPartialOutput: false,
            ),
          );
        }

        final realCh2 = key; // 0-based (모노 슬롯이 L/R 페어를 연다는 기존 설계 유지)
        if (realCh2 < hwCount) {
          items.add(
            ChannelRoutingItem(
              value: ChannelDropdownValueHelper.getMonoValue(realCh2),
              label: _withCustomSuffix(
                'Mono (Ch-${realCh2 + 1})',
                setting.customName,
                pairSuffix: 'R',
              ),
              realChannel0: realCh2,
              isPartialOutput: false,
            ),
          );
        }
      }
    } else {
      for (int i = 0; i < hwCount; i++) {
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
        // 짝 채널(realCh+1, 0-based)이 하드웨어에 없으면 만들지 않는다.
        if (realCh + 1 < hwCount) {
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
        // 시작 채널 자체가 하드웨어 상한을 넘으면 만들지 않는다.
        if (startCh < hwCount) {
          items.add(
            _buildMultiItem(
              startCh: startCh,
              fileCh: fileCh,
              hwCount: hwCount,
              customName: setting.customName,
            ),
          );
        }
      }
    } else {
      final lastStart = hwCount - fileCh;
      if (lastStart >= 0) {
        for (int i = 0; i <= lastStart; i++) {
          items.add(_buildMultiItem(startCh: i, fileCh: fileCh, hwCount: hwCount));
        }
      } else {
        // 파일 채널 수가 하드웨어 채널 수보다 많아 완전히 들어갈 자리가 없는
        // 경우에도, 트랙이 선택 불가가 되지 않도록 Ch-1 시작 항목 하나는
        // 남긴다(부분 출력으로 표시).
        items.add(_buildMultiItem(startCh: 0, fileCh: fileCh, hwCount: hwCount));
      }
    }
  }

  return items;
}

/// Multi 항목 1개를 만든다. 요청한 범위(`startCh`~`startCh+fileCh-1`)가
/// 하드웨어 상한(`hwCount-1`)을 넘으면 실제로 열리는 범위까지만 라벨에 담고
/// `isPartialOutput`을 true로 표시한다.
ChannelRoutingItem _buildMultiItem({
  required int startCh,
  required int fileCh,
  required int hwCount,
  String customName = '',
}) {
  final requestedEnd = startCh + fileCh - 1;
  final availableEnd = hwCount - 1;
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
