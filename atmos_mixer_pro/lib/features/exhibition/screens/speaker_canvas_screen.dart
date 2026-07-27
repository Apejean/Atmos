import 'dart:ui';
import 'dart:io';
import 'dart:async';
import 'dart:convert';
import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:file_picker/file_picker.dart';

import 'package:path_provider/path_provider.dart';
import 'package:uuid/uuid.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/speaker_node_widget.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/trajectory.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/trajectory_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart';

const double _gridSize = 50.0;
const double _canvasWidth = 2000.0;
const double _canvasHeight = 2000.0;
const double _speakerSize = 60.0;

class SpeakerCanvasScreen extends ConsumerStatefulWidget {
  const SpeakerCanvasScreen({super.key});

  @override
  ConsumerState<SpeakerCanvasScreen> createState() =>
      _SpeakerCanvasScreenState();
}

class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {
  final TransformationController _transformationController =
      TransformationController();
  Size _viewportSize = const Size(800, 600);

  static int _roomColorIndex = 0;

  Timer? _automationTimer;
  double _automationProgress = 0.0;
  bool _isPlayingAutomation = false;
  Offset? _currentAutomationPos;

  bool _showSpeakers = true;
  bool _showRooms = true;
  bool _showTrajectories = true;
  bool _showHeatmap = true;

  @override
  void initState() {
    super.initState();
    _transformationController.value = Matrix4.identity()
      ..setTranslationRaw(
        -_canvasWidth / 2 + 400,
        -_canvasHeight / 2 + 300,
        0.0,
      );
  }

  @override
  void dispose() {
    _automationTimer?.cancel();
    _transformationController.dispose();
    super.dispose();
  }

  DateTime? _lastSyncTime;

  void _syncSpatialConfigRealtime() {
    final now = DateTime.now();
    if (_lastSyncTime != null && now.difference(_lastSyncTime!).inMilliseconds < 16) {
      return;
    }
    _lastSyncTime = now;

    final nodes = ref.read(speakerLayoutProvider);
    final rooms = ref.read(roomZoneProvider);
    final trajectories = ref.read(trajectoryProvider);

    final payload = {
      'channel_positions': List.generate(64, (index) {
        final node = nodes.where((n) => n.channel == index).firstOrNull;
        if (node == null) return null;
        return {
          'x': node.x / _gridSize,
          'y': node.y / _gridSize,
          'z': 0.0,
        };
      }),
      'room_zones': rooms.map((r) {
        return {
          'room_id': r.id.hashCode.abs(),
          'boundary_min': {'x': r.x / _gridSize, 'y': r.y / _gridSize, 'z': 0.0},
          'boundary_max': {'x': (r.x + r.width) / _gridSize, 'y': (r.y + r.height) / _gridSize, 'z': 2.0},
        };
      }).toList(),
      'trajectory': trajectories.isNotEmpty && trajectories.first.waypoints.isNotEmpty ? {
        'waypoints': trajectories.first.waypoints.map((w) => {'x': w.x / _gridSize, 'y': w.y / _gridSize, 'z': 0.0}).toList(),
        'current_position': _currentAutomationPos != null 
            ? {'x': _currentAutomationPos!.dx / _gridSize, 'y': _currentAutomationPos!.dy / _gridSize, 'z': 0.0} 
            : {'x': trajectories.first.waypoints.first.x / _gridSize, 'y': trajectories.first.waypoints.first.y / _gridSize, 'z': 0.0},
      } : null,
    };

    apiUpdateSpatialConfigJson(jsonPayload: jsonEncode(payload)).catchError((e) {
      debugPrint('Real-time FFI sync error: $e');
    });
  }

  bool _automationReverse = false;

  void _toggleAutomation() {
    final trajectories = ref.read(trajectoryProvider);
    if (trajectories.isEmpty || trajectories.first.waypoints.isEmpty) return;

    if (_isPlayingAutomation) {
      setState(() {
        _isPlayingAutomation = false;
        _automationTimer?.cancel();
        _automationTimer = null;
      });
    } else {
      setState(() {
        _isPlayingAutomation = true;
      });
      final trajectory = trajectories.first;
      final waypoints = trajectory.waypoints;
      _automationTimer = Timer.periodic(const Duration(milliseconds: 16), (timer) {
        setState(() {
          double delta = 0.002 * trajectory.speed;
          if (_automationReverse) {
            _automationProgress -= delta;
          } else {
            _automationProgress += delta;
          }

          if (trajectory.isPingPong) {
            if (_automationProgress >= 1.0) {
              _automationProgress = 1.0;
              _automationReverse = true;
            } else if (_automationProgress <= 0.0) {
              _automationProgress = 0.0;
              _automationReverse = false;
            }
          } else {
            if (_automationProgress >= 1.0) _automationProgress = 0.0;
            if (_automationProgress <= 0.0) _automationProgress = 1.0;
            _automationReverse = false;
          }
          
          final totalPoints = waypoints.length;
          if (totalPoints == 1) {
            _currentAutomationPos = Offset(waypoints.first.x, waypoints.first.y);
          } else {
            final fIndex = _automationProgress * totalPoints;
            final iIndex = fIndex.floor() % totalPoints;
            final nextIndex = (iIndex + 1) % totalPoints;
            final t = fIndex - fIndex.floor();
            
            final w1 = waypoints[iIndex];
            final w2 = waypoints[nextIndex];
            _currentAutomationPos = Offset(
              w1.x + (w2.x - w1.x) * t,
              w1.y + (w2.y - w1.y) * t,
            );
          }
        });
        _syncSpatialConfigRealtime();
      });
    }
  }


  Future<void> _pickBlueprint() async {
    final result = await FilePicker.pickFiles(
      type: FileType.custom,
      allowedExtensions: ['png', 'jpg', 'jpeg', 'pdf'],
      withData: true,
    );
    if (result != null && result.files.single.bytes != null) {
      final ext = result.files.single.extension ?? 'png';
      final docDir = await getApplicationDocumentsDirectory();
      final targetFile = File('${docDir.path}/atmos_blueprint_cache.$ext');
      await targetFile.writeAsBytes(result.files.single.bytes!);
      
      ref.read(blueprintProvider.notifier).setBlueprint(targetFile.path);
    }
  }



  Offset _getCanvasCenter() {
    final centerMatrix = _transformationController.value.clone()..invert();
    final viewportCenter = Offset(
      _viewportSize.width / 2,
      _viewportSize.height / 2,
    );
    return MatrixUtils.transformPoint(centerMatrix, viewportCenter);
  }

