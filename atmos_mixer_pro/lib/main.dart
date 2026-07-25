import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:window_manager/window_manager.dart';
import 'package:atmos_mixer_pro/src/rust/frb_generated.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart';

import 'package:atmos_mixer_pro/features/splash/screens/audio_init_splash_screen.dart';

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

class AtmosMixerProApp extends StatefulWidget {
  const AtmosMixerProApp({super.key});

  @override
  State<AtmosMixerProApp> createState() => _AtmosMixerProAppState();
}

class _AtmosMixerProAppState extends State<AtmosMixerProApp>
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
