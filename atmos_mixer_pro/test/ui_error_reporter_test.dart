import 'package:atmos_mixer_pro/core/utils/ui_error_reporter.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  late List<String> lines;
  late DateTime clock;
  UiErrorReporter make({Future<void> Function(String, String, String)? sink}) => UiErrorReporter(
        sink: sink ??
            (kind, message, stack) async {
              lines.add('$kind:$message');
            },
        now: () => clock,
      );

  setUp(() {
    lines = [];
    clock = DateTime(2026, 11, 13, 20);
  });

  test('같은 오류는 60초에 한 번만 남기고, 지나면 억제 수와 함께 다시 남긴다', () {
    final r = make();
    expect(r.report('flutter', StateError('x'), null), isTrue);
    expect(r.report('flutter', StateError('x'), null), isFalse);
    clock = clock.add(const Duration(seconds: 61));
    expect(r.report('flutter', StateError('x'), null), isTrue);
    expect(lines, ['flutter:Bad state: x', 'flutter:Bad state: x (그 사이 억제 1건)']);
  });

  test('다른 오류는 따로 남기되 1분 상한을 넘지 않는다', () {
    final r = UiErrorReporter(
      sink: (k, m, s) async => lines.add(m),
      now: () => clock,
      maxPerMinute: 3,
    );
    for (var i = 0; i < 5; i++) {
      r.report('platform', 'e$i', null);
    }
    expect(lines, ['e0', 'e1', 'e2']);
    clock = clock.add(const Duration(minutes: 1));
    r.report('platform', 'e9', null);
    expect(lines.last, 'e9 (그 사이 억제 2건)');
  });

  test('기록하는 곳이 실패해도 예외를 밖으로 던지지 않는다', () async {
    final r = make(sink: (k, m, s) async => throw StateError('ffi down'));
    expect(() => r.report('flutter', 'boom', StackTrace.current), returnsNormally);
    await Future<void>.delayed(Duration.zero); // 비동기 실패가 처리되지 않은 오류로 새지 않는다
  });

  test('스택은 문자열로 넘긴다', () {
    String? got;
    final r = UiErrorReporter(sink: (k, m, s) async => got = s, now: () => clock);
    r.report('flutter', 'e', StackTrace.fromString('#0 main (file.dart:1)'));
    expect(got, '#0 main (file.dart:1)');
  });

  test('꺼져 있으면(준비 전·종료 중) 남기지 않는다', () {
    final r = make();
    r.enabled = false;
    expect(r.report('flutter', 'e', null), isFalse);
    expect(lines, isEmpty);
  });

  test('기록 중에 다시 들어온 오류는 남기지 않는다(재진입 막기)', () {
    late UiErrorReporter r;
    r = UiErrorReporter(
      sink: (k, m, s) async {
        lines.add(m);
        r.report('flutter', 'inner', null); // 기록하는 곳에서 또 오류가 난 상황
      },
      now: () => clock,
    );
    r.report('flutter', 'outer', null);
    expect(lines, ['outer']);
  });

  test('install은 기존 FlutterError 처리기를 그대로 부른다', () {
    final saved = FlutterError.onError;
    addTearDown(() => FlutterError.onError = saved);
    var previousCalled = 0;
    FlutterError.onError = (_) => previousCalled++;
    final r = make();
    r.install();
    FlutterError.onError!(FlutterErrorDetails(exception: StateError('w')));
    expect(previousCalled, 1);
    expect(lines, ['flutter:Bad state: w']);
  });
}
