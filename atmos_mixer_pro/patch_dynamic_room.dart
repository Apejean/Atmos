import 'dart:io';

void main() {
  final file = File('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart');
  String content = file.readAsStringSync();

  // Add import
  content = content.replaceFirst(
    'import "package:atmos_mixer_pro/features/exhibition/models/room_zone.dart";',
    'import "package:atmos_mixer_pro/features/exhibition/models/room_zone.dart";\nimport "package:atmos_mixer_pro/features/exhibition/state/three_js_engine_provider.dart";\nimport "dart:async";'
  );

  // Replace state variables
  content = content.replaceFirst(
    '''  HttpServer? _server;
  String? _serverUrl;
  WebViewController? _webViewController;
  bool _isEngineReady = false;
  String _selectedView = "Auto";''',
    '''  String _selectedView = "Auto";
  StreamSubscription<String>? _speakerSub;'''
  );

  // Replace initState
  content = content.replaceFirst(
    '''  @override
  void initState() {
    super.initState();
    _startLocalServer();
  }''',
    '''  @override
  void initState() {
    super.initState();
    _speakerSub = ref.read(threeJsEngineProvider).onSpeakerTapped.listen((id) {
      if (widget.onSpeakerTapped != null) {
        widget.onSpeakerTapped!(id);
      }
    });
  }'''
  );

  // Replace dispose
  content = content.replaceFirst(
    '''  @override
  void dispose() {
    _server?.close(force: true);
    super.dispose();
  }''',
    '''  @override
  void dispose() {
    _speakerSub?.cancel();
    super.dispose();
  }'''
  );

  // Remove server methods
  final startServerRegex = RegExp(r'Future<void> _startLocalServer\(\) async \{.*?\n  \}', dotAll: true);
  content = content.replaceFirst(startServerRegex, '');

  final initWebviewRegex = RegExp(r'void _initWebViewController\(\) \{.*?\n  \}', dotAll: true);
  content = content.replaceFirst(initWebviewRegex, '');

  // Replace didUpdateWidget
  content = content.replaceFirst(
    '''  @override
  void didUpdateWidget(Dynamic3DRoom oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_isEngineReady) {
      _syncSceneData();
    }
  }''',
    '''  @override
  void didUpdateWidget(Dynamic3DRoom oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (ref.read(threeJsEngineProvider).isEngineReady) {
      _syncSceneData();
    }
  }'''
  );

  // Replace _syncSceneData
  content = content.replaceFirst(
    '''  void _syncSceneData() {
    if (_webViewController == null || !_isEngineReady) return;''',
    '''  void _syncSceneData() {
    final engine = ref.read(threeJsEngineProvider);
    if (!engine.isEngineReady) return;'''
  );

  content = content.replaceFirst(
    '''_webViewController!.runJavaScript(jsCall);''',
    '''engine.executeJavaScript(jsCall);'''
  );

  // Replace _setCameraView
  content = content.replaceFirst(
    '''  void _setCameraView(String viewName) {
    setState(() => _selectedView = viewName);
    if (_webViewController != null && _isEngineReady) {
      _webViewController!.runJavaScript("if (typeof window.setCameraView === 'function') { window.setCameraView('\$viewName'); }");
    }
  }''',
    '''  void _setCameraView(String viewName) {
    setState(() => _selectedView = viewName);
    ref.read(threeJsEngineProvider).executeJavaScript("if (typeof window.setCameraView === 'function') { window.setCameraView('\$viewName'); }");
  }'''
  );

  // Replace build method Stack child 1
  final buildChildRegex = RegExp(r'// 1\. Core 3D WebGL Three\.js Studio Engine.*?Positioned\.fill\(.*?child: _webViewController == null.*?\)\n\s+\),', dotAll: true);
  content = content.replaceFirst(buildChildRegex, '''// 1. Core 3D WebGL Three.js Studio Engine
          Positioned.fill(
            child: Consumer(
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
            ),
          ),''');

  file.writeAsStringSync(content);
}