  void _addSpeaker() {
    final canvasCenter = _getCanvasCenter();
    final scale = _transformationController.value.getMaxScaleOnAxis();
    final effectiveScale = scale > 0 ? scale : 1.0;
    
    double cx = (canvasCenter.dx / _gridSize).round() * _gridSize;
    double cy = (canvasCenter.dy / _gridSize).round() * _gridSize;
    
    // Offset for intuitive mouse drop
    cx -= (_speakerSize / 2) / effectiveScale;
    cy -= (_speakerSize / 2) / effectiveScale;
    
    cx = cx.clamp(0.0, _canvasWidth - _speakerSize);
    cy = cy.clamp(0.0, _canvasHeight - _speakerSize);

    final newNode = SpeakerNode(
      id: const Uuid().v4(),
      x: cx,
      y: cy,
      channel: 0,
    );
    ref.read(speakerLayoutProvider.notifier).addSpeaker(newNode);
  }

  void _addRoom() {
    final canvasCenter = _getCanvasCenter();
    final scale = _transformationController.value.getMaxScaleOnAxis();
    final effectiveScale = scale > 0 ? scale : 1.0;
    
    double cx = (canvasCenter.dx / _gridSize).round() * _gridSize;
    double cy = (canvasCenter.dy / _gridSize).round() * _gridSize;
    
    // Offset for intuitive mouse drop
    cx -= (300.0 / 2) / effectiveScale;
    cy -= (200.0 / 2) / effectiveScale;
    
    cx = cx.clamp(0.0, _canvasWidth - 300.0);
    cy = cy.clamp(0.0, _canvasHeight - 200.0);

    final colorValue = AppColors
        .roomAccents[_roomColorIndex % AppColors.roomAccents.length]
        .toARGB32();
    _roomColorIndex++;

    final newRoom = RoomZone(
      id: const Uuid().v4(),
      x: cx,
      y: cy,
      width: 300.0,
      height: 200.0,
      color: colorValue,
    );
    ref.read(roomZoneProvider.notifier).addRoomZone(newRoom);
  }

  void _addTrajectory() {
    final canvasCenter = _getCanvasCenter();
    double cx = canvasCenter.dx.clamp(100.0, _canvasWidth - 100.0);
    double cy = canvasCenter.dy.clamp(100.0, _canvasHeight - 100.0);

    final t = Trajectory(
      id: const Uuid().v4(),
      waypoints: [
        TrajectoryWaypoint(cx - 100, cy),
        TrajectoryWaypoint(cx + 100, cy),
      ],
    );
    ref.read(trajectoryProvider.notifier).addTrajectory(t);
  }

  void _clearTrajectories() {
    ref.read(trajectoryProvider.notifier).clearAll();
    setState(() {
      _isPlayingAutomation = false;
      _automationTimer?.cancel();
      _automationTimer = null;
      _currentAutomationPos = null;
    });
    _syncSpatialConfigRealtime();
  }

