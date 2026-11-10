import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:flutter_test/flutter_test.dart';

/// Reset·프로젝트 불러오기: 화면의 음소거 표시만 끄면 엔진은 계속 음소거라 소리가 안 난다(2026-11-10 사용자 결정으로 수정).
void main() {
  testWidgets('음소거가 아니면 엔진에 아무것도 보내지 않고 기다리지도 않는다', (tester) async {
    var unmutes = 0;

    await clearEngineMuteAfterStop(muted: false, unmute: () async => unmutes++);

    expect(unmutes, 0);
  });

  testWidgets('음소거 중이면 정지한 트랙이 줄어든 뒤에 엔진 음소거를 푼다', (tester) async {
    var unmutes = 0;

    final done = clearEngineMuteAfterStop(muted: true, unmute: () async => unmutes++);

    await tester.pump(kUnmuteAfterStopDelay - const Duration(milliseconds: 1));
    expect(unmutes, 0, reason: '트랙이 줄어드는 중에 풀면 음소거해 둔 소리가 샌다');

    await tester.pump(const Duration(milliseconds: 1));
    await done;
    expect(unmutes, 1);
  });

  test('기다리는 시간은 트랙이 줄어드는 300ms보다 길다(mixer.rs fade_frames)', () {
    expect(kUnmuteAfterStopDelay, greaterThan(const Duration(milliseconds: 300)));
  });

  testWidgets('엔진 음소거 해제가 실패하면 호출한 쪽이 알 수 있다', (tester) async {
    final done = clearEngineMuteAfterStop(muted: true, unmute: () async => throw StateError('engine'));
    final check = expectLater(done, throwsStateError);

    await tester.pump(kUnmuteAfterStopDelay);
    await check;
  });
}
