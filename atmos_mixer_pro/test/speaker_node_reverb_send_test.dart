// 스피커 리버브 센드의 정규화 규칙을 고정한다.
//
// 예전 UI는 센드를 0~100(%)로 저장했고 지금 기본값은 0~1이다. 그래서 1보다
// 크면 퍼센트로 보고 100으로 나눈다. 엔진 전송(speaker_layout_state)과
// 스피커 인스펙터의 Send 슬라이더가 이 규칙 하나를 공유해야 화면에 보이는
// 값과 실제로 걸리는 잔향 양이 어긋나지 않는다.

import 'package:flutter_test/flutter_test.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';

SpeakerNode _node(double send) =>
    SpeakerNode(id: 'spk', x: 0.0, y: 0.0, channel: 0, reverbSend: send);

void main() {
  test('0~1 값은 그대로 쓴다', () {
    expect(_node(0.0).reverbSendNormalized, 0.0);
    expect(_node(0.5).reverbSendNormalized, 0.5);
    expect(_node(1.0).reverbSendNormalized, 1.0);
  });

  test('예전 퍼센트 저장값은 0~1로 바꾼다', () {
    expect(_node(100.0).reverbSendNormalized, 1.0);
    expect(_node(19.1).reverbSendNormalized, closeTo(0.191, 1e-9));
  });

  test('범위를 벗어난 값은 잘라낸다', () {
    expect(_node(150.0).reverbSendNormalized, 1.0);
    expect(_node(-0.3).reverbSendNormalized, 0.0);
  });
}