  void _clearCanvas() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        backgroundColor: AppColors.cardSurface,
        title: const Text('Clear Canvas', style: TextStyle(color: Colors.white)),
        content: const Text('Are you sure you want to delete all speakers, rooms, and trajectories?', style: TextStyle(color: Colors.white70)),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel', style: TextStyle(color: Colors.white70)),
          ),
          TextButton(
            onPressed: () {
              ref.read(speakerLayoutProvider.notifier).clearAll();
              ref.read(roomZoneProvider.notifier).clearAll();
              _clearTrajectories();
              ref.read(blueprintProvider.notifier).clearBlueprint();
              Navigator.pop(context);
            },
            child: const Text('Clear All', style: TextStyle(color: Colors.redAccent)),
          ),
        ],
      ),
    );
  }

  Future<void> _editRoom(RoomZone room) async {
    final blueprint = ref.read(blueprintProvider);
    final nameController = TextEditingController(text: room.label);
    final widthController = TextEditingController(text: room.physicalWidth.toStringAsFixed(1));
    final heightController = TextEditingController(text: room.physicalHeight.toStringAsFixed(1));
    int selectedColor = room.color;
    bool hasDoor = room.hasDoor;
    int doorWall = room.doorWall;
    double doorOffset = room.doorOffset;
    double rotation = room.rotation;

    await showDialog(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) => AlertDialog(
          backgroundColor: AppColors.cardSurface,
          title: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text('Edit Room Settings', style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
              IconButton(
                icon: const Icon(Icons.close, color: Colors.white54),
                onPressed: () => Navigator.pop(context),
              ),
            ],
          ),
          content: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                TextField(
                  controller: nameController,
                  style: const TextStyle(color: Colors.white),
                  decoration: const InputDecoration(
                    labelText: 'Room Name',
                    labelStyle: TextStyle(color: Colors.white70),
                    enabledBorder: UnderlineInputBorder(borderSide: BorderSide(color: Colors.white24)),
                    focusedBorder: UnderlineInputBorder(borderSide: BorderSide(color: AppColors.primaryNeon)),
                  ),
                ),
                const SizedBox(height: 16),
                Row(
                  children: [
                    Expanded(
                      child: TextField(
                        controller: widthController,
                        style: const TextStyle(color: Colors.white),
                        keyboardType: const TextInputType.numberWithOptions(decimal: true),
                        decoration: const InputDecoration(
                          labelText: 'Width (m)',
                          labelStyle: TextStyle(color: Colors.white70),
                          enabledBorder: UnderlineInputBorder(borderSide: BorderSide(color: Colors.white24)),
                          focusedBorder: UnderlineInputBorder(borderSide: BorderSide(color: AppColors.primaryNeon)),
                        ),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: TextField(
                        controller: heightController,
                        style: const TextStyle(color: Colors.white),
                        keyboardType: const TextInputType.numberWithOptions(decimal: true),
                        decoration: const InputDecoration(
                          labelText: 'Height (m)',
                          labelStyle: TextStyle(color: Colors.white70),
                          enabledBorder: UnderlineInputBorder(borderSide: BorderSide(color: Colors.white24)),
                          focusedBorder: UnderlineInputBorder(borderSide: BorderSide(color: AppColors.primaryNeon)),
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 20),
                const Text('Theme Color', style: TextStyle(color: Colors.white70, fontSize: 13)),
                const SizedBox(height: 10),
                Wrap(
                  spacing: 10,
                  runSpacing: 10,
                  children: AppColors.roomAccents.map((color) {
                    final isSelected = color.toARGB32() == selectedColor;
                    return GestureDetector(
                      onTap: () => setDialogState(() => selectedColor = color.toARGB32()),
                      child: Container(
                        width: 36,
                        height: 36,
                        decoration: BoxDecoration(
                          color: color,
                          shape: BoxShape.circle,
                          border: Border.all(
                            color: isSelected ? Colors.white : Colors.transparent,
                            width: 3,
                          ),
                        ),
                      ),
                    );
                  }).toList(),
                ),
                const SizedBox(height: 20),
                const Divider(color: Colors.white24),
                const SizedBox(height: 8),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text('Rotation: ${rotation.toInt()}°', style: const TextStyle(color: Colors.white, fontWeight: FontWeight.w600)),
                  ],
                ),
                Slider(
                  value: rotation,
                  min: 0.0,
                  max: 360.0,
                  activeColor: AppColors.primaryNeon,
                  onChanged: (val) => setDialogState(() => rotation = val),
                ),
                const SizedBox(height: 20),
                const Divider(color: Colors.white24),
                const SizedBox(height: 8),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    const Text('CAD Entrance Marker (Door)', style: TextStyle(color: Colors.white, fontWeight: FontWeight.w600)),
                    Switch(
                      value: hasDoor,
                      activeTrackColor: AppColors.primaryNeon,
                      onChanged: (val) => setDialogState(() => hasDoor = val),
                    ),
                  ],
                ),
                if (hasDoor) ...[
                  const SizedBox(height: 12),
                  Row(
                    children: [
                      const Text('Door Wall: ', style: TextStyle(color: Colors.white70, fontSize: 13)),
                      const SizedBox(width: 12),
                      Expanded(
                        child: DropdownButton<int>(
                          value: doorWall,
                          dropdownColor: AppColors.cardSurface,
                          isExpanded: true,
                          style: const TextStyle(color: Colors.white),
                          items: const [
                            DropdownMenuItem(value: 0, child: Text('Top Wall')),
                            DropdownMenuItem(value: 1, child: Text('Right Wall')),
                            DropdownMenuItem(value: 2, child: Text('Bottom Wall')),
                            DropdownMenuItem(value: 3, child: Text('Left Wall')),
                          ],
                          onChanged: (val) {
                            if (val != null) setDialogState(() => doorWall = val);
                          },
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  Text('Door Position: ${(doorOffset * 100).toInt()}%', style: const TextStyle(color: Colors.white70, fontSize: 13)),
                  Slider(
                    value: doorOffset,
                    min: 0.0,
                    max: 1.0,
                    activeColor: AppColors.primaryNeon,
                    onChanged: (val) => setDialogState(() => doorOffset = val),
                  ),
                ],
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () {
                ref.read(roomZoneProvider.notifier).removeRoomZone(room.id);
                Navigator.pop(context);
              },
              child: const Text('Delete Room', style: TextStyle(color: Colors.redAccent)),
            ),
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancel', style: TextStyle(color: Colors.white70)),
            ),
            ElevatedButton(
              style: ElevatedButton.styleFrom(
                backgroundColor: AppColors.primaryNeon,
                foregroundColor: Colors.black,
              ),
              onPressed: () {
                final newWidthM = double.tryParse(widthController.text) ?? room.physicalWidth;
                final newHeightM = double.tryParse(heightController.text) ?? room.physicalHeight;
                final scale = blueprint.scale > 0 ? blueprint.scale : 40.0;
                
                ref.read(roomZoneProvider.notifier).updateRoomZone(
                      room.copyWith(
                        label: nameController.text.trim().isNotEmpty ? nameController.text.trim() : room.label,
                        color: selectedColor,
                        physicalWidth: newWidthM,
                        physicalHeight: newHeightM,
                        width: newWidthM * scale,
                        height: newHeightM * scale,
                        hasDoor: hasDoor,
                        doorWall: doorWall,
                        doorOffset: doorOffset,
                        rotation: rotation,
                      ),
                      immediate: true,
                    );
                Navigator.pop(context);
              },
              child: const Text('Save Changes', style: TextStyle(fontWeight: FontWeight.bold)),
            ),
          ],
        ),
      ),
    );
  }

  List<SpeakerNode> _getSpeakersInRoom(RoomZone room, List<SpeakerNode> nodes) {
    return nodes
        .where((n) => room.containsPoint(n.x + _speakerSize / 2, n.y + _speakerSize / 2))
        .toList();
  }

  Color? _getRoomColorForSpeaker(SpeakerNode speaker, List<RoomZone> rooms) {
    for (final room in rooms) {
      if (room.containsPoint(speaker.x + _speakerSize / 2, speaker.y + _speakerSize / 2)) {
        return Color(room.color);
      }
    }
    return null;
  }

  Widget _buildLayerToggle(String tooltip, IconData icon, bool isActive, VoidCallback onTap) {
    return Tooltip(
      message: tooltip,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(8),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          decoration: BoxDecoration(
            color: isActive ? AppColors.primaryNeon.withValues(alpha: 0.2) : Colors.transparent,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: isActive ? AppColors.primaryNeon : Colors.white24,
            ),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, color: isActive ? AppColors.primaryNeon : Colors.white54, size: 18),
              const SizedBox(width: 6),
              Text(tooltip.split(' ')[0], style: TextStyle(color: isActive ? AppColors.primaryNeon : Colors.white54, fontSize: 12, fontWeight: FontWeight.bold)),
            ],
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final blueprint = ref.watch(blueprintProvider);

    return Scaffold(
      backgroundColor: AppColors.background,
      appBar: AppBar(
        title: Row(
          children: [
            const Text('Exhibition Canvas'),
            const Spacer(),
            Consumer(
              builder: (context, ref, child) {
                final isMasterMuted = ref.watch(
                  engineStateProvider.select((state) => state.masterMuteActive),
                );
                if (!isMasterMuted) return const SizedBox.shrink();
                return RepaintBoundary(
                  child: ClipRRect(
                    borderRadius: BorderRadius.circular(4),
                    child: BackdropFilter(
                      filter: ImageFilter.blur(sigmaX: 10, sigmaY: 10),
                      child: Container(
                        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
                        decoration: BoxDecoration(
                          color: Colors.orange.shade800.withValues(alpha: 0.8),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: const Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(Icons.warning_amber_rounded, color: Colors.white, size: 16),
                            SizedBox(width: 8),
                            Text(
                              'MASTER MUTE ACTIVE',
                              style: TextStyle(
                                color: Colors.white,
                                fontWeight: FontWeight.bold,
                                fontSize: 12,
                                letterSpacing: 1.0,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                );
              },
            ),
            const SizedBox(width: 16),
            IconButton(
              tooltip: _isPlayingAutomation ? 'Stop Automation' : 'Play Automation',
              icon: Icon(_isPlayingAutomation ? Icons.stop : Icons.play_arrow, color: _isPlayingAutomation ? Colors.redAccent : AppColors.primaryNeon),
              onPressed: _toggleAutomation,
            ),
            const SizedBox(width: 8),
            IconButton(
              tooltip: 'Set Blueprint',
              icon: const Icon(Icons.image, color: Colors.white70),
              onPressed: _pickBlueprint,
            ),
            const SizedBox(width: 8),
            IconButton(
              tooltip: 'Set Scale',
              icon: const Icon(Icons.aspect_ratio, color: Colors.white70),
              onPressed: () {
                showDialog(
                  context: context,
                  builder: (context) {
                    double currentScale = blueprint.scale;
                    return AlertDialog(
                      backgroundColor: Colors.grey.shade900,
                      title: const Text('Set Physical Scale', style: TextStyle(color: Colors.white)),
                      content: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          const Text('Pixels per Meter', style: TextStyle(color: Colors.white70)),
                          const SizedBox(height: 16),
                          StatefulBuilder(
                            builder: (context, setDialogState) {
                              return Row(
                                children: [
                                  Text('${currentScale.toInt()} px/m', style: const TextStyle(color: Colors.white)),
                                  Expanded(
                                    child: Slider(
                                      value: currentScale,
                                      min: 10.0,
                                      max: 200.0,
                                      activeColor: AppColors.primaryNeon,
                                      onChanged: (val) {
                                        setDialogState(() {
                                          currentScale = val;
                                        });
                                        ref.read(blueprintProvider.notifier).setScale(val);
                                      },
                                    ),
                                  ),
                                ],
                              );
                            },
                          ),
                        ],
                      ),
                      actions: [
                        TextButton(
                          onPressed: () => Navigator.pop(context),
                          child: const Text('Close', style: TextStyle(color: AppColors.primaryNeon)),
                        ),
                      ],
                    );
                  },
                );
              },
            ),
            const SizedBox(width: 8),
            Container(
              height: 24,
              width: 1,
              color: Colors.white24,
            ),
            const SizedBox(width: 8),
              Wrap(
                spacing: 8,
                children: [
                  _buildLayerToggle('Speaker Layer', Icons.speaker, _showSpeakers, () {
                    setState(() => _showSpeakers = !_showSpeakers);
                  }),
                  _buildLayerToggle('Room Layer', Icons.meeting_room, _showRooms, () {
                    setState(() => _showRooms = !_showRooms);
                  }),
                  _buildLayerToggle('Trajectory Layer', Icons.route, _showTrajectories, () {
                    setState(() {
                      _showTrajectories = !_showTrajectories;
                      if (!_showTrajectories && _isPlayingAutomation) {
                        _toggleAutomation(); // Pauses if playing
                      }
                    });
                  }),
                  _buildLayerToggle('Heatmap Layer', Icons.wb_sunny, _showHeatmap, () {
                    setState(() => _showHeatmap = !_showHeatmap);
                  }),
                ],
              ),
            const SizedBox(width: 8),
            Container(
              height: 24,
              width: 1,
              color: Colors.white24,
            ),
            const SizedBox(width: 8),
            IconButton(
              icon: const Icon(Icons.route_outlined, color: Colors.orangeAccent),
              onPressed: _clearTrajectories,
              tooltip: 'Clear Trajectories',
            ),
            IconButton(
              icon: const Icon(Icons.delete_sweep, color: Colors.redAccent),
              onPressed: _clearCanvas,
              tooltip: 'Clear Canvas',
            ),
          ],
        ),
        backgroundColor: Colors.black,
      ),
      body: LayoutBuilder(
        builder: (context, constraints) {
          _viewportSize = Size(constraints.maxWidth, constraints.maxHeight);
          return InteractiveViewer(
            transformationController: _transformationController,
            boundaryMargin: const EdgeInsets.all(double.infinity),
            minScale: 0.1,
            maxScale: 2.0,
            constrained: false,
            child: SizedBox(
              width: _canvasWidth,
              height: _canvasHeight,
              child: Stack(
                clipBehavior: Clip.none,
                children: [
                  Positioned.fill(
                    child: RepaintBoundary(
                      child: CustomPaint(painter: _GridPainter(blueprint.scale)),
                    ),
                  ),
                  if (blueprint.imagePath != null)
                    Positioned.fill(
                      child: Image.file(
                        File(blueprint.imagePath!),
                        fit: BoxFit.contain,
                        color: Colors.white.withValues(alpha: blueprint.opacity),
                        colorBlendMode: BlendMode.modulate,
                      ),
                    ),
                  if (_showHeatmap)
                    Consumer(
                      builder: (context, ref, _) {
                        final nodes = ref.watch(speakerLayoutProvider);
                        return Positioned.fill(
                          child: RepaintBoundary(
                            child: CustomPaint(
                              painter: _HeatmapPainter(nodes),
                            ),
                          ),
                        );
                      },
                    ),
                  if (_showRooms)
                    Consumer(
                      builder: (context, ref, _) {
                        final rooms = ref.watch(roomZoneProvider);
                        final nodes = ref.watch(speakerLayoutProvider);
                        return Stack(
                          clipBehavior: Clip.none,
                          children: rooms.map((room) {
                            final containedSpeakers = _getSpeakersInRoom(room, nodes);
                            return _DraggableRoomWidget(
                              key: ValueKey(room.id),
                              room: room,
                              containedSpeakers: containedSpeakers,
                              transformationController: _transformationController,
                              onEdit: () => _editRoom(room),
                              onDragUpdate: _syncSpatialConfigRealtime,
                            );
                          }).toList(),
                        );
                      },
                    ),
                  if (_showTrajectories)
                    Consumer(
                    builder: (context, ref, _) {
                      final trajectories = ref.watch(trajectoryProvider);
                      return Stack(
                        children: [
                          Positioned.fill(
                            child: RepaintBoundary(
                              child: CustomPaint(
                                painter: _TrajectoryPainter(trajectories),
                              ),
                            ),
                          ),
                          ...trajectories.expand((t) {
                            return t.waypoints.asMap().entries.map((entry) {
                              final idx = entry.key;
                              final wp = entry.value;
                              return _DraggableWaypointWidget(
                                key: ValueKey('${t.id}_$idx'),
                                trajectory: t,
                                waypointIndex: idx,
                                waypoint: wp,
                                transformationController: _transformationController,
                                onDragUpdate: _syncSpatialConfigRealtime,
                              );
                            });
                          }),
                        ],
                      );
                    },
                  ),
                  if (_showSpeakers)
                    Consumer(
                      builder: (context, ref, _) {
                        final nodes = ref.watch(speakerLayoutProvider);
                      final rooms = ref.watch(roomZoneProvider);
                      return Stack(
                        clipBehavior: Clip.none,
                        children: nodes.map((node) {
                          final isDuplicate = nodes
                              .where((n) => n.id != node.id && n.channel == node.channel)
                              .isNotEmpty;
                          final roomColor = _getRoomColorForSpeaker(node, rooms);
                          return _DraggableSpeakerWidget(
                            key: ValueKey(node.id),
                            node: node,
                            roomColor: roomColor ?? AppColors.primaryNeon,
                            isDuplicate: isDuplicate,
                            transformationController: _transformationController,
                          );
                        }).toList(),
                      );
                    },
                  ),
                ],
              ),
            ),
          );
        },
      ),
      floatingActionButton: PopupMenuButton<String>(
        onSelected: (value) {
          if (value == 'speaker') { _addSpeaker(); }
          else if (value == 'room') { _addRoom(); }
          else if (value == 'trajectory') { _addTrajectory(); }
        },
        itemBuilder: (BuildContext context) => [
          const PopupMenuItem(value: 'speaker', child: Text('Add Speaker')),
          const PopupMenuItem(value: 'room', child: Text('Add Room')),
          const PopupMenuItem(value: 'trajectory', child: Text('Add Trajectory')),
        ],
        child: FloatingActionButton(
          onPressed: null,
          backgroundColor: AppColors.primaryNeon,
          child: const Icon(Icons.add, color: Colors.black),
        ),
      ),
    );
  }
}

class _DraggableRoomWidget extends ConsumerStatefulWidget {
  final RoomZone room;
  final List<SpeakerNode> containedSpeakers;
  final TransformationController transformationController;
  final VoidCallback onEdit;
  final VoidCallback? onDragUpdate;

  const _DraggableRoomWidget({
    super.key,
    required this.room,
    required this.containedSpeakers,
    required this.transformationController,
    required this.onEdit,
    this.onDragUpdate,
  });

  @override
  ConsumerState<_DraggableRoomWidget> createState() => _DraggableRoomWidgetState();
}

class _DraggableRoomWidgetState extends ConsumerState<_DraggableRoomWidget> {
  late double _localX;
  late double _localY;
  late double _localW;
  late double _localH;
  bool _isInteracting = false;
  Map<String, Offset> _draggedSpeakersOffsets = {};

  @override
  void initState() {
    super.initState();
    _localX = widget.room.x;
    _localY = widget.room.y;
    _localW = widget.room.width;
    _localH = widget.room.height;
  }

  @override
  void didUpdateWidget(covariant _DraggableRoomWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!_isInteracting) {
      _localX = widget.room.x;
      _localY = widget.room.y;
      _localW = widget.room.width;
      _localH = widget.room.height;
    }
  }

  Widget _buildResizeHandle(Alignment alignment) {
    return Align(
      alignment: alignment,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onPanStart: (_) {
          setState(() => _isInteracting = true);
        },
        onPanUpdate: (details) {
          final scale = widget.transformationController.value.getMaxScaleOnAxis();
          final currentScale = scale > 0 ? scale : 1.0;
          final dxGlobal = details.delta.dx / currentScale;
          final dyGlobal = details.delta.dy / currentScale;

          // Convert global gesture delta to room's rotated local coordinate system
          final rad = widget.room.rotation * math.pi / 180.0;
          final cosA = math.cos(-rad);
          final sinA = math.sin(-rad);
          final dx = dxGlobal * cosA - dyGlobal * sinA;
          final dy = dxGlobal * sinA + dyGlobal * cosA;

          setState(() {
            if (alignment.x < 0) {
              _localX += dx;
              _localW -= dx;
              if (_localW < 80) {
                _localX -= (80 - _localW);
                _localW = 80;
              }
            } else if (alignment.x > 0) {
              _localW += dx;
              if (_localW < 80) _localW = 80;
            }

            if (alignment.y < 0) {
              _localY += dy;
              _localH -= dy;
              if (_localH < 80) {
                _localY -= (80 - _localH);
                _localH = 80;
              }
            } else if (alignment.y > 0) {
              _localH += dy;
              if (_localH < 80) _localH = 80;
            }

            _localX = _localX.clamp(0.0, _canvasWidth - _localW);
            _localY = _localY.clamp(0.0, _canvasHeight - _localH);
            _localW = _localW.clamp(80.0, _canvasWidth - _localX);
            _localH = _localH.clamp(80.0, _canvasHeight - _localY);
          });

          final blueprint = ref.read(blueprintProvider);
          final scaleM = blueprint.scale > 0 ? blueprint.scale : 40.0;
          ref.read(roomZoneProvider.notifier).updateRoomZone(
                widget.room.copyWith(
                  x: _localX,
                  y: _localY,
                  width: _localW,
                  height: _localH,
                  physicalWidth: _localW / scaleM,
                  physicalHeight: _localH / scaleM,
                ),
                immediate: true,
              );
          widget.onDragUpdate?.call();
        },
        onPanEnd: (_) {
          double snappedX = (_localX / _gridSize).round() * _gridSize;
          double snappedY = (_localY / _gridSize).round() * _gridSize;
          double snappedW = (_localW / _gridSize).round() * _gridSize;
          double snappedH = (_localH / _gridSize).round() * _gridSize;

          snappedX = snappedX.clamp(0.0, _canvasWidth - snappedW);
          snappedY = snappedY.clamp(0.0, _canvasHeight - snappedH);
          snappedW = snappedW.clamp(80.0, _canvasWidth - snappedX);
          snappedH = snappedH.clamp(80.0, _canvasHeight - snappedY);

          final blueprint = ref.read(blueprintProvider);
          final scaleM = blueprint.scale > 0 ? blueprint.scale : 40.0;

          setState(() {
            _localX = snappedX;
            _localY = snappedY;
            _localW = snappedW;
            _localH = snappedH;
            _isInteracting = false;
          });

          ref.read(roomZoneProvider.notifier).updateRoomZone(
                widget.room.copyWith(
                  x: snappedX,
                  y: snappedY,
                  width: snappedW,
                  height: snappedH,
                  physicalWidth: snappedW / scaleM,
                  physicalHeight: snappedH / scaleM,
                ),
                immediate: true,
              );
          widget.onDragUpdate?.call();
        },
        child: Container(
          width: 28,
          height: 28,
          color: Colors.transparent,
          child: Center(
            child: Container(
              width: 12,
              height: 12,
              decoration: BoxDecoration(
                color: AppColors.primaryNeon,
                border: Border.all(color: Colors.black, width: 2),
                shape: BoxShape.circle,
                boxShadow: const [
                  BoxShadow(color: Colors.black54, blurRadius: 4),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildDoorHandle() {
    if (!widget.room.hasDoor) return const SizedBox.shrink();

    double left = 0, top = 0;
    final double doorWidth = 44.0;
    
    if (widget.room.doorWall == 0) {
      left = (widget.room.doorOffset * _localW - doorWidth / 2).clamp(0.0, _localW - doorWidth);
      top = -doorWidth / 2;
    } else if (widget.room.doorWall == 1) {
      left = _localW - doorWidth / 2;
      top = (widget.room.doorOffset * _localH - doorWidth / 2).clamp(0.0, _localH - doorWidth);
    } else if (widget.room.doorWall == 2) {
      left = (widget.room.doorOffset * _localW - doorWidth / 2).clamp(0.0, _localW - doorWidth);
      top = _localH - doorWidth / 2;
    } else if (widget.room.doorWall == 3) {
      left = -doorWidth / 2;
      top = (widget.room.doorOffset * _localH - doorWidth / 2).clamp(0.0, _localH - doorWidth);
    }

    return Positioned(
      left: left,
      top: top,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onPanStart: (_) => setState(() => _isInteracting = true),
        onPanUpdate: (details) {
          final scale = widget.transformationController.value.getMaxScaleOnAxis();
          final currentScale = scale > 0 ? scale : 1.0;
          final dx = details.delta.dx / currentScale;
          final dy = details.delta.dy / currentScale;
          
          double newOffset = widget.room.doorOffset;
          if (widget.room.doorWall == 0 || widget.room.doorWall == 2) {
            newOffset += dx / (_localW > 0 ? _localW : 1.0);
          } else {
            newOffset += dy / (_localH > 0 ? _localH : 1.0);
          }
          newOffset = newOffset.clamp(0.0, 1.0);
          
          ref.read(roomZoneProvider.notifier).updateRoomZone(
            widget.room.copyWith(doorOffset: newOffset),
            immediate: true,
          );
        },
        onPanEnd: (_) => setState(() => _isInteracting = false),
        child: Container(
          width: doorWidth,
          height: doorWidth,
          color: Colors.transparent,
          child: CustomPaint(
            painter: _CadDoorPainter(
              color: Colors.cyanAccent,
              wall: widget.room.doorWall,
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildRotateHandle() {
    return Positioned(
      top: -36,
      left: _localW / 2 - 14,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onPanStart: (_) => setState(() => _isInteracting = true),
        onPanUpdate: (details) {
          final renderBox = context.findRenderObject() as RenderBox?;
          if (renderBox != null) {
            final localTouch = renderBox.globalToLocal(details.globalPosition);
            final roomCenterLocal = Offset(_localW / 2, _localH / 2);
            final dx = localTouch.dx - roomCenterLocal.dx;
            final dy = localTouch.dy - roomCenterLocal.dy;
            double angleRad = math.atan2(dy, dx) + math.pi / 2;
            double angleDeg = (angleRad * 180 / math.pi) % 360;
            if (angleDeg < 0) angleDeg += 360;

            ref.read(roomZoneProvider.notifier).updateRoomZone(
                  widget.room.copyWith(rotation: angleDeg),
                  immediate: true,
                );
          }
        },
        onPanEnd: (_) => setState(() => _isInteracting = false),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 22,
              height: 20,
              decoration: BoxDecoration(
                color: AppColors.primaryNeon,
                shape: BoxShape.circle,
                border: Border.all(color: Colors.black, width: 2),
                boxShadow: const [BoxShadow(color: Colors.black54, blurRadius: 4)],
              ),
              child: const Icon(Icons.rotate_right, size: 12, color: Colors.black),
            ),
            Container(width: 2, height: 16, color: AppColors.primaryNeon),
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final roomColor = Color(widget.room.color);
    return Positioned(
      left: _localX,
      top: _localY,
      width: _localW,
      height: _localH,
      child: Transform.rotate(
        angle: widget.room.rotation * math.pi / 180.0,
        child: Stack(
          clipBehavior: Clip.none,
          children: [
            RepaintBoundary(
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: widget.onEdit,
                onPanStart: (details) {
                  setState(() => _isInteracting = true);
                  _draggedSpeakersOffsets = {
                    for (var s in widget.containedSpeakers) s.id: Offset(s.x, s.y)
                  };
                },
                onPanUpdate: (details) {
                  final scale = widget.transformationController.value.getMaxScaleOnAxis();
                  final currentScale = scale > 0 ? scale : 1.0;
                  final dx = details.delta.dx / currentScale;
                  final dy = details.delta.dy / currentScale;

                  setState(() {
                    _localX = (_localX + dx).clamp(0.0, _canvasWidth - _localW);
                    _localY = (_localY + dy).clamp(0.0, _canvasHeight - _localH);

                    final currentNodes = ref.read(speakerLayoutProvider);
                    for (final nodeId in _draggedSpeakersOffsets.keys) {
                      final prev = _draggedSpeakersOffsets[nodeId]!;
                      final nx = (prev.dx + dx).clamp(0.0, _canvasWidth - _speakerSize);
                      final ny = (prev.dy + dy).clamp(0.0, _canvasHeight - _speakerSize);
                      _draggedSpeakersOffsets[nodeId] = Offset(nx, ny);
                      try {
                        final node = currentNodes.firstWhere((n) => n.id == nodeId);
                        ref.read(speakerLayoutProvider.notifier).updateSpeaker(node.copyWith(x: nx, y: ny));
                      } catch (_) {}
                    }
                  });
                  ref.read(roomZoneProvider.notifier).updateRoomZone(
                        widget.room.copyWith(x: _localX, y: _localY),
                        immediate: true,
                      );
                  widget.onDragUpdate?.call();
                },
                onPanEnd: (details) {
                  double snappedX = (_localX / _gridSize).round() * _gridSize;
                  double snappedY = (_localY / _gridSize).round() * _gridSize;
                  snappedX = snappedX.clamp(0.0, _canvasWidth - _localW);
                  snappedY = snappedY.clamp(0.0, _canvasHeight - _localH);

                  setState(() {
                    _localX = snappedX;
                    _localY = snappedY;
                    _isInteracting = false;
                  });

                  ref.read(roomZoneProvider.notifier).updateRoomZone(
                        widget.room.copyWith(x: snappedX, y: snappedY),
                        immediate: true,
                      );

                  final currentNodes = ref.read(speakerLayoutProvider);
                  for (final nodeId in _draggedSpeakersOffsets.keys) {
                    try {
                      final node = currentNodes.firstWhere((n) => n.id == nodeId);
                      final nX = (node.x / _gridSize).round() * _gridSize;
                      final nY = (node.y / _gridSize).round() * _gridSize;
                      ref.read(speakerLayoutProvider.notifier).updateSpeaker(
                            node.copyWith(
                              x: nX.clamp(0.0, _canvasWidth - _speakerSize),
                              y: nY.clamp(0.0, _canvasHeight - _speakerSize),
                            ),
                            immediate: true,
                          );
                    } catch (_) {}
                  }
                  _draggedSpeakersOffsets.clear();
                  widget.onDragUpdate?.call();
                },
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(8),
                  child: BackdropFilter(
                    filter: ImageFilter.blur(sigmaX: 10, sigmaY: 10),
                    child: Container(
                      decoration: BoxDecoration(
                        color: roomColor.withValues(alpha: 0.18),
                        border: Border.all(
                          color: roomColor,
                          width: 2.5,
                        ),
                        borderRadius: BorderRadius.circular(8),
                        boxShadow: [
                          BoxShadow(
                            color: roomColor.withValues(alpha: 0.25),
                            blurRadius: 12,
                            spreadRadius: 1,
                          ),
                        ],
                      ),
                      child: Center(
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                Text(
                                  widget.room.label,
                                  style: const TextStyle(
                                    color: Colors.white,
                                    fontSize: 16,
                                    fontWeight: FontWeight.bold,
                                    letterSpacing: 0.5,
                                  ),
                                ),
                                const SizedBox(width: 6),
                                const Icon(Icons.edit, size: 14, color: Colors.white70),
                              ],
                            ),
                            const SizedBox(height: 2),
                            Text(
                              '${widget.room.physicalWidth.toStringAsFixed(1)}m × ${widget.room.physicalHeight.toStringAsFixed(1)}m (${widget.room.rotation.toInt()}°)',
                              style: TextStyle(
                                color: Colors.white.withValues(alpha: 0.7),
                                fontSize: 11,
                                fontWeight: FontWeight.w500,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
            if (widget.room.hasDoor) _buildDoorHandle(),
            _buildRotateHandle(),
            Positioned(
              top: -28,
              left: 0,
              child: Wrap(
                spacing: 4,
                children: widget.containedSpeakers.map((s) => Container(
                      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                      decoration: BoxDecoration(
                        color: roomColor,
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        '[Ch ${s.channel + 1}]',
                        style: const TextStyle(
                          color: Colors.white,
                          fontSize: 12,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    )).toList(),
              ),
            ),
            _buildResizeHandle(Alignment.topLeft),
            _buildResizeHandle(Alignment.topCenter),
            _buildResizeHandle(Alignment.topRight),
            _buildResizeHandle(Alignment.centerLeft),
            _buildResizeHandle(Alignment.centerRight),
            _buildResizeHandle(Alignment.bottomLeft),
            _buildResizeHandle(Alignment.bottomCenter),
            _buildResizeHandle(Alignment.bottomRight),
          ],
        ),
      ),
    );
  }
}

class _DraggableSpeakerWidget extends ConsumerStatefulWidget {
  final SpeakerNode node;
  final Color? roomColor;
  final bool isDuplicate;
  final TransformationController transformationController;

  const _DraggableSpeakerWidget({
    super.key,
    required this.node,
    this.roomColor,
    required this.isDuplicate,
    required this.transformationController,
  });

  @override
  ConsumerState<_DraggableSpeakerWidget> createState() => _DraggableSpeakerWidgetState();
}

class _DraggableSpeakerWidgetState extends ConsumerState<_DraggableSpeakerWidget> {
  late double _localX;
  late double _localY;
  bool _isDragging = false;

  @override
  void initState() {
    super.initState();
    _localX = widget.node.x;
    _localY = widget.node.y;
  }

  @override
  void didUpdateWidget(covariant _DraggableSpeakerWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!_isDragging) {
      _localX = widget.node.x;
      _localY = widget.node.y;
    }
  }

  @override
  Widget build(BuildContext context) {
    return Positioned(
      left: _localX,
      top: _localY,
      child: Stack(
        clipBehavior: Clip.none,
        children: [
          Transform.rotate(
            angle: widget.node.rotation * math.pi / 180.0,
            child: GestureDetector(
              onPanStart: (_) => setState(() => _isDragging = true),
              onPanUpdate: (details) {
                final scale = widget.transformationController.value.getMaxScaleOnAxis();
                final currentScale = scale > 0 ? scale : 1.0;
                setState(() {
                  _localX = (_localX + details.delta.dx / currentScale).clamp(0.0, _canvasWidth - _speakerSize);
                  _localY = (_localY + details.delta.dy / currentScale).clamp(0.0, _canvasHeight - _speakerSize);
                });
                ref.read(speakerLayoutProvider.notifier).updateSpeaker(
                  widget.node.copyWith(x: _localX, y: _localY),
                  immediate: true,
                );
              },
              onPanEnd: (details) {
                final snappedX = (_localX / _gridSize).round() * _gridSize;
                final snappedY = (_localY / _gridSize).round() * _gridSize;
                final updated = widget.node.copyWith(
                  x: snappedX.clamp(0.0, _canvasWidth - _speakerSize),
                  y: snappedY.clamp(0.0, _canvasHeight - _speakerSize),
                );
                setState(() {
                  _localX = updated.x;
                  _localY = updated.y;
                  _isDragging = false;
                });
                ref.read(speakerLayoutProvider.notifier).updateSpeaker(updated, immediate: true);
              },
              child: SpeakerNodeWidget(
                node: widget.node,
                roomColor: widget.roomColor,
                onChannelChanged: (ch) {
                  ref.read(speakerLayoutProvider.notifier).updateSpeaker(widget.node.copyWith(channel: ch));
                },
                onDelete: () {
                  ref.read(speakerLayoutProvider.notifier).removeSpeaker(widget.node.id);
                },
                isDuplicateChannel: widget.isDuplicate,
              ),
            ),
          ),
          // Speaker Rotation Handle Knob
          Positioned(
            top: -22,
            left: _speakerSize / 2 - 10,
            child: GestureDetector(
              onPanUpdate: (details) {
                final renderBox = context.findRenderObject() as RenderBox?;
                if (renderBox != null) {
                  final localTouch = renderBox.globalToLocal(details.globalPosition);
                  final centerLocal = Offset(_speakerSize / 2, _speakerSize / 2);
                  final dx = localTouch.dx - centerLocal.dx;
                  final dy = localTouch.dy - centerLocal.dy;
                  double angleRad = math.atan2(dy, dx) + math.pi / 2;
                  double angleDeg = (angleRad * 180 / math.pi) % 360;
                  if (angleDeg < 0) angleDeg += 360;

                  ref.read(speakerLayoutProvider.notifier).updateSpeaker(
                        widget.node.copyWith(rotation: angleDeg),
                        immediate: true,
                      );
                }
              },
              child: Container(
                width: 20,
                height: 20,
                decoration: BoxDecoration(
                  color: Colors.orangeAccent,
                  shape: BoxShape.circle,
                  border: Border.all(color: Colors.black, width: 1.5),
                  boxShadow: const [BoxShadow(color: Colors.black45, blurRadius: 3)],
                ),
                child: const Icon(Icons.rotate_right, size: 11, color: Colors.black),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _DraggableWaypointWidget extends ConsumerStatefulWidget {
  final Trajectory trajectory;
  final int waypointIndex;
  final TrajectoryWaypoint waypoint;
  final TransformationController transformationController;
  final VoidCallback onDragUpdate;

  const _DraggableWaypointWidget({
    super.key,
    required this.trajectory,
    required this.waypointIndex,
    required this.waypoint,
    required this.transformationController,
    required this.onDragUpdate,
  });

  @override
  ConsumerState<_DraggableWaypointWidget> createState() => _DraggableWaypointWidgetState();
}

class _DraggableWaypointWidgetState extends ConsumerState<_DraggableWaypointWidget> {
  late double _localX;
  late double _localY;

  @override
  void initState() {
    super.initState();
    _localX = widget.waypoint.x;
    _localY = widget.waypoint.y;
  }

  @override
  void didUpdateWidget(covariant _DraggableWaypointWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    _localX = widget.waypoint.x;
    _localY = widget.waypoint.y;
  }

  @override
  Widget build(BuildContext context) {
    return Positioned(
      left: _localX - 20,
      top: _localY - 20,
      child: GestureDetector(
        onPanUpdate: (details) {
          final scale = widget.transformationController.value.getMaxScaleOnAxis();
          final currentScale = scale > 0 ? scale : 1.0;
          setState(() {
            _localX = (_localX + details.delta.dx / currentScale).clamp(0.0, _canvasWidth);
            _localY = (_localY + details.delta.dy / currentScale).clamp(0.0, _canvasHeight);
          });
          
          // 실시간으로 선이 따라가도록 Riverpod 업데이트
          final wps = List<TrajectoryWaypoint>.from(widget.trajectory.waypoints);
          wps[widget.waypointIndex] = TrajectoryWaypoint(_localX, _localY);
          ref.read(trajectoryProvider.notifier).updateTrajectory(
            widget.trajectory.copyWith(waypoints: wps),
            immediate: true,
          );
          widget.onDragUpdate();
        },
        onPanEnd: (details) {
          // 강제 동기화 보장 (Trailing Edge 유실 방지)
          widget.onDragUpdate();
        },
        onDoubleTap: () {
          // 더블클릭시 삭제 (최소 2개의 점은 유지해야 함)
          if (widget.trajectory.waypoints.length > 2) {
             final wps = List<TrajectoryWaypoint>.from(widget.trajectory.waypoints);
             wps.removeAt(widget.waypointIndex);
             ref.read(trajectoryProvider.notifier).updateTrajectory(
               widget.trajectory.copyWith(waypoints: wps),
             );
          } else {
             ref.read(trajectoryProvider.notifier).removeTrajectory(widget.trajectory.id);
          }
        },
        child: Container(
          width: 40,
          height: 40,
          color: Colors.transparent,
          child: Center(
            child: Container(
              width: 16,
              height: 16,
              decoration: BoxDecoration(
                color: Colors.white,
                shape: BoxShape.circle,
                border: Border.all(color: Colors.cyanAccent, width: 3),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _GridPainter extends CustomPainter {
  final double scale;

  _GridPainter(this.scale);

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.white12
      ..strokeWidth = 1.0;

    final textPainter = TextPainter(
      textDirection: TextDirection.ltr,
    );

    for (double i = 0; i <= size.width; i += _gridSize) {
      canvas.drawLine(Offset(i, 0), Offset(i, size.height), paint);
      
      if (i > 0 && i % (_gridSize * 5) == 0) {
        final meters = (i / scale).toStringAsFixed(1);
        textPainter.text = TextSpan(
          text: '${meters}m',
          style: const TextStyle(color: Colors.white38, fontSize: 10),
        );
        textPainter.layout();
        textPainter.paint(canvas, Offset(i + 2, 2));
      }
    }
    for (double i = 0; i <= size.height; i += _gridSize) {
      canvas.drawLine(Offset(0, i), Offset(size.width, i), paint);

      if (i > 0 && i % (_gridSize * 5) == 0) {
        final meters = (i / scale).toStringAsFixed(1);
        textPainter.text = TextSpan(
          text: '${meters}m',
          style: const TextStyle(color: Colors.white38, fontSize: 10),
        );
        textPainter.layout();
        textPainter.paint(canvas, Offset(2, i + 2));
      }
    }
  }

  @override
  bool shouldRepaint(covariant _GridPainter oldDelegate) => oldDelegate.scale != scale;
}

class _TrajectoryPainter extends CustomPainter {
  final List<Trajectory> trajectories;

  _TrajectoryPainter(this.trajectories);

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.cyanAccent
      ..strokeWidth = 3.0
      ..style = PaintingStyle.stroke;

    final headPaint = Paint()
      ..color = Colors.cyanAccent
      ..style = PaintingStyle.fill;

    for (var t in trajectories) {
      if (t.waypoints.length < 2) continue;

      final path = Path();
      path.moveTo(t.waypoints.first.x, t.waypoints.first.y);

      if (t.waypoints.length == 2) {
        path.lineTo(t.waypoints.last.x, t.waypoints.last.y);
      } else {
        for (int i = 0; i < t.waypoints.length - 1; i++) {
          final p0 = i == 0 ? t.waypoints[i] : t.waypoints[i - 1];
          final p1 = t.waypoints[i];
          final p2 = t.waypoints[i + 1];
          final p3 = i == t.waypoints.length - 2 ? t.waypoints[i + 1] : t.waypoints[i + 2];
          
          final double tension = 0.25;
          final cp1x = p1.x + (p2.x - p0.x) * tension;
          final cp1y = p1.y + (p2.y - p0.y) * tension;
          final cp2x = p2.x - (p3.x - p1.x) * tension;
          final cp2y = p2.y - (p3.y - p1.y) * tension;
          
          path.cubicTo(cp1x, cp1y, cp2x, cp2y, p2.x, p2.y);
        }
      }
      canvas.drawPath(path, paint);

      // 화살표 머리 렌더링 (Arrowhead) - 끝에서 두번째 점과 마지막 점 사이의 각도를 이용
      final p1 = t.waypoints[t.waypoints.length - 2];
      final p2 = t.waypoints.last;
      final angle = math.atan2(p2.y - p1.y, p2.x - p1.x);
      final headLength = 15.0;
      final headAngle = math.pi / 6; // 30 degrees

      final h1x = p2.x - headLength * math.cos(angle - headAngle);
      final h1y = p2.y - headLength * math.sin(angle - headAngle);
      final h2x = p2.x - headLength * math.cos(angle + headAngle);
      final h2y = p2.y - headLength * math.sin(angle + headAngle);

      final headPath = Path()
        ..moveTo(p2.x, p2.y)
        ..lineTo(h1x, h1y)
        ..lineTo(h2x, h2y)
        ..close();

      canvas.drawPath(headPath, headPaint);
    }
  }

  @override
  bool shouldRepaint(covariant _TrajectoryPainter oldDelegate) {
    return true;
  }
}

class _HeatmapPainter extends CustomPainter {
  final List<SpeakerNode> nodes;

  _HeatmapPainter(this.nodes);

  @override
  void paint(Canvas canvas, Size size) {
    for (var node in nodes) {
      final center = Offset(node.x + _speakerSize / 2, node.y + _speakerSize / 2);
      
      // Draw coverage ring
      final ringPaint = Paint()
        ..color = Colors.redAccent.withValues(alpha: 0.3)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 2.0;
      canvas.drawCircle(center, 150.0, ringPaint);

      // Draw heatmap gradient
      final Rect rect = Rect.fromCircle(center: center, radius: 150.0);
      final gradient = RadialGradient(
        colors: [
          Colors.redAccent.withValues(alpha: 0.4),
          Colors.orangeAccent.withValues(alpha: 0.15),
          Colors.transparent,
        ],
        stops: const [0.0, 0.5, 1.0],
      );
      final paint = Paint()..shader = gradient.createShader(rect);
      canvas.drawCircle(center, 150.0, paint);
    }
  }

  @override
  bool shouldRepaint(covariant _HeatmapPainter oldDelegate) => true;
}

class _CadDoorPainter extends CustomPainter {
  final Color color;
  final int wall; // 0: Top, 1: Right, 2: Bottom, 3: Left
  
  _CadDoorPainter({required this.color, required this.wall});

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.0;

    canvas.save();
    canvas.translate(size.width / 2, size.height / 2);
    canvas.rotate(wall * math.pi / 2);
    canvas.translate(-size.width / 2, -size.height / 2);

    final center = Offset(size.width, size.height);
    canvas.drawLine(center, Offset(0, size.height), paint);
    canvas.drawArc(
      Rect.fromCircle(center: center, radius: size.width),
      math.pi,
      math.pi / 2,
      false,
      paint,
    );
    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant _CadDoorPainter oldDelegate) {
    return oldDelegate.color != color || oldDelegate.wall != wall;
  }
}

