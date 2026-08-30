import 'dart:io';

void main() {
  final file = File('lib/features/splash/screens/audio_init_splash_screen.dart');
  String content = file.readAsStringSync();

  // Add import
  content = content.replaceFirst(
    "import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';",
    "import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';\nimport 'package:atmos_mixer_pro/features/exhibition/state/three_js_engine_provider.dart';"
  );

  // Trigger Provider read
  content = content.replaceFirst(
    '''      // 2. Load EQ and delay settings from shared preferences
      await ref.read(tuningStateProvider.notifier).ensureLoaded();''',
    '''      // 2. Load EQ and delay settings from shared preferences
      await ref.read(tuningStateProvider.notifier).ensureLoaded();
      
      // Initialize 3D Engine in background
      ref.read(threeJsEngineProvider);'''
  );

  file.writeAsStringSync(content);
}
