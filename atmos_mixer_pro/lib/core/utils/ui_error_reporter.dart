import 'package:flutter/foundation.dart';

/// 화면(Dart) 쪽 예외를 앱 로그에 남긴다(검토 보고서 A8). 기록만 하고 동작은 바꾸지 않는다.
/// - 같은 오류는 [repeatWindow] 안에 한 번만, 전체는 1분에 [maxPerMinute]줄까지 남긴다(빌드 오류가 프레임마다
///   되풀이될 때 로그가 넘치지 않게).
/// - 기록 중에 나는 예외는 삼킨다(오류 처리기 안에서 다시 던지지 않는다).
class UiErrorReporter {
  UiErrorReporter({
    required this.sink,
    DateTime Function()? now,
    this.repeatWindow = const Duration(seconds: 60),
    this.maxPerMinute = 20,
  }) : _now = now ?? DateTime.now;

  /// 실제로 남기는 곳(앱에서는 Rust 앱 로그). kind: flutter / platform.
  final Future<void> Function(String kind, String message, String stack) sink;
  final DateTime Function() _now;
  final Duration repeatWindow;
  final int maxPerMinute;

  /// 꺼져 있으면 아무것도 남기지 않는다(Rust 준비 전·앱 종료 중).
  bool enabled = true;
  bool _inReport = false;

  final Map<String, DateTime> _lastSeen = {};
  DateTime? _windowStart;
  int _windowCount = 0;
  int _suppressed = 0;

  /// 남겼으면 true(억제됐으면 false).
  bool report(String kind, Object error, StackTrace? stack) {
    // 기록하다 난 예외가 다시 오류 처리기로 들어와 되풀이되지 않게 한다.
    if (!enabled || _inReport) return false;
    _inReport = true;
    try {
      final now = _now();
      final message = '$error';
      final key = '$kind|$message';
      final last = _lastSeen[key];
      if (last != null && now.difference(last) < repeatWindow) {
        _suppressed++;
        return false;
      }
      if (_windowStart == null || now.difference(_windowStart!) >= const Duration(minutes: 1)) {
        _windowStart = now;
        _windowCount = 0;
      }
      if (_windowCount >= maxPerMinute) {
        _suppressed++;
        return false;
      }
      _windowCount++;
      _lastSeen[key] = now;
      if (_lastSeen.length > 200) _lastSeen.remove(_lastSeen.keys.first);
      final note = _suppressed > 0 ? ' (그 사이 억제 $_suppressed건)' : '';
      _suppressed = 0;
      sink(kind, '$message$note', stack?.toString() ?? '').catchError((_) {});
      return true;
    } catch (_) {
      return false;
    } finally {
      _inReport = false;
    }
  }

  /// 전역 처리기를 단다. 기존 FlutterError.onError는 기록 뒤 그대로 부르고, PlatformDispatcher.onError는
  /// 기록 뒤 false를 돌려 기본 처리를 바꾸지 않는다. Rust(RustLib.init)가 준비된 뒤에 부른다.
  void install() {
    final previous = FlutterError.onError;
    FlutterError.onError = (details) {
      report('flutter', details.exceptionAsString(), details.stack);
      previous?.call(details);
    };
    final previousPlatform = PlatformDispatcher.instance.onError;
    PlatformDispatcher.instance.onError = (error, stack) {
      report('platform', error, stack);
      return previousPlatform?.call(error, stack) ?? false;
    };
  }
}
