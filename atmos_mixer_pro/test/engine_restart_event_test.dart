import 'package:atmos_mixer_pro/core/state/engine_resync.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('EngineRestarted 이벤트에서 순번을 꺼내고, 다른 이벤트는 무시한다', () {
    expect(parseEngineRestartedSeq('EngineRestarted:7'), 7);
    expect(parseEngineRestartedSeq('EngineReady'), isNull);
    expect(parseEngineRestartedSeq('EngineRestarted:'), isNull);
    expect(parseEngineRestartedSeq('DeviceNotAvailable'), isNull);
  });

  test('자기 재시작 때 EngineReady에서 방금 재동기화했으면 EngineRestarted에서는 다시 하지 않는다', () {
    var now = DateTime(2026, 10, 21, 12);
    final gate = RestartResyncGate(now: () => now);
    gate.readyResynced();
    now = now.add(const Duration(milliseconds: 100));
    expect(gate.resyncOnRestart(), isFalse);
    expect(gate.resyncOnRestart(), isTrue, reason: 'EngineReady 없이 온 다음 재시작은 재동기화한다');
  });

  test('EngineReady가 한참 전이면 EngineRestarted에서 재동기화한다', () {
    var now = DateTime(2026, 10, 21, 12);
    final gate = RestartResyncGate(now: () => now);
    gate.readyResynced();
    now = now.add(const Duration(seconds: 2));
    expect(gate.resyncOnRestart(), isTrue);
  });
}
