import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/dashboard/screens/dashboard_screen.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
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
