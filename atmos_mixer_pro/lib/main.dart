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

import 'package:atmos_mixer_pro/features/splash/screens/audio_init_splash_screen.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/acoustic_sync_provider.dart';

Future<void> main(List<String> args) async {
  WidgetsFlutterBinding.ensureInitialized();

  // Initialize rust bridge
  //
  // 앱 안의 Rust 라이브러리를 직접 연다(core/utils/rust_library.dart). 기본 로더는 작업
  // 디렉터리의 rust/target/release/를 먼저 봐서, 프로젝트 폴더에서 flutter run하면 옛 Rust가 실렸다.
  await RustLib.init(externalLibrary: bundledRustLibrary());

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

  // Initialize window_manager for frameless kiosk mode
  await windowManager.ensureInitialized();

  WindowOptions windowOptions = const WindowOptions(
    size: Size(1024, 768),
    minimumSize: Size(800, 600),
    center: true,
    backgroundColor: Colors.transparent,
    skipTaskbar: false,
    titleBarStyle: TitleBarStyle.normal,
  );

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

class AtmosMixerProApp extends ConsumerStatefulWidget {
  const AtmosMixerProApp({super.key});

  @override
  ConsumerState<AtmosMixerProApp> createState() => _AtmosMixerProAppState();
}

class _AtmosMixerProAppState extends ConsumerState<AtmosMixerProApp>
    with WindowListener {
  Timer? _heartbeat;
  bool _heartbeatInFlight = false;

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
      }
      // 엔진이 스스로 재시작했다(워치독·장치 오류). 설정을 다시 밀어 넣은 뒤 알려야 Rust가
      // 멈춘 위치부터 재생을 이어 튼다(rust core::restart_resume).
      final seq = event == null ? null : parseEngineRestartedSeq(event);
      if (seq != null) {
        resyncEngineStateFromWidgetRef(ref);
        apiAckEngineRestart(seq: seq);
      }
    });

    return MaterialApp(
      title: 'Atmos Mixer Pro',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        brightness: Brightness.dark,
        fontFamily: 'Pretendard', // Fallback to system font if not provided
      ),
      home: const AudioInitSplashScreen(),
    );
  }
}
