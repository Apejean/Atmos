import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# Add import
content = content.replace(
    'import "package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart";',
    'import "package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart";\nimport "package:atmos_mixer_pro/features/exhibition/state/three_js_engine_provider.dart";'
)

# Replace the state variables
content = re.sub(
    r'class _Dynamic3DRoomState extends ConsumerState<Dynamic3DRoom> \{\s+HttpServer\? _server;\s+String\? _serverUrl;\s+WebViewController\? _webViewController;\s+bool _isEngineReady = false;\s+String _selectedView = "Auto";',
    'class _Dynamic3DRoomState extends ConsumerState<Dynamic3DRoom> {\n  String _selectedView = "Auto";\n  StreamSubscription<String>? _speakerSub;',
    content,
    flags=re.MULTILINE
)

# Replace initState and dispose
initState_dispose = """
  @override
  void initState() {
    super.initState();
    _speakerSub = ref.read(threeJsEngineProvider).onSpeakerTapped.listen((id) {
      if (widget.onSpeakerTapped != null) {
        widget.onSpeakerTapped!(id);
      }
    });
  }

  @override
  void dispose() {
    _speakerSub?.cancel();
    super.dispose();
  }
"""

content = re.sub(
    r'@override\s+void initState\(\) \{.*?\n\s+\}\s+@override\s+void dispose\(\) \{.*?\n\s+\}',
    initState_dispose,
    content,
    flags=re.DOTALL
)

# Remove _startLocalServer and _initWebViewController
content = re.sub(
    r'Future<void> _startLocalServer\(\) async \{.*?\n\s+\}\s+void _initWebViewController\(\) \{.*?\n\s+\}',
    '',
    content,
    flags=re.DOTALL
)

# Replace didUpdateWidget and _syncSceneData
sync_logic = """
  @override
  void didUpdateWidget(Dynamic3DRoom oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (ref.read(threeJsEngineProvider).isEngineReady) {
      _syncSceneData();
    }
  }

  void _syncSceneData() {
    final engine = ref.read(threeJsEngineProvider);
    if (!engine.isEngineReady) return;

    final allSpeakers = ref.read(speakerLayoutProvider);
    final speakers = allSpeakers.where((s) => s.roomId == null || s.roomId == widget.activeRoom?.id).toList();
    final bp = ref.read(blueprintProvider);

    final roomWidth = widget.activeRoom?.physicalWidth ?? bp.canvasWidthMeters;
    final roomDepth = widget.activeRoom?.physicalHeight ?? bp.canvasHeightMeters;
    final roomHeight = widget.activeRoom?.ceilingHeight ?? 3.0;

    final payload = {
      "room": {
        "width": roomWidth,
        "depth": roomDepth,
        "height": roomHeight,
      },
      "selectedSpeakerId": widget.selectedSpeakerId,
      "speakers": speakers.map((s) => {
        "id": s.id,
        "channel": s.channel,
        "x": s.x,
        "y": s.y,
        "z": s.heightZ,
        "pitchTilt": s.pitchTilt,
        "rotation": s.rotation,
        "dispersionAngle": s.dispersionAngle,
        "maxSPL": s.maxSPL,
      }).toList(),
    };

    final jsCall = "if (typeof window.updateScene === 'function') { window.updateScene(${jsonEncode(payload)}); } else { console.error('updateScene is not defined!'); }";
    engine.executeJavaScript(jsCall);
  }

  void _setCameraView(String viewName) {
    setState(() => _selectedView = viewName);
    ref.read(threeJsEngineProvider).executeJavaScript("if (typeof window.setCameraView === 'function') { window.setCameraView('$viewName'); }");
  }
"""

content = re.sub(
    r'@override\s+void didUpdateWidget\(Dynamic3DRoom oldWidget\) \{.*?\n\s+\}\s+void _syncSceneData\(\) \{.*?\n\s+\}\s+void _setCameraView\(String viewName\) \{.*?\n\s+\}',
    sync_logic,
    content,
    flags=re.DOTALL
)

# Update build method stack
build_logic = """
          // 1. Core 3D WebGL Three.js Studio Engine
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
          ),
"""

content = re.sub(
    r'// 1\. Core 3D WebGL Three\.js Studio Engine\s+Positioned\.fill\(\s+child: _webViewController == null.*?\s+\),\s+\),\s+// 2\. Top-Left Room Info & Camera View Presets',
    build_logic + '\n          // 2. Top-Left Room Info & Camera View Presets',
    content,
    flags=re.DOTALL
)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)

print("Patch applied.")
