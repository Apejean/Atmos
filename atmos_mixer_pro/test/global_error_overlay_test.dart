import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/global_error_overlay.dart';
import 'package:atmos_mixer_pro/src/rust/api/error.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

/// 오류 창 나누기(HANDOFF 남은 일 14): 저장·불러오기 같은 작업 실패는 확인만 받고 엔진을 건드리지 않는다.
/// 엔진·장치 오류는 예전처럼 케이블 확인·엔진 리셋 안내와 함께 뜬다.
void main() {
  Future<ProviderContainer> pumpOverlay(WidgetTester tester) async {
    final container = ProviderContainer(overrides: [
      deviceEventStreamProvider.overrideWith((ref) => const Stream<String>.empty()),
    ]);
    addTearDown(container.dispose);
    await tester.pumpWidget(UncontrolledProviderScope(
      container: container,
      child: const MaterialApp(
        home: Scaffold(body: Stack(children: [GlobalErrorOverlay()])),
      ),
    ));
    return container;
  }

  test('AtmosError는 메시지를, 다른 오류는 문자열을 쓴다', () {
    expect(errorText(const AtmosError(message: '장치 없음')), '장치 없음');
    expect(errorText(StateError('x')), 'Bad state: x');
  });

  testWidgets('작업 실패는 확인만 받고 닫힌다', (tester) async {
    final container = await pumpOverlay(tester);
    container.read(globalErrorProvider.notifier).showOperationError('로그 저장 실패: 쓸 수 없는 폴더');
    await tester.pump();

    expect(find.text('작업을 마치지 못했습니다'), findsOneWidget);
    expect(find.text('로그 저장 실패: 쓸 수 없는 폴더'), findsOneWidget);
    expect(find.text('치명적 시스템 오류 발생'), findsNothing);
    expect(find.text('엔진 리셋 (원클릭 복구)'), findsNothing);

    await tester.tap(find.text('확인'));
    await tester.pump();
    expect(container.read(globalErrorProvider), isNull);
    expect(find.text('작업을 마치지 못했습니다'), findsNothing);
  });

  testWidgets('엔진 오류는 엔진 리셋 안내와 함께 뜬다', (tester) async {
    final container = await pumpOverlay(tester);
    container.read(globalErrorProvider.notifier).showError('오디오 엔진 시작 실패: 장치 없음');
    await tester.pump();

    expect(find.text('치명적 시스템 오류 발생'), findsOneWidget);
    expect(find.text('오디오 엔진 시작 실패: 장치 없음'), findsOneWidget);
    expect(find.text('엔진 리셋 (원클릭 복구)'), findsOneWidget);
    expect(find.text('확인'), findsNothing);
  });
}
