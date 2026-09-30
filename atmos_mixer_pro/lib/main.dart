import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/core/state/engine_resync.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:window_manager/window_manager.dart';
import 'package:atmos_mixer_pro/src/rust/frb_generated.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart';

import 'package:atmos_mixer_pro/features/splash/screens/audio_init_splash_screen.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/acoustic_sync_provider.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Initialize rust bridge
  await RustLib.init();

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

  runApp(const ProviderScope(child: AtmosMixerProApp()));
}

class AtmosMixerProApp extends ConsumerStatefulWidget {
  const AtmosMixerProApp({super.key});

  @override
  ConsumerState<AtmosMixerProApp> createState() => _AtmosMixerProAppState();
}

class _AtmosMixerProAppState extends ConsumerState<AtmosMixerProApp>
    with WindowListener {
  @override
  void initState() {
    super.initState();
    windowManager.addListener(this);
  }

  @override
  void dispose() {
    windowManager.removeListener(this);
    super.dispose();
  }

  @override
  void onWindowClose() async {
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
      if (next.value == 'EngineReady') {
        resyncEngineStateFromWidgetRef(ref);
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
