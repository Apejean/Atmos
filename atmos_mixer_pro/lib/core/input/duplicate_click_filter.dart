import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';

/// 원격 제어가 클릭 한 번을 두 번으로 보내는 것을 앱 입구에서 걸러낸다.
///
/// 2026-11-24 Windows 실기: TeamViewer 마우스 모드가 클릭 한 번에 눌림·뗌을 15ms 간격으로 두 쌍 보냈다(OS 수준 기록).
/// Flutter는 두 번을 각각 정상 클릭으로 처리해, 메뉴 창이 두 장 열리고(1.3.20에서 창마다 막음) 재생·전시 모드·표 칸처럼
/// 누를 때마다 뒤집히는 단추는 두 번 눌려 제자리로 돌아갔다. 사람이 일부러 두 번 누르는 간격은 보통 100ms를 넘으므로,
/// 같은 장치·같은 단추로 같은 자리(몇 픽셀 안)에 [window] 안에 다시 들어온 누름은 그 뒤 움직임·뗌까지 통째로 버린다.
/// 기준 시각은 버리지 않은 앞 누름이라, 세 번 이상 겹쳐 와도 [window] 안이면 모두 버린다.
/// 창은 100ms다(2026-11-27 사용자 결정). 휴대폰 TeamViewer의 두 번째 눌림은 32ms(11-24)뿐 아니라 47·62·78ms(11-27)
/// 뒤에도 왔다(OS 기록 `D:\dev\logs\2026-11-27\W1127_mouse_hover*.log`). 처음의 40ms로는 일부를 못 걸렀다.
class DuplicateClickFilter {
  DuplicateClickFilter({this.window = const Duration(milliseconds: 100), this.slop = 4.0});

  /// 이 시간 안에 다시 온 누름은 같은 클릭으로 본다.
  final Duration window;

  /// 같은 자리로 보는 거리(논리 픽셀).
  final double slop;

  _Down? _last;
  final Set<int> _dropping = <int>{};

  /// 지금까지 버린 누름 수(확인용).
  int dropped = 0;

  /// true면 이 이벤트를 앱에 넘기지 않는다.
  bool shouldDrop(PointerEvent event) {
    if (_dropping.contains(event.pointer)) {
      if (event is PointerUpEvent || event is PointerCancelEvent) {
        _dropping.remove(event.pointer);
      }
      return true;
    }
    if (event is! PointerDownEvent) return false;
    if (event.kind != PointerDeviceKind.mouse && event.kind != PointerDeviceKind.touch) return false;
    final last = _last;
    final gap = last == null ? null : event.timeStamp - last.time;
    if (last != null &&
        gap != null &&
        gap >= Duration.zero &&
        gap < window &&
        last.kind == event.kind &&
        last.device == event.device &&
        last.buttons == event.buttons &&
        (event.position - last.position).distance <= slop) {
      _dropping.add(event.pointer);
      dropped++;
      return true;
    }
    _last = _Down(event.timeStamp, event.position, event.kind, event.device, event.buttons);
    return false;
  }
}

class _Down {
  const _Down(this.time, this.position, this.kind, this.device, this.buttons);
  final Duration time;
  final Offset position;
  final PointerDeviceKind kind;
  final int device;
  final int buttons;
}

/// 앱 바인딩. 모든 포인터 이벤트가 [DuplicateClickFilter]를 지난 뒤 Flutter로 간다.
/// `main`의 맨 앞에서 [AtmosBinding.ensureInitialized]를 부른다. 다른 바인딩이 이미 있으면(통합 시험의
/// IntegrationTestWidgetsFlutterBinding) 그것을 그대로 쓰고 거름망은 걸지 않는다 — 바인딩은 하나만 만들 수 있다.
class AtmosBinding extends WidgetsFlutterBinding {
  final DuplicateClickFilter clickFilter = DuplicateClickFilter();

  static WidgetsBinding ensureInitialized() {
    if (!_bindingExists()) AtmosBinding();
    return WidgetsBinding.instance;
  }

  static bool _bindingExists() {
    try {
      WidgetsBinding.instance;
      return true;
    } catch (_) {
      return false;
    }
  }

  @override
  void handlePointerEvent(PointerEvent event) {
    if (clickFilter.shouldDrop(event)) return;
    super.handlePointerEvent(event);
  }
}
