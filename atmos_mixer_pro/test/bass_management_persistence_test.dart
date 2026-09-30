import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:atmos_mixer_pro/features/exhibition/state/bass_management_provider.dart';

/// 베이스 매니지먼트 전체 설정(크로스오버·LFE +10dB)은 앱을 다시 켜도 유지되어야 한다.
/// 예전에는 메모리에만 있어서 재시작하면 80Hz로 돌아갔다.
/// 서브우퍼 지정은 스피커 속성이다(subwoofer_per_room_test.dart).
void main() {
  // 저장(300ms 디바운스)과 불러오기(비동기)가 끝날 때까지 기다린다.
  Future<void> settle() => Future.delayed(const Duration(milliseconds: 500));

  test('크로스오버 주파수가 재시작 후에도 남는다', () async {
    SharedPreferences.setMockInitialValues({});

    final first = ProviderContainer();
    first.read(bassManagementProvider);
    await settle();
    first.read(bassManagementProvider.notifier).setCrossoverFreq(120.0);
    await settle();
    first.dispose();

    // 앱 재시작: 새 컨테이너가 저장값을 불러온다.
    final second = ProviderContainer();
    second.read(bassManagementProvider);
    await settle();
    expect(second.read(bassManagementProvider).crossoverFreq, 120.0);
    // 불러온 값은 엔진에 다시 보내야 한다(엔진은 이 값을 모른다).
    expect(second.read(bassManagementProvider.notifier).resyncCount, 1);
    second.dispose();
  });

  test('LFE +10dB 토글도 재시작 후에 남는다', () async {
    SharedPreferences.setMockInitialValues({});
    final first = ProviderContainer();
    first.read(bassManagementProvider);
    await settle();
    expect(first.read(bassManagementProvider).lfeBoostEnabled, isFalse,
        reason: '기본은 꺼짐(서브 레벨은 출력단·하드웨어에서 맞춘다)');
    first.read(bassManagementProvider.notifier).setLfeBoostEnabled(true);
    expect(first.read(bassManagementProvider).lfeBoostEnabled, isTrue);
    await settle();
    first.dispose();

    final second = ProviderContainer();
    second.read(bassManagementProvider);
    await settle();
    expect(second.read(bassManagementProvider).lfeBoostEnabled, isTrue);
    second.dispose();
  });

  test('저장값이 손상되어도 기본값으로 시작한다', () async {
    SharedPreferences.setMockInitialValues({'bass_management_state': '{깨진 json'});

    final container = ProviderContainer();
    container.read(bassManagementProvider);
    await settle();
    final state = container.read(bassManagementProvider);
    expect(state.crossoverFreq, 80.0);
    expect(state.lfeBoostEnabled, isFalse);
    container.dispose();
  });
}
