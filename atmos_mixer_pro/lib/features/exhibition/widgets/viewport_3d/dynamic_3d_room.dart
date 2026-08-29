import "dart:convert";
import "dart:io";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:webview_flutter/webview_flutter.dart";

import "package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart";
import "package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart";
import "package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart";
import "package:atmos_mixer_pro/features/exhibition/models/room_zone.dart";

class Dynamic3DRoom extends ConsumerStatefulWidget {
  final Function(String)? onSpeakerTapped;
  final String? selectedSpeakerId;
  final RoomZone? activeRoom;
  final bool showHeatmap;
  final VoidCallback? onOpenRoomSetup;

  const Dynamic3DRoom({
    super.key,
    this.onSpeakerTapped,
    this.selectedSpeakerId,
    this.activeRoom,
    this.showHeatmap = false,
    this.onOpenRoomSetup,
  });

  @override
  ConsumerState<Dynamic3DRoom> createState() => _Dynamic3DRoomState();
}

class _Dynamic3DRoomState extends ConsumerState<Dynamic3DRoom> {
  HttpServer? _server;
  String? _serverUrl;
  WebViewController? _webViewController;
  bool _isEngineReady = false;
  String _selectedView = "Auto";

  @override
  void initState() {
    super.initState();
    _startLocalServer();
  }

  @override
  void dispose() {
    _server?.close(force: true);
    super.dispose();
  }

  Future<void> _startLocalServer() async {
    try {
      final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
      _server = server;
      final port = server.port;
      _serverUrl = "http://127.0.0.1:$port/";

      server.listen((HttpRequest request) async {
        final path = request.uri.path;
        final response = request.response;

        try {
          if (path == "/" || path == "/index.html") {
            final html = await rootBundle.loadString("assets/3d_simulator/studio_engine.html");
            response
              ..statusCode = HttpStatus.ok
              ..headers.contentType = ContentType.html
              ..write(html);
          } else if (path == "/js/three.min.js") {
            final data = await rootBundle.load("assets/js/three.min.js");
            response
              ..statusCode = HttpStatus.ok
              ..headers.set("Content-Type", "application/javascript")
              ..add(data.buffer.asUint8List());
          } else if (path == "/js/OrbitControls.js") {
            final data = await rootBundle.load("assets/js/OrbitControls.js");
            response
              ..statusCode = HttpStatus.ok
              ..headers.set("Content-Type", "application/javascript")
              ..add(data.buffer.asUint8List());
          } else {
            response
              ..statusCode = HttpStatus.notFound
              ..write("Not found");
          }
        } catch (e) {
          response
            ..statusCode = HttpStatus.internalServerError
            ..write("Error loading asset: $e");
        } finally {
          await response.close();
        }
      });

      _initWebViewController();
    } catch (e) {
      debugPrint("Error starting 3D local server: $e");
    }
  }

  void _initWebViewController() {
    if (_serverUrl == null) return;

    final controller = WebViewController();
    controller.setJavaScriptMode(JavaScriptMode.unrestricted);
    // controller.setBackgroundColor(const Color(0xFF0B0F14)); // REMOVED DUE TO OPAQUE BUG
    controller.addJavaScriptChannel(
        "SpeakerBridge",
        onMessageReceived: (message) {
          try {
            final data = jsonDecode(message.message);
            if (data["type"] == "SPEAKER_SELECTED" && data["speakerId"] != null) {
              final id = data["speakerId"] as String;
              if (widget.onSpeakerTapped != null) {
                widget.onSpeakerTapped!(id);
              }
            }
          } catch (e) {
            debugPrint("Error handling JS message: $e");
          }
        },
      );
    controller.setNavigationDelegate(
        NavigationDelegate(
          onPageFinished: (url) {
            if (mounted) {
              setState(() {
                _isEngineReady = true;
              });
              _syncSceneData();
            }
          },
        ),
      );
    controller.loadRequest(Uri.parse(_serverUrl!));

    if (mounted) {
      setState(() {
        _webViewController = controller;
      });
    }
  }

