import 'dart:async';
import 'package:atmos_mixer_pro/features/dashboard/widgets/osc_port_banner.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  Future<StreamController<String>> pump(WidgetTester tester) async {
    final c = StreamController<String>.broadcast(sync: true);
    addTearDown(c.close);
    await tester.pumpWidget(MaterialApp(home: Scaffold(body: OscPortBanner(statusStream: c.stream))));
    return c;
  }

  test('parse reads the port only for the blocked prefix', () {
    expect(parseOscPortBlocked('OscPortBlocked:8000'), 8000);
    expect(parseOscPortBlocked('OscPortOk:8000'), isNull);
    expect(parseOscPortBlocked('OscPortBlocked:x'), isNull);
    expect(parseOscPortBlocked('Running'), isNull);
  });

  testWidgets('shows on blocked, ignores engine and other ports, clears on ok for the same port', (tester) async {
    final c = await pump(tester);
    expect(find.textContaining('OSC 수신 포트'), findsNothing);

    c.add('OscPortBlocked:8000');
    await tester.pump();
    expect(find.textContaining('포트 8000'), findsOneWidget);

    for (final s in ['Running', 'Stopped', 'EngineDown:x', 'LoadNormal', 'OscPortOk:9000']) {
      c.add(s);
      await tester.pump();
      expect(find.textContaining('포트 8000'), findsOneWidget, reason: s);
    }

    c.add('OscPortOk:8000');
    await tester.pump();
    expect(find.textContaining('OSC 수신 포트'), findsNothing);
  });
}
