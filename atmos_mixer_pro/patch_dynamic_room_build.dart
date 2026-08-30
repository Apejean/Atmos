import 'dart:io';

void main() {
  final file = File('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart');
  String content = file.readAsStringSync();

  content = content.replaceFirst(
    '''child: Consumer(
              builder: (context, ref, child) {
                final engine = ref.watch(threeJsEngineProvider);
                if (engine.controller == null || !engine.isEngineReady) {
                  return const Center(
                    child: CircularProgressIndicator(
                      valueColor: AlwaysStoppedAnimation<Color>(Colors.lightBlueAccent),
                    ),
                  );
                }
                
                // Immediately sync scene data on first display if ready
                WidgetsBinding.instance.addPostFrameCallback((_) {
                  if (mounted) _syncSceneData();
                });

                return WebViewWidget(controller: engine.controller!);
              },
            ),''',
    '''child: Consumer(
              builder: (context, ref, child) {
                final engine = ref.read(threeJsEngineProvider);
                
                return ValueListenableBuilder<bool>(
                  valueListenable: engine.isEngineReadyNotifier,
                  builder: (context, isReady, child) {
                    if (engine.controller == null || !isReady) {
                      return const Center(
                        child: CircularProgressIndicator(
                          valueColor: AlwaysStoppedAnimation<Color>(Colors.lightBlueAccent),
                        ),
                      );
                    }
                    
                    // Immediately sync scene data on first display if ready
                    WidgetsBinding.instance.addPostFrameCallback((_) {
                      if (mounted) _syncSceneData();
                    });

                    return WebViewWidget(controller: engine.controller!);
                  },
                );
              },
            ),'''
  );

  file.writeAsStringSync(content);
}
