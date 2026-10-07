---
paths:
  - "atmos_mixer_pro/lib/**"
  - "atmos_mixer_pro/test/**"
  - "atmos_mixer_pro/integration_test/**"
---

# Flutter 규칙

- 비즈니스 로직은 Riverpod provider에 둔다. 위젯은 표시와 입력만.
- 60fps/120Hz로 다시 그리는 캔버스(VU, 3D 방, 히트맵)는 `Listenable`·`RepaintBoundary`로 그 영역만 다시 그린다. 화면 전체 rebuild를 만들지 않는다.
- `flutter analyze` 0건을 유지한다.
- 3D 방 뷰어는 macOS가 `webview_flutter`, Windows가 `webview_windows`(WebView2)다(`three_js_engine_provider.dart`의 `Platform.isWindows` 분기, HANDOFF 남은 일 10). HTML(`studio_engine.html`)은 같고, Windows는 문서 시작 때 넣는 `window.SpeakerBridge` shim으로 JS→Dart 메시지를 받으며 3D 화면이 안 보이면 WebView2를 멈춘다. 한쪽 분기를 바꾸면 다른 쪽 동작을 함께 확인하고 적는다.
- 앱을 띄워 확인할 때는 실제 환경설정을 건드리지 않도록 번들 ID 복사본으로 띄운다(`CFFIXED_USER_HOME`만으로는 환경설정이 분리되지 않는다).
