import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/bass_management_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';

/// 서브우퍼 지정은 **스피커 속성**이다(방별 베이스 매니지먼트).
///
/// 예전에는 베이스 매니지먼트 설정에 서브 채널이 전체에 하나(lfeChannel)라, 모든 방
/// 스피커의 저역이 그 채널 하나로 모였다. 이제 방마다 서브를 하나씩 지정하고, 각 방
/// 메인은 자기 방 서브로만 저역을 보낸다(엔진: rust/src/audio/bass_route.rs).
void main() {
  Future<void> settle() => Future.delayed(const Duration(milliseconds: 400));

  SpeakerNode byId(ProviderContainer c, String id) =>
      c.read(speakerLayoutProvider).firstWhere((n) => n.id == id);

  test('서브 지정은 스피커에 저장되고 재시작 후에도 남는다', () async {
    SharedPreferences.setMockInitialValues({});
    final first = ProviderContainer();
    first.read(speakerLayoutProvider);
    await settle();
    final layout = first.read(speakerLayoutProvider.notifier);
    layout.addSpeaker(SpeakerNode(id: 'main', roomId: 'A', x: 1, y: 1, channel: 0));
    layout.addSpeaker(SpeakerNode(id: 'sub', roomId: 'A', x: 2, y: 1, channel: 2));
    layout.setSubwoofer('sub', true);
    expect(byId(first, 'sub').isSubwoofer, isTrue);
    expect(byId(first, 'main').isSubwoofer, isFalse);
    await settle();
    first.dispose();

    final second = ProviderContainer();
    second.read(speakerLayoutProvider);
    await settle();
    expect(byId(second, 'sub').isSubwoofer, isTrue, reason: '재시작 후 서브 지정이 풀렸다');
    expect(byId(second, 'main').isSubwoofer, isFalse);
    second.dispose();
  });

  test('한 방에 서브는 하나다 — 다른 방 서브는 건드리지 않는다', () async {
    SharedPreferences.setMockInitialValues({});
    final c = ProviderContainer();
    c.read(speakerLayoutProvider);
    await settle();
    final layout = c.read(speakerLayoutProvider.notifier);
    layout.addSpeaker(SpeakerNode(id: 'a1', roomId: 'A', x: 1, y: 1, channel: 0));
    layout.addSpeaker(SpeakerNode(id: 'a2', roomId: 'A', x: 2, y: 1, channel: 1));
    layout.addSpeaker(SpeakerNode(id: 'b1', roomId: 'B', x: 1, y: 1, channel: 2));

    layout.setSubwoofer('a1', true);
    layout.setSubwoofer('b1', true);
    // 방 A의 서브를 a2로 바꾼다.
    layout.setSubwoofer('a2', true);
    expect(byId(c, 'a1').isSubwoofer, isFalse, reason: '같은 방에 서브가 둘이 됐다');
    expect(byId(c, 'a2').isSubwoofer, isTrue);
    expect(byId(c, 'b1').isSubwoofer, isTrue, reason: '다른 방 서브가 풀렸다');

    layout.setSubwoofer('a2', false);
    expect(byId(c, 'a2').isSubwoofer, isFalse);
    expect(byId(c, 'b1').isSubwoofer, isTrue);
    // 자동 조준이 읽은 청사진 상태의 비동기 로드가 끝난 뒤에 정리한다.
    await settle();
    c.dispose();
  });

  test('엔진 payload에 채널별 서브 지정이 실린다', () {
    final payload = buildChannelPositionsPayload([
      SpeakerNode(id: 'main', roomId: 'A', x: 1, y: 1, channel: 0),
      SpeakerNode(id: 'sub', roomId: 'A', x: 2, y: 1, channel: 1, isSubwoofer: true),
    ], 3);
    expect(payload[0]!['is_subwoofer'], isFalse);
    expect(payload[1]!['is_subwoofer'], isTrue);
    expect(payload[2], isNull);
  });

  test('예전 전역 LFE 지정은 그 채널 스피커로 한 번만 옮겨진다', () async {
    // 예전 저장본: 서브가 베이스 매니지먼트 설정의 lfeChannel(내부 채널 번호) 하나였다.
    SharedPreferences.setMockInitialValues({
      'exhibition_speaker_layout': jsonEncode([
        SpeakerNode(id: 'main', roomId: 'A', x: 1, y: 1, channel: 0).toJson()
          ..remove('is_subwoofer'),
        SpeakerNode(id: 'sub', roomId: 'A', x: 2, y: 1, channel: 3).toJson()
          ..remove('is_subwoofer'),
      ]),
      'bass_management_state':
          '{"isEnabled":true,"lfeChannel":3,"crossoverFreq":120.0,"lfeBoostEnabled":true}',
    });
    final first = ProviderContainer();
    first.read(speakerLayoutProvider);
    first.read(bassManagementProvider);
    await settle();
    expect(byId(first, 'sub').isSubwoofer, isTrue, reason: '예전 LFE 지정이 스피커로 옮겨지지 않았다');
    expect(byId(first, 'main').isSubwoofer, isFalse);
    // 나머지 베이스 매니지먼트 설정은 그대로다.
    expect(first.read(bassManagementProvider).crossoverFreq, 120.0);
    expect(first.read(bassManagementProvider).lfeBoostEnabled, isTrue);

    final prefs = await SharedPreferences.getInstance();
    final bass = jsonDecode(prefs.getString('bass_management_state')!) as Map<String, dynamic>;
    expect(bass.containsKey('lfeChannel'), isFalse, reason: '옮긴 뒤에도 예전 키가 남았다');
    expect(bass['crossoverFreq'], 120.0);

    // 사용자가 해제하면 다음 실행에 되살아나지 않는다.
    first.read(speakerLayoutProvider.notifier).setSubwoofer('sub', false);
    await settle();
    first.dispose();
    final second = ProviderContainer();
    second.read(speakerLayoutProvider);
    await settle();
    expect(byId(second, 'sub').isSubwoofer, isFalse, reason: '해제한 서브가 되살아났다');
    second.dispose();
  });

  test('예전에 꺼져 있던 LFE 지정은 옮기지 않는다', () async {
    SharedPreferences.setMockInitialValues({
      'exhibition_speaker_layout': jsonEncode([
        SpeakerNode(id: 'spk', roomId: 'A', x: 1, y: 1, channel: 3).toJson()
          ..remove('is_subwoofer'),
      ]),
      'bass_management_state': '{"isEnabled":false,"lfeChannel":3,"crossoverFreq":80.0}',
    });
    final c = ProviderContainer();
    c.read(speakerLayoutProvider);
    await settle();
    expect(byId(c, 'spk').isSubwoofer, isFalse);
    c.dispose();
  });
}
