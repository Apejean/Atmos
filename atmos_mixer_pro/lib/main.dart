import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/core/state/engine_resync.dart';
import 'package:atmos_mixer_pro/core/state/launch_mode.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:window_manager/window_manager.dart';
import 'package:atmos_mixer_pro/src/rust/frb_generated.dart';
import 'package:atmos_mixer_pro/src/rust/api/lifecycle.dart';
import 'package:atmos_mixer_pro/src/rust/api/show.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart';
import 'package:atmos_mixer_pro/core/utils/rust_library.dart';
import 'package:atmos_mixer_pro/core/utils/ui_error_reporter.dart';
import 'package:atmos_mixer_pro/src/rust/api/ui_log.dart';
import 'package:atmos_mixer_pro/core/state/config_notice.dart';
import 'package:atmos_mixer_pro/core/input/duplicate_click_filter.dart';
import 'package:atmos_mixer_pro/src/rust/api/prefs_file.dart';

import 'package:atmos_mixer_pro/features/splash/screens/audio_init_splash_screen.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/acoustic_sync_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/three_js_engine_provider.dart';

/// 화면 쪽 예외 기록기(core/utils/ui_error_reporter.dart). 앱 로그(rust api::ui_log)로 보내고 기다리지 않는다.
final uiErrorReporter = UiErrorReporter(
  sink: (kind, message, stack) => apiLogUiError(kind: kind, message: message, stack: stack),
);

Future<void> main(List<String> args) async {
  // 원격 제어가 클릭 한 번을 두 번으로 보내는 것을 앱 입구에서 거른다(core/input/duplicate_click_filter.dart).
  // 다른 바인딩보다 먼저 만들어야 걸린다.
  AtmosBinding.ensureInitialized();

  // Initialize rust bridge
  //
  // 앱 안의 Rust 라이브러리를 직접 연다(core/utils/rust_library.dart). 기본 로더는 작업
  // 디렉터리의 rust/target/release/를 먼저 봐서, 프로젝트 폴더에서 flutter run하면 옛 Rust가 실렸다.
  await RustLib.init(externalLibrary: bundledRustLibrary());
  // 화면 쪽 예외를 앱 로그에 남긴다(Rust가 준비된 뒤). 기록만 하고 기존 처리는 그대로다.
  uiErrorReporter.install();

  // 시작 관문(rust api::lifecycle). 창을 띄우기 전에 감시 프로그램으로 넘기거나, 이미 떠 있으면 끝낸다.
  switch (await apiStartupGate(args: args)) {
    case StartupDecision.handedToSupervisor:
      exit(0);
    case StartupDecision.duplicate:
      exit(3);
    case StartupDecision.proceed:
      break;
  }
  // 지난 공연 상태를 읽어 두고 5초마다 저장을 시작한다(rust api::show). 이어 가기(스플래시)는
  // 여기서 읽어 둔 상태를 쓰므로, 저장이 파일을 덮어써도 멈춘 위치를 잃지 않는다.
  await apiStartShowState(dir: await showStateDir());

  // 설계 데이터 파일(shared_preferences.json)을 앱이 처음 읽기 전에 검사한다(rust core::prefs_guard, 검토 A3).
  // 플러그인은 처음 읽은 내용을 메모리에 두므로 반드시 SharedPreferences를 쓰는 어떤 코드보다 먼저여야 한다
  // (test/prefs_check_order_test.dart가 이 순서를 지킨다). 안내는 시작 절차가 끝난 뒤 설정 복구 배너로 보인다.
  try {
    pendingPrefsNotice = await apiCheckPrefsFile(
      path: '${await showStateDir()}${Platform.pathSeparator}shared_preferences.json',
    );
  } catch (_) {
    // 검사에 실패해도 앱은 뜬다(플러그인이 지금처럼 읽는다).
  }

  // 로딩창 로고를 창이 뜨기 전에 읽어 둔다.
  await precacheSplashLogo();

  // Initialize window_manager for frameless kiosk mode
  await windowManager.ensureInitialized();

  // 창 제목은 회사 이름(2026-10-22 사용자 결정). 예전에는 실행 파일 이름 "atmos_mixer_pro"가 그대로 보였다.
  WindowOptions windowOptions = const WindowOptions(
    size: Size(1024, 768),
    minimumSize: Size(800, 600),
    center: true,
    backgroundColor: Colors.transparent,
    skipTaskbar: false,
    title: 'The Awesome',
    titleBarStyle: TitleBarStyle.normal,
  );

  // Windows에서는 창이 여기서 처음 보인다(크기를 정하고 가운데로 옮긴 뒤). runner(flutter_window.cpp)가 첫
  // 프레임에 창을 보이면 크기만 바뀌고 아직 가운데로 가지 않은 창이 (10,10)에 잠깐 보였다(1.3.8 설치본, 2026-11-03
  // 5번 중 5번 41~83ms). 여기까지 오지 못하면 runner가 5초 뒤에 창을 보인다.
  windowManager.waitUntilReadyToShow(windowOptions, () async {
    await windowManager.setPreventClose(true);
    await windowManager.show();
    await windowManager.focus();
  });

  runApp(ProviderScope(
    overrides: [launchArgsProvider.overrideWithValue(args)],
    child: const AtmosMixerProApp(),
  ));
}

