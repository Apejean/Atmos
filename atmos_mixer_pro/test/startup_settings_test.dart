import 'package:atmos_mixer_pro/features/settings/widgets/startup_settings.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

const _autostartLabel = 'Windows 시작 시\n자동 실행';
const _resumeLabel = '로그인할 때\n공연 자동 시작';

Future<void> _pump(
  WidgetTester tester, {
  bool showWindowsAutostart = true,
  bool windowsAutostart = true,
  bool autoResumeOnLogon = true,
  ValueChanged<bool>? onWindowsAutostart,
  ValueChanged<bool>? onAutoResume,
}) {
  return tester.pumpWidget(MaterialApp(
    home: Scaffold(
      body: StartupSettings(
        showWindowsAutostart: showWindowsAutostart,
        windowsAutostart: windowsAutostart,
        autoResumeOnLogon: autoResumeOnLogon,
        onWindowsAutostartChanged: onWindowsAutostart ?? (_) {},
        onAutoResumeOnLogonChanged: onAutoResume ?? (_) {},
      ),
    ),
  ));
}

List<Switch> _switches(WidgetTester tester) => tester.widgetList<Switch>(find.byType(Switch)).toList();

void main() {
  testWidgets('Windows에서는 자동 실행 스위치가 공연 자동 시작 위에 있다', (tester) async {
    await _pump(tester);
    expect(find.text(_autostartLabel), findsOneWidget);
    expect(find.text(_resumeLabel), findsOneWidget);
    expect(tester.getTopLeft(find.text(_autostartLabel)).dy, lessThan(tester.getTopLeft(find.text(_resumeLabel)).dy));
  });

  testWidgets('자동 실행을 끄면 공연 자동 시작은 값을 둔 채 잠긴다', (tester) async {
    await _pump(tester, windowsAutostart: false, autoResumeOnLogon: true);
    final switches = _switches(tester);
    expect(switches, hasLength(2));
    expect(switches[0].value, isFalse);
    expect(switches[0].onChanged, isNotNull, reason: '자동 실행은 다시 켤 수 있어야 한다');
    expect(switches[1].value, isTrue);
    expect(switches[1].onChanged, isNull);
  });

  testWidgets('자동 실행이 켜져 있으면 공연 자동 시작을 바꿀 수 있다', (tester) async {
    bool? changed;
    await _pump(tester, onAutoResume: (value) => changed = value);
    await tester.tap(find.byType(Switch).last);
    expect(changed, isFalse);
  });

  testWidgets('자동 실행 스위치를 누르면 바뀐 값을 알린다', (tester) async {
    bool? changed;
    await _pump(tester, onWindowsAutostart: (value) => changed = value);
    await tester.tap(find.byType(Switch).first);
    expect(changed, isFalse);
  });

  testWidgets('Windows가 아니면 자동 실행 스위치가 없고 공연 자동 시작은 그대로 쓴다', (tester) async {
    await _pump(tester, showWindowsAutostart: false, windowsAutostart: false);
    expect(find.text(_autostartLabel), findsNothing);
    final switches = _switches(tester);
    expect(switches, hasLength(1));
    expect(switches.single.onChanged, isNotNull);
  });
}
