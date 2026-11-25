/// 같은 창을 겹쳐 열지 않게 하는 문지기.
///
/// 메뉴 항목 하나를 누를 때 클릭이 한 프레임 안에 두 번 들어오면 `onPressed`가 두 번 불려 창이 두 장 겹쳐 열린다.
/// 앞 창에서 저장한 값을 뒤 창이 옛 값으로 다시 저장해 환경설정이 되돌아간다. 2026-11-24 Windows 실기에서 원격
/// 제어(TeamViewer 마우스 모드)가 클릭 한 번에 눌림·뗌을 15ms 간격으로 두 쌍 보냈고(OS 수준 기록에서 확인),
/// Flutter 쪽은 그 두 번을 각각 정상 처리했을 뿐이다. 입력을 보내는 쪽은 앱이 고칠 수 없으니, 창이 열려 있는
/// 동안에는 같은 창을 다시 열지 않는다.
class SingleOpenGuard {
  bool _open = false;

  /// [run]이 아직 끝나지 않았다(창이 열려 있다).
  bool get isOpen => _open;

  /// [open]을 부른다. 앞서 부른 [open]이 끝나기 전에 다시 부르면 아무것도 하지 않는다.
  /// 창을 여는 쪽은 `showDialog`처럼 창이 닫힐 때 끝나는 Future를 돌려줘야 한다.
  /// [open]이 예외를 던져도 표시는 풀린다(그대로 두면 그 창을 다시는 열 수 없다).
  Future<void> run(Future<void> Function() open) async {
    if (_open) return;
    _open = true;
    try {
      await open();
    } finally {
      _open = false;
    }
  }
}
