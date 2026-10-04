import 'package:atmos_mixer_pro/core/state/engine_resync.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('EngineRestarted 이벤트에서 순번을 꺼내고, 다른 이벤트는 무시한다', () {
    expect(parseEngineRestartedSeq('EngineRestarted:7'), 7);
    expect(parseEngineRestartedSeq('EngineReady'), isNull);
    expect(parseEngineRestartedSeq('EngineRestarted:'), isNull);
    expect(parseEngineRestartedSeq('DeviceNotAvailable'), isNull);
  });
}
