import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/dashboard/screens/dashboard_screen.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/core/state/launch_mode.dart';
import 'package:atmos_mixer_pro/src/rust/api/show.dart' as show_api;
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/three_js_engine_provider.dart';

class AudioInitSplashScreen extends ConsumerStatefulWidget {
  const AudioInitSplashScreen({super.key});

  @override
  ConsumerState<AudioInitSplashScreen> createState() =>
      _AudioInitSplashScreenState();
}

class _AudioInitSplashScreenState extends ConsumerState<AudioInitSplashScreen> {
  String _statusMessage = 'Initializing Audio Engine...';
  bool _hasError = false;

  @override
  void initState() {
    super.initState();
    _initAudioSystem();
  }

  Future<void> _initAudioSystem() async {
    try {
      // 1. Ensure config is loaded first. We wait for config to be ready.
      await ref.read(configProvider.notifier).loadConfigAsync();

      // 2. Load EQ and delay settings from shared preferences
      await ref.read(tuningStateProvider.notifier).ensureLoaded();
      
      // Initialize 3D Engine in background
      ref.read(threeJsEngineProvider);

      if (mounted) {
        setState(() {
          _statusMessage = 'Loading large audio assets...';
        });
      }

      // 3. Wait for the audio engine to fully initialize before sending FFI commands
      // loadConfigAsync() already started the audio engine, we just need a safe delay
      if (!(await rust_api.apiIsEngineReady())) {
        try {
          await rust_api
              .apiCreateDeviceEventStream()
              .firstWhere((e) => e == 'EngineReady')
              .timeout(const Duration(milliseconds: 5000));
        } catch (e) {
          // Ignore
        }
      }

      // Apply tuning settings after engine starts
      ref.read(tuningStateProvider.notifier).applyAllToBackend();

      await _resumeShowIfLaunchedForIt();

      _navigateToDashboard();
    } catch (e) {
      if (mounted) {
        setState(() {
          _hasError = true;
          _statusMessage = 'Audio Initialization Failed:\n$e';
        });
      }
      // Optionally redirect to preferences even on error after a delay
      Future.delayed(const Duration(seconds: 3), _navigateToPreferences);
    }
  }

  /// 감시 프로그램이 다시 띄웠거나 로그인 자동 실행이면 공연을 이어 간다(rust api::show). 멈춘 위치부터
  /// 다시 틀고, 이어 갈 수 없으면 첫 방 테마로 시작한다. 사람이 켰으면 대기한다.
  Future<void> _resumeShowIfLaunchedForIt() async {
    final resume = shouldResumeShow(
      args: ref.read(launchArgsProvider),
      resumeOnLogon: await loadAutoResumeOnLogon(),
    );
    if (!resume) return;
    try {
      await show_api.apiResumeShow();
    } catch (e) {
      // 이어 가지 못해도 앱은 계속 쓴다(이유는 Rust가 앱 로그에 남긴다).
    }
  }

  void _navigateToDashboard() {
    if (!mounted) return;
    Navigator.of(context).pushReplacement(
      MaterialPageRoute(builder: (_) => const DashboardScreen()),
    );
  }

  void _navigateToPreferences() {
    if (!mounted) return;
    // We navigate to Dashboard but immediately open Settings Modal?
    // Or we have a dedicated Preferences route.
    // Usually Settings is a Modal over Dashboard. Let's just go to Dashboard and open Settings.
    Navigator.of(context).pushReplacement(
      MaterialPageRoute(
        builder: (_) {
          // We can pass a flag to Dashboard to open settings immediately.
          return const DashboardScreen(); // We'll need to adapt DashboardScreen to accept openSettingsOnInit
        },
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF1A1A1A),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Icon(Icons.graphic_eq, size: 64, color: Colors.blueAccent),
            const SizedBox(height: 24),
            const Text(
              'Atmos Mixer Pro',
              style: TextStyle(
                fontSize: 24,
                fontWeight: FontWeight.bold,
                color: Colors.white,
              ),
            ),
            const SizedBox(height: 16),
            if (!_hasError)
              const SizedBox(
                width: 32,
                height: 32,
                child: CircularProgressIndicator(
                  strokeWidth: 3,
                  color: Colors.blueAccent,
                ),
              ),
            const SizedBox(height: 16),
            Text(
              _statusMessage,
              style: TextStyle(
                fontSize: 14,
                color: _hasError ? Colors.redAccent : Colors.white70,
              ),
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }
}
