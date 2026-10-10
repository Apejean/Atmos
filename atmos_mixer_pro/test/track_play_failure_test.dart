import 'package:atmos_mixer_pro/features/dashboard/widgets/track_play_failure.dart';
import 'package:atmos_mixer_pro/src/rust/api/error.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

/// 재생 실패는 엔진 고장 창이 아니라 이유가 담긴 알림으로 보인다(2026-10-10 Windows P6-E: 예전에는
/// "치명적 시스템 오류" 창에 `Instance of 'AtmosError'`만 떴다).
void main() {
  Future<BuildContext> pumpScaffold(WidgetTester tester) async {
    late BuildContext captured;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: Builder(builder: (context) {
        captured = context;
        return const SizedBox();
      })),
    ));
    return captured;
  }

  testWidgets('엔진 오류는 메시지를, 그 밖의 오류는 문자열을 보여 준다', (tester) async {
    final context = await pumpScaffold(tester);

    showTrackPlayFailure(context, const AtmosError(message: '파일을 찾을 수 없습니다: /Users/x/a.wav'));
    await tester.pump();
    expect(find.text('트랙 재생 실패: 파일을 찾을 수 없습니다: /Users/x/a.wav'), findsOneWidget);
    expect(find.textContaining('Instance of'), findsNothing);

    ScaffoldMessenger.of(context).removeCurrentSnackBar();
    await tester.pump();
    showTrackPlayFailure(context, StateError('엔진 없음'), prefix: '전시 모드 트랙 재생 실패');
    await tester.pump();
    expect(find.text('전시 모드 트랙 재생 실패: Bad state: 엔진 없음'), findsOneWidget);
  });
}
