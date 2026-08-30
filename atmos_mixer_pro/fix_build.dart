import 'dart:io';

void main() {
  final file = File('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart');
  String content = file.readAsStringSync();

  final regex = RegExp(r'child: _webViewController == null[\s\S]*?WebViewWidget\(controller: _webViewController!\),');
  
  content = content.replaceFirst(regex, '''child: Consumer(
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
            ),''');

  file.writeAsStringSync(content);
}
