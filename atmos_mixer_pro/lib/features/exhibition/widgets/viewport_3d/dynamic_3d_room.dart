import "dart:convert";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:webview_flutter/webview_flutter.dart";

import "package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart";
import "package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart";
import "package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart";
import "package:atmos_mixer_pro/features/exhibition/models/room_zone.dart";
import "package:atmos_mixer_pro/features/exhibition/state/three_js_engine_provider.dart";
import "package:atmos_mixer_pro/features/exhibition/widgets/viewport_3d/environment_slider_widget.dart";
import "dart:async";

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
  String _selectedView = "Auto";
  StreamSubscription<String>? _speakerSub;
  StreamSubscription<Map<String, dynamic>>? _speakerMovedSub;

  @override
  void initState() {
    super.initState();
    _speakerSub = ref.read(threeJsEngineProvider).onSpeakerTapped.listen((id) {
      if (widget.onSpeakerTapped != null) {
        widget.onSpeakerTapped!(id);
      }
    });
    
    _speakerMovedSub = ref.read(threeJsEngineProvider).onSpeakerMoved.listen((data) {
      final String id = data['id'];
      final double x = (data['x'] as num).toDouble();
      final double y = (data['y'] as num).toDouble();
      final bool isFinal = data['isFinal'] ?? false;
      
      final currentNodes = ref.read(speakerLayoutProvider);
      final node = currentNodes.where((n) => n.id == id).firstOrNull;
      if (node != null && !node.isFixed) {
        ref.read(speakerLayoutProvider.notifier).updateSpeaker(
          node.copyWith(x: x, y: y),
          immediate: isFinal
        );
      }
    });
  }

  @override
  void dispose() {
    _speakerSub?.cancel();
    _speakerMovedSub?.cancel();
    super.dispose();
  }

  

  

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
    
    if (widget.activeRoom != null) {
      engine.setEarLevel(widget.activeRoom!.earLevel);
    }

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
        "earLevel": widget.activeRoom?.earLevel ?? 1.2,
      },
      "selectedSpeakerId": widget.selectedSpeakerId,
      "speakers": speakers.asMap().entries.map((entry) {
        final s = entry.value;
        return {
        "index": entry.key + 1,
        "id": s.id,
        "channel": s.channel,
        "x": s.x,
        "y": s.y,
        "z": s.heightZ,
        "pitchTilt": s.pitchTilt,
        "rotation": s.rotation,
        "dispersionAngle": s.dispersionAngle,
        "maxSPL": s.maxSPL,
        "isFixed": s.isFixed,
        };
      }).toList(),
    };

    final jsCall = "if (typeof window.updateScene === 'function') { window.updateScene(${jsonEncode(payload)}); } else { console.error('updateScene is not defined!'); }";
    engine.executeJavaScript(jsCall);
  }

  void _setCameraView(String viewName) {
    setState(() => _selectedView = viewName);
    ref.read(threeJsEngineProvider).executeJavaScript("if (typeof window.setCameraView === 'function') { window.setCameraView('$viewName'); }");
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
            child: Consumer(
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
            ),
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
                  const EnvironmentSliderWidget(),
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
                    ? 0
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