/// 앱 화면 전체를 접근성 트리에서 뺀다(MaterialApp.builder). Flutter Windows 엔진의 접근성 연결부는 화면
/// 낭독기 같은 접근성 프로그램이 붙어 있을 때 창 크기 변경 등으로 노드가 옮겨지면 빈 포인터를 읽어 앱을
/// 끈다(flutter/flutter#192689, 2026-11-01 실기: AccessibilityBridge::CreateRemoveReparentedNodesUpdate).
/// 운영 앱이라 화면 낭독기를 쓰지 않으므로 트리를 만들지 않아 그 경로를 없앤다(2026-11-01 사용자 결정).
Widget excludeAppSemantics(BuildContext context, Widget? child) =>
    ExcludeSemantics(child: child ?? const SizedBox.shrink());

class AtmosMixerProApp extends ConsumerStatefulWidget {
  const AtmosMixerProApp({super.key});

  @override
  ConsumerState<AtmosMixerProApp> createState() => _AtmosMixerProAppState();
}

class _AtmosMixerProAppState extends ConsumerState<AtmosMixerProApp>
    with WindowListener {
  Timer? _heartbeat;
  bool _heartbeatInFlight = false;
  final _restartResync = RestartResyncGate();

  @override
  void initState() {
    super.initState();
    windowManager.addListener(this);
    // 감시 프로그램이 이 앱이 응답하는지 본다(rust api::lifecycle). 3D 로딩과 상관없이 바로 시작한다.
    _beat();
    _heartbeat = Timer.periodic(const Duration(seconds: 2), (_) => _beat());
  }

  /// 앞 호출이 아직 안 끝났으면 건너뛴다. Rust가 막혀 있으면 하트비트가 끊겨야 감시가 알아챈다.
  Future<void> _beat() async {
    if (_heartbeatInFlight) return;
    _heartbeatInFlight = true;
    try {
      await apiHeartbeat();
    } catch (_) {
      // 이번 하트비트를 못 썼다. 다음 주기에 다시 쓴다.
    } finally {
      _heartbeatInFlight = false;
    }
  }

  @override
  void dispose() {
    _heartbeat?.cancel();
    windowManager.removeListener(this);
    super.dispose();
  }

  @override
  void onWindowClose() async {
    // 종료 중에는 화면 오류를 남기지 않는다(엔진·Rust가 정리되는 중).
    uiErrorReporter.enabled = false;
    // 운영자가 닫는다. 감시 프로그램이 다시 띄우지 않도록 엔진을 멈추기 전에 표시한다(rust api::lifecycle).
    await Future.any([
      apiMarkCleanExit(),
      Future.delayed(const Duration(milliseconds: 500)),
    ]);
    // Explicitly release ASIO hardware locks and cleanly stop audio engine
    // Adding timeout guard to prevent ghost processes
    await Future.any([
      apiStopAudioEngine(),
      Future.delayed(const Duration(milliseconds: 1500)),
    ]);
    // 3D 웹뷰는 Flutter 엔진이 살아 있을 때 정리한다(Windows 종료 크래시,
    // ThreeJsEngineService.disposeWindowsWebview 참고). 아직 만들지 않았으면 새로 만들지 않는다.
    if (ref.exists(threeJsEngineProvider)) {
      await Future.any([
        ref
            .read(threeJsEngineProvider)
            .disposeWindowsWebview()
            .catchError((Object _) {}),
        Future.delayed(const Duration(milliseconds: 1000)),
      ]);
    }
    await windowManager.destroy();
  }

  @override
  Widget build(BuildContext context) {
    // 스피커 배치 -> 채널 FX 자동 동기화를 앱 수명 내내 살려둔다.
    //
    // 예전에는 대시보드 화면에서만 watch해서, 스피커 레이아웃 화면에 바로
    // 들어가면 Riverpod 지연 생성 때문에 동기화가 아예 만들어지지 않았다.
    // 그러면 스피커를 아무리 움직여도 EQ/게인/딜레이가 갱신되지 않는다
    // (실기 보고: "FX 설정들이 스피커를 옮기면 아예 변경이 안됨").
    ref.watch(acousticSyncProvider);

    // 엔진이 스스로 다시 준비됐을 때(워치독 복구, 장치 재연결)에도 현재 설정을
    // 다시 밀어 넣는다. 이 경로는 Dart가 재기동을 요청한 게 아니라서 위의
    // 명시적 호출들로는 덮이지 않는다.
    ref.listen(deviceEventStreamProvider, (previous, next) {
      final event = next.value;
      if (event == 'EngineReady') {
        resyncEngineStateFromWidgetRef(ref);
        _restartResync.readyResynced();
      }
      // 엔진이 스스로 재시작했다(워치독·장치 오류). 설정을 다시 밀어 넣은 뒤 알려야 Rust가
      // 멈춘 위치부터 재생을 이어 튼다(rust core::restart_resume). 같은 재시작의 EngineReady에서
      // 방금 밀어 넣었으면 다시 하지 않는다(RestartResyncGate).
      final seq = event == null ? null : parseEngineRestartedSeq(event);
      if (seq != null) {
        if (_restartResync.resyncOnRestart()) resyncEngineStateFromWidgetRef(ref);
        apiAckEngineRestart(seq: seq);
      }
    });

    return MaterialApp(
      title: 'The Awesome',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        brightness: Brightness.dark,
        fontFamily: 'Pretendard', // Fallback to system font if not provided
      ),
      builder: excludeAppSemantics,
      home: const AudioInitSplashScreen(),
    );
  }
}