  @override
  void didUpdateWidget(Dynamic3DRoom oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_isEngineReady) {
      _syncSceneData();
    }
  }

  void _syncSceneData() {
    if (_webViewController == null || !_isEngineReady) return;

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

    final jsCall = "if (typeof window.updateScene === 'function') { window.updateScene(${jsonEncode(payload)}); }";
    _webViewController!.runJavaScript(jsCall);
  }

  void _setCameraView(String viewName) {
    setState(() => _selectedView = viewName);
    if (_webViewController != null && _isEngineReady) {
      _webViewController!.runJavaScript("if (typeof window.setCameraView === 'function') { window.setCameraView('$viewName'); }");
    }
  }

  @override
  Widget build(BuildContext context) {
    ref.listen(speakerLayoutProvider, (prev, next) {
      _syncSceneData();
    });

    final allSpeakers = ref.watch(speakerLayoutProvider);
    final speakers = allSpeakers.where((s) => s.roomId == null || s.roomId == widget.activeRoom?.id).toList();
    final bp = ref.watch(blueprintProvider);

    final roomWidth = widget.activeRoom?.physicalWidth ?? bp.canvasWidthMeters;
    final roomDepth = widget.activeRoom?.physicalHeight ?? bp.canvasHeightMeters;
    final roomHeight = widget.activeRoom?.ceilingHeight ?? 3.0;
    final roomLabel = widget.activeRoom?.label ?? "Room 1";

    return Scaffold(
      backgroundColor: const Color(0xFF0B0F14),
      body: Stack(
        children: [
          // 1. Core 3D WebGL Three.js Studio Engine
          Positioned.fill(
            child: _webViewController == null
                ? const Center(
                    child: CircularProgressIndicator(
                      valueColor: AlwaysStoppedAnimation<Color>(Colors.lightBlueAccent),
                    ),
                  )
                : WebViewWidget(controller: _webViewController!),
          ),

          // 2. Top-Left Room Info & Camera View Presets
          Positioned(
            top: 16,
            left: 16,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
              decoration: BoxDecoration(
                color: const Color(0xFF161E28).withValues(alpha: 0.90),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.lightBlueAccent.withValues(alpha: 0.3)),
                boxShadow: [
                  BoxShadow(
                    color: Colors.black.withValues(alpha: 0.4),
                    blurRadius: 10,
                  ),
                ],
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(Icons.view_in_ar_rounded, size: 16, color: Colors.lightBlueAccent),
                  const SizedBox(width: 8),
                  Text(
                    "$roomLabel: ${roomWidth.toStringAsFixed(1)}m × ${roomDepth.toStringAsFixed(1)}m × ${roomHeight.toStringAsFixed(1)}m | 3D Speakers: ${speakers.length}",
                    style: const TextStyle(
                      color: Colors.white70,
                      fontSize: 12,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 0),
                    height: 24,
                    decoration: BoxDecoration(
                      color: Colors.black.withValues(alpha: 0.4),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: DropdownButtonHideUnderline(
                      child: DropdownButton<String>(
                        value: _selectedView,
                        icon: const Icon(Icons.arrow_drop_down, color: Colors.lightBlueAccent, size: 16),
                        dropdownColor: const Color(0xFF1B232E),
                        style: const TextStyle(
                          color: Colors.lightBlueAccent,
                          fontSize: 11,
                          fontWeight: FontWeight.bold,
                        ),
                        items: const [
                          DropdownMenuItem(value: "Auto", child: Text("View: Orbit")),
                          DropdownMenuItem(value: "Front", child: Text("Front View")),
                          DropdownMenuItem(value: "Back", child: Text("Back View")),
                          DropdownMenuItem(value: "Side(L)", child: Text("Left View")),
                          DropdownMenuItem(value: "Side(R)", child: Text("Right View")),
                          DropdownMenuItem(value: "Top", child: Text("Top View")),
                        ],
                        onChanged: (val) {
                          if (val != null) {
                            _setCameraView(val);
                          }
                        },
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),

          // 3. Bottom-Right Floating Action Button: Add 3D Speaker
          Positioned(
            bottom: 24,
            right: widget.selectedSpeakerId != null ? 360 : 24,
            child: FloatingActionButton.extended(
              onPressed: () {
                final newId = "spk_${DateTime.now().millisecondsSinceEpoch}";
                final nextChannel = speakers.isEmpty
                    ? 1
                    : (speakers.map((s) => s.channel).reduce((a, b) => a > b ? a : b) + 1);

                final newNode = SpeakerNode(
                  id: newId,
                  roomId: widget.activeRoom?.id,
                  x: roomWidth * 0.25,
                  y: roomDepth * 0.25,
                  heightZ: 1.8,
                  channel: nextChannel,
                  pitchTilt: 15.0,
                  rotation: 45.0,
                  dispersionAngle: 90.0,
                );
                ref.read(speakerLayoutProvider.notifier).addSpeaker(newNode);
                if (widget.onSpeakerTapped != null) {
                  widget.onSpeakerTapped!(newId);
                }
              },
              backgroundColor: const Color(0xFF2563EB),
              foregroundColor: Colors.white,
              elevation: 6,
              icon: const Icon(Icons.add_rounded),
              label: const Text(
                "Add 3D Speaker",
                style: TextStyle(fontWeight: FontWeight.bold),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
