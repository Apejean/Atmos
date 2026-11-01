import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:atmos_mixer_pro/main.dart' show excludeAppSemantics;

// 2026-11-01 Windows 실기 강제 종료: 접근성 프로그램이 붙은 채 창 크기가 바뀌자 Flutter 엔진의 접근성
// 연결부(AccessibilityBridge::CreateRemoveReparentedNodesUpdate)가 빈 포인터를 읽었다
// (flutter/flutter#192689). 앱 화면을 접근성 트리에서 빼서 그 경로를 없앤다.
Widget _app({TransitionBuilder? builder}) => MaterialApp(
      builder: builder,
      home: Scaffold(
        body: Column(children: [
          const Text('Start'),
          Slider(value: 0.5, onChanged: (_) {}),
        ]),
      ),
    );

void main() {
  testWidgets('대조: 그냥 두면 앱 화면이 접근성 트리에 들어간다', (tester) async {
    final semantics = tester.ensureSemantics();
    await tester.pumpWidget(_app());
    expect(find.bySemanticsLabel('Start'), findsOneWidget);
    semantics.dispose();
  });

  testWidgets('앱 화면은 접근성 트리를 만들지 않는다', (tester) async {
    final semantics = tester.ensureSemantics();
    await tester.pumpWidget(_app(builder: excludeAppSemantics));
    expect(find.bySemanticsLabel('Start'), findsNothing, reason: '앱 화면이 접근성 트리에 남아 있다');
    semantics.dispose();
  });
}
