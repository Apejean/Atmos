import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// 2026-11-16 ④ 실기 F3: 인터페이스를 끈 채 켜면 "치명적 시스템 오류" 창이 떴고, 엔진 유지자가 장치를 다시 열어도
// 닫히지 않았다. 엔진이 다시 열렸다는 상태(Running·Failover)가 오면 엔진 오류 창만 닫는다.
void main() {
  test('엔진이 다시 열린 상태값만 엔진 오류 창을 닫는다', () {
    expect(engineRecoveredStatus('Running'), isTrue);
    expect(engineRecoveredStatus('Failover'), isTrue);
    expect(engineRecoveredStatus('WaitingForDevice:3:30'), isFalse);
    expect(engineRecoveredStatus('EngineDown:start_hung'), isFalse);
    expect(engineRecoveredStatus('StoppedByUser'), isFalse);
    expect(engineRecoveredStatus('Stopped'), isFalse);
  });

  test('clearEngineError는 엔진 오류만 닫고 작업 실패 창은 그대로 둔다', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(globalErrorProvider.notifier);

    notifier.showError('오디오 엔진 연결 시간 초과');
    notifier.clearEngineError();
    expect(container.read(globalErrorProvider), isNull);

    notifier.showOperationError('프로젝트 내보내기 실패');
    notifier.clearEngineError();
    expect(container.read(globalErrorProvider)?.message, '프로젝트 내보내기 실패');
  });
}
