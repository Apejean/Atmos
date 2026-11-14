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

  test('parseWaitingForDevice reads attempts and next seconds', () {
    expect(parseWaitingForDevice('WaitingForDevice:3:30'), (3, 30));
    expect(parseWaitingForDevice('WaitingForDevice:x:30'), isNull);
    expect(parseWaitingForDevice('Running'), isNull);
  });

  testWidgets('shows the waiting banner, keeps its start time, clears on Running and StoppedByUser', (tester) async {
    final c = await pump(tester);
    for (final recover in ['Running', 'StoppedByUser']) {
      c.add('WaitingForDevice:1:1');
      await tester.pump();
      expect(find.textContaining('오디오 장치를 기다리는 중입니다 — 1번 시도, 다음 시도 1초 뒤'), findsOneWidget);
      expect(find.textContaining('09:05:07'), findsOneWidget);
      clock = DateTime(2026, 11, 12, 9, 7, 0);
      c.add('WaitingForDevice:3:4');
      await tester.pump();
      expect(find.textContaining('3번 시도, 다음 시도 4초 뒤'), findsOneWidget);
      expect(find.textContaining('09:05:07'), findsOneWidget, reason: '대기 시작 시각은 처음 값이다');
      c.add('Stopped');
      await tester.pump();
      expect(find.textContaining('기다리는 중'), findsOneWidget, reason: 'Stopped로는 내리지 않는다(자기 재시작 중에도 온다)');
      c.add(recover);
      await tester.pump();
      expect(find.textContaining('기다리는 중'), findsNothing, reason: recover);
      clock = DateTime(2026, 11, 12, 9, 5, 7);
    }
  });

  testWidgets('known failure causes show their own advice', (tester) async {
    final c = await pump(tester);
    c.add('EngineDown:start_hung');
    await tester.pump();
    expect(find.textContaining('오디오 드라이버가 응답하지 않습니다. 앱을 다시 시작해야 합니다.'), findsOneWidget);
    c.add('EngineDown:open_failed');
    await tester.pump();
    expect(find.textContaining('오디오 장치는 보이지만 열지 못했습니다'), findsOneWidget);
    c.add('EngineDown:unavailable');
    await tester.pump();
    expect(find.textContaining('이 컴퓨터에서 쓸 수 없습니다'), findsOneWidget);
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
