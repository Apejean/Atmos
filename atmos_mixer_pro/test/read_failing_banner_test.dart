import 'dart:async';
import 'package:atmos_mixer_pro/features/dashboard/widgets/read_failing_banner.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  Future<StreamController<String>> pump(WidgetTester tester) async {
    final c = StreamController<String>.broadcast(sync: true);
    addTearDown(c.close);
    await tester.pumpWidget(MaterialApp(home: Scaffold(body: ReadFailingBanner(statusStream: c.stream))));
    return c;
  }

  test('parse reads the count and keeps colons in the file name', () {
    expect(ReadFailing.parse('ReadFailing:0')?.count, 0);
    final one = ReadFailing.parse('ReadFailing:1:zone 1.wav');
    expect(one?.count, 1);
    expect(one?.firstFile, 'zone 1.wav');
    expect(ReadFailing.parse('ReadFailing:2:a:b.wav')?.firstFile, 'a:b.wav');
    expect(ReadFailing.parse('ReadFailing:x:a.wav'), isNull);
    expect(ReadFailing.parse('ReadFailing:3'), isNull, reason: 'a count above 0 needs a file name');
    expect(ReadFailing.parse('Running'), isNull);
  });

  testWidgets('shows on the first value, updates, ignores other statuses, clears on ReadFailing:0', (tester) async {
    final c = await pump(tester);
    expect(find.textContaining('음원 파일을 읽지 못해'), findsNothing);

    // 구독할 때 Rust가 보내는 처음 값으로도 바로 보인다.
    c.add('ReadFailing:1:zone 1.wav');
    await tester.pump();
    expect(find.textContaining('트랙 1개'), findsOneWidget);
    expect(find.textContaining('zone 1.wav'), findsOneWidget);

    c.add('ReadFailing:2:zone 1.wav');
    await tester.pump();
    expect(find.textContaining('트랙 2개'), findsOneWidget);

    // 엔진 상태·다른 배너 값으로는 내려가지 않는다(엔진을 다시 열어 트랙이 버려지면 Rust가 ReadFailing:0을 보낸다).
    for (final s in ['Running', 'Stopped', 'StoppedByUser', 'EngineDown:x', 'LoadNormal', 'OscPortOk:8000']) {
      c.add(s);
      await tester.pump();
      expect(find.textContaining('트랙 2개'), findsOneWidget, reason: s);
    }

    c.add('ReadFailing:1:b.wav');
    await tester.pump();
    expect(find.textContaining('트랙 1개'), findsOneWidget);
    expect(find.textContaining('b.wav'), findsOneWidget);

    c.add('ReadFailing:0');
    await tester.pump();
    expect(find.textContaining('음원 파일을 읽지 못해'), findsNothing);
  });
}
