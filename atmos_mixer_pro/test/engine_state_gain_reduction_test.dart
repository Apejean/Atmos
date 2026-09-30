// 대시보드 GR 미터가 실측 게인 리덕션을 받도록 배선을 고정한다.
//
// 예전에는 Rust가 EngineStateUpdate.gain_reduction_db로 실측값을 보내는데
// Dart EngineState에만 그 필드가 없어서 값이 끊겼고, 위젯이 shortTermLufs로
// GR을 추정해 표시했다("-((lufs + 12) * 1.5)", 원 주석에도 Rough
// approximation). 그래서 리미터가 동작하지 않아도 라우드니스만 높으면
// 리덕션이 걸린 것처럼 보였다. 필드나 copyWith 전달이 다시 빠지면 같은
// 증상으로 조용히 되돌아가므로 여기서 계약을 잠근다.

import 'package:flutter_test/flutter_test.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';

void main() {
  test('EngineState가 게인 리덕션을 보존한다', () {
    const gr = 4.25;
    final s = EngineState(gainReductionDb: gr);
    expect(s.gainReductionDb, gr);

    // 다른 필드만 바꿔도 GR은 유지되어야 한다.
    final kept = s.copyWith(shortTermLufs: -9.0);
    expect(
      kept.gainReductionDb,
      gr,
      reason: 'copyWith가 gainReductionDb를 떨어뜨렸다',
    );

    final updated = s.copyWith(gainReductionDb: 7.5);
    expect(updated.gainReductionDb, 7.5);
  });

  test('기본 게인 리덕션은 0dB이다', () {
    expect(EngineState().gainReductionDb, 0.0);
  });
}
