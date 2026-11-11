import 'dart:async';
import 'package:atmos_mixer_pro/features/dashboard/widgets/load_warning_banner.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  Future<StreamController<String>> pump(WidgetTester tester) async {
    final c = StreamController<String>.broadcast(sync: true);
    addTearDown(c.close);
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: LoadWarningBanner(statusStream: c.stream)),
    ));
    return c;
  }

  test('parse reads five values and rejects malformed input', () {
    final l = LoadHigh.parse('LoadHigh:24.0/21.3/3/1/64')!;
    expect(l.maxMs, 24.0);
    expect(l.budgetMs, 21.3);
    expect(l.overruns, 3);
    expect(l.late, 1);
    expect(l.processingChannels, 64);
    expect(LoadHigh.parse('LoadHigh:24.0/21.3/3/1'), isNull);
    expect(LoadHigh.parse('LoadHigh:a/b/c/d/e'), isNull);
    expect(LoadHigh.parse('Running'), isNull);
  });

  testWidgets('shows on LoadHigh, updates values, ignores other statuses', (tester) async {
    final c = await pump(tester);
    expect(find.textContaining('부하가 높아'), findsNothing);

    c.add('LoadHigh:24.0/21.3/3/1/64');
    await tester.pump();
    expect(find.textContaining('최대 24.0ms'), findsOneWidget);
    expect(find.textContaining('처리 채널 64개'), findsOneWidget);

    c.add('LoadHigh:22.5/21.3/1/0/64');
    await tester.pump();
    expect(find.textContaining('최대 22.5ms'), findsOneWidget);

    for (final s in ['Stopped', 'HotReloading', 'EngineDown:x', 'Failover']) {
      c.add(s);
      await tester.pump();
      expect(find.textContaining('부하가 높아'), findsOneWidget, reason: s);
    }
  });

  testWidgets('clears on LoadNormal and on Running', (tester) async {
    final c = await pump(tester);
    for (final clear in ['LoadNormal', 'Running']) {
      c.add('LoadHigh:24.0/21.3/3/1/64');
      await tester.pump();
      expect(find.textContaining('부하가 높아'), findsOneWidget);
      c.add(clear);
      await tester.pump();
      expect(find.textContaining('부하가 높아'), findsNothing, reason: clear);
    }
  });
}
