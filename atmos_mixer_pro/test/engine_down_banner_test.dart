import 'dart:async';
import 'package:atmos_mixer_pro/features/dashboard/widgets/engine_down_banner.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  var clock = DateTime(2026, 11, 12, 9, 5, 7);

  Future<StreamController<String>> pump(WidgetTester tester) async {
    clock = DateTime(2026, 11, 12, 9, 5, 7);
    final c = StreamController<String>.broadcast(sync: true);
    addTearDown(c.close);
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: EngineDownBanner(
          statusStream: c.stream,
          now: () => clock,
        ),
      ),
    ));
    return c;
  }

  test('parseEngineDown reads the cause only for the EngineDown prefix', () {
    expect(parseEngineDown('EngineDown:restart_failed'), 'restart_failed');
    expect(parseEngineDown('Running'), isNull);
    expect(parseEngineDown('Stopped'), isNull);
  });

  testWidgets('hidden until EngineDown, shows cause and time, persists over other statuses', (tester) async {
    final c = await pump(tester);
    expect(find.textContaining('복구에 실패'), findsNothing);

    c.add('EngineDown:restart_failed');
    await tester.pump();
    expect(find.textContaining('restart_failed'), findsOneWidget);
    expect(find.textContaining('09:05:07'), findsOneWidget);
    expect(find.textContaining('연속 1회'), findsOneWidget);

    for (final s in ['Stopped', 'HotReloading', 'WatchdogActive']) {
      c.add(s);
      await tester.pump();
      await tester.pump();
      expect(find.textContaining('restart_failed'), findsOneWidget, reason: s);
    }

    clock = DateTime(2026, 11, 12, 9, 6, 30);
    c.add('EngineDown:restart_failed');
    await tester.pump();
    expect(find.textContaining('연속 2회'), findsOneWidget);
    // 두 번째 실패 뒤에도 시작 시각은 처음 값이다.
    expect(find.textContaining('09:05:07'), findsOneWidget);
    expect(find.textContaining('09:06:30'), findsNothing);
  });

  testWidgets('clears on Running and on Failover', (tester) async {
    final c = await pump(tester);
    for (final recover in ['Running', 'Failover']) {
      c.add('EngineDown:x');
      await tester.pump();
      expect(find.textContaining('마지막 원인'), findsOneWidget);
      c.add(recover);
      await tester.pump();
      expect(find.textContaining('마지막 원인'), findsNothing, reason: recover);
    }
  });
}
