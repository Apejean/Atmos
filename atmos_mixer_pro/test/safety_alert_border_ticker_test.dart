import 'package:atmos_mixer_pro/features/dashboard/widgets/safety_alert_border.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

// 2026-11-17 장치 대기 중 CPU(한 코어의 20~29%) 조사: 대시보드를 감싸는 경고 테두리의 깜빡임 애니메이션이
// 경고가 없어도 initState부터 계속 돌아, 앱이 떠 있는 내내 화면을 매 프레임(60Hz) 다시 그렸다.
// 경고가 있을 때만 깜빡인다.
void main() {
  Widget border({bool watchdog = false, bool autoGuard = false}) => MaterialApp(
        home: SafetyAlertBorderWidget(
          isWatchdogActive: watchdog,
          isAutoGuardActive: autoGuard,
          child: const SizedBox(),
        ),
      );

  testWidgets('경고가 없으면 깜빡임 애니메이션이 돌지 않는다', (tester) async {
    await tester.pumpWidget(border());
    expect(tester.hasRunningAnimations, isFalse);
  });

  testWidgets('경고가 켜지면 깜빡이고 꺼지면 다시 멈춘다', (tester) async {
    await tester.pumpWidget(border(watchdog: true));
    expect(tester.hasRunningAnimations, isTrue);

    await tester.pumpWidget(border());
    expect(tester.hasRunningAnimations, isFalse);

    await tester.pumpWidget(border(autoGuard: true));
    expect(tester.hasRunningAnimations, isTrue);

    await tester.pumpWidget(border());
    expect(tester.hasRunningAnimations, isFalse);
  });

  testWidgets('처음부터 경고가 있으면 바로 깜빡인다', (tester) async {
    await tester.pumpWidget(border(autoGuard: true));
    await tester.pump(const Duration(milliseconds: 300));
    expect(tester.hasRunningAnimations, isTrue);
  });
}
