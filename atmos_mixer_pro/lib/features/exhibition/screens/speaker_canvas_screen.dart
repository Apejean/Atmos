import 'package:atmos_mixer_pro/features/exhibition/widgets/room_zone_widget.dart';
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
import 'package:atmos_mixer_pro/features/dashboard/widgets/vu_meter.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/trajectory.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/trajectory_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/trajectory_layer_painter.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/trajectory_sidebar_widget.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/trajectory_editor_toolbar.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;

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
  final FocusNode _canvasFocusNode = FocusNode();
  Size _viewportSize = const Size(800, 600);

  static int _roomColorIndex = 0;

  Timer? _automationTimer;
  bool _isPlayingAutomation = false;

  bool _showSpeakers = true;
  bool _showRooms = true;
  bool _showTrajectories = true;
  bool _showHeatmap = true;
  String _selectedOctaveFilter = 'All';
  String? _selectedRoomId;
  bool _isRoomInteracting = false;

  bool _isMeasuringScale = false;
  Offset? _measureStart;
  Offset? _measureEnd;

  bool _isSidebarOpen = false;

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
    _canvasFocusNode.dispose();
    _automationTimer?.cancel();
    _transformationController.dispose();
    super.dispose();
  }

  DateTime? _lastSyncTime;

  void _syncSpatialConfigRealtime() {
    final now = DateTime.now();
    if (_lastSyncTime != null &&
        now.difference(_lastSyncTime!).inMilliseconds < 16) {
      return;
    }
    _lastSyncTime = now;

    final nodes = ref.read(speakerLayoutProvider);
    final rooms = ref.read(roomZoneProvider);
    final trajectories = ref.read(trajectoryProvider);

    final payload = {
      'channel_positions': List.generate(
        ref.read(engineStateProvider).outputChannelCount,
        (index) {
          final node = nodes.where((n) => n.channel == index).firstOrNull;
          if (node == null) return null;
          return {
            'x': node.x / ref.read(blueprintProvider).scale,
            'y': node.y / ref.read(blueprintProvider).scale,
            'z': 0.0,
          };
        },
      ),
      'room_zones': rooms.map((r) {
        return {
          'room_id': r.id.hashCode.abs(),
          'boundary_min': {
            'x': r.x / ref.read(blueprintProvider).scale,
            'y': r.y / ref.read(blueprintProvider).scale,
            'z': 0.0,
          },
          'boundary_max': {
            'x': (r.x + r.width) / ref.read(blueprintProvider).scale,
            'y': (r.y + r.height) / ref.read(blueprintProvider).scale,
            'z': 2.0,
          },
          'absorption_coeff': r.absorptionCoeff,
          'material_name': r.materialName,
          'transmission_loss': r.wallTransmissionLoss,
        };
      }).toList(),
      'trajectory':
          trajectories.isNotEmpty && trajectories.first.waypoints.isNotEmpty
          ? {
              'waypoints': trajectories.first.waypoints
                  .map(
                    (w) => {
                      'x': w.position.dx,
                      'y': w.position.dy,
                      'z': w.heightZ,
                    },
                  )
                  .toList(),
              'current_position': {
                'x': trajectories.first.getCurrentPositionMeter().dx,
                'y': trajectories.first.getCurrentPositionMeter().dy,
                'z': trajectories.first.getCurrentHeightZ(),
              },
              'audio_file_path': trajectories.first.audioFilePath,
            }
          : null,
    };

    rust_api
        .apiUpdateSpatialConfigJson(jsonPayload: jsonEncode(payload))
        .catchError((e) {
          debugPrint('Real-time FFI sync error: $e');
        });
  }

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
      _automationTimer = Timer.periodic(const Duration(milliseconds: 16), (
        timer,
      ) {
        for (var trajectory in trajectories) {
          trajectory.updateProgress(16.0 / 1000.0, trajectory.totalPathLength);
        }
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

  void _finishMeasurement() {
    final distancePx = (_measureEnd! - _measureStart!).distance;
    setState(() {
      _isMeasuringScale = false;
    });

    if (distancePx < 10) return;

    double inputMeter = 1.0;
    showDialog(
      context: context,
      builder: (context) {
        return AlertDialog(
          backgroundColor: AppColors.cardSurface,
          title: const Text('실제 거리 입력', style: TextStyle(color: Colors.white)),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                '선택한 선의 화면상 길이: ${distancePx.toStringAsFixed(1)}px',
                style: const TextStyle(color: Colors.white70),
              ),
              const SizedBox(height: 16),
              TextField(
                keyboardType: const TextInputType.numberWithOptions(
                  decimal: true,
                ),
                style: const TextStyle(color: Colors.white),
                decoration: const InputDecoration(
                  labelText: '실제 거리 (미터)',
                  labelStyle: TextStyle(color: Colors.white54),
                  enabledBorder: OutlineInputBorder(
                    borderSide: BorderSide(color: Colors.white24),
                  ),
                  focusedBorder: OutlineInputBorder(
                    borderSide: BorderSide(color: AppColors.primaryNeon),
                  ),
                ),
                onChanged: (val) {
                  inputMeter = double.tryParse(val) ?? 1.0;
                },
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('취소', style: TextStyle(color: Colors.white54)),
            ),
            ElevatedButton(
              style: ElevatedButton.styleFrom(
                backgroundColor: AppColors.primaryNeon,
                foregroundColor: Colors.black,
              ),
              onPressed: () {
                if (inputMeter > 0) {
                  final newScale = distancePx / inputMeter;
                  ref.read(blueprintProvider.notifier).setScale(newScale);
                  _syncSpatialConfigRealtime();
                }
                Navigator.pop(context);
              },
              child: const Text('적용'),
            ),
          ],
        );
      },
    );
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

    double cx =
        (canvasCenter.dx / ref.read(blueprintProvider).scale).round() *
        ref.read(blueprintProvider).scale;
    double cy =
        (canvasCenter.dy / ref.read(blueprintProvider).scale).round() *
        ref.read(blueprintProvider).scale;

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

    double cx =
        (canvasCenter.dx / ref.read(blueprintProvider).scale).round() *
        ref.read(blueprintProvider).scale;
    double cy =
        (canvasCenter.dy / ref.read(blueprintProvider).scale).round() *
        ref.read(blueprintProvider).scale;

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

    final t = TrajectoryModel(
      id: const Uuid().v4(),
      waypoints: [
        Waypoint(
          position: Offset(
            (cx - 100) / ref.read(blueprintProvider).scale,
            cy / ref.read(blueprintProvider).scale,
          ),
        ),
        Waypoint(
          position: Offset(
            (cx + 100) / ref.read(blueprintProvider).scale,
            cy / ref.read(blueprintProvider).scale,
          ),
        ),
      ],
      color: AppColors.primaryNeon,
    );
    ref.read(trajectoryProvider.notifier).addTrajectory(t);
  }

  void _clearTrajectories() {
    ref.read(trajectoryProvider.notifier).clearAll();
    setState(() {
      _isPlayingAutomation = false;
      _automationTimer?.cancel();
      _automationTimer = null;
    });
    _syncSpatialConfigRealtime();
  }

  void _clearCanvas() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        backgroundColor: AppColors.cardSurface,
        title: const Text('캔버스 전체 초기화', style: TextStyle(color: Colors.white)),
        content: const Text(
          '모든 스피커와 구역, 궤도를 삭제하시겠습니까?\n이 작업은 되돌릴 수 없습니다.',
          style: TextStyle(color: Colors.white70),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('취소', style: TextStyle(color: Colors.white70)),
          ),
          ElevatedButton(
            style: ElevatedButton.styleFrom(backgroundColor: Colors.redAccent),
            onPressed: () {
              ref.read(speakerLayoutProvider.notifier).clearAll();
              ref.read(roomZoneProvider.notifier).clearAll();
              _clearTrajectories();
              ref.read(blueprintProvider.notifier).clearBlueprint();
              Navigator.pop(context);
            },
            child: const Text('삭제', style: TextStyle(color: Colors.white)),
          ),
        ],
      ),
    );
  }

  Future<void> _editRoom(RoomZone room) async {
    final blueprint = ref.read(blueprintProvider);
    final nameController = TextEditingController(text: room.label);
    final widthController = TextEditingController(
      text: room.physicalWidth.toStringAsFixed(1),
    );
    final heightController = TextEditingController(
      text: room.physicalHeight.toStringAsFixed(1),
    );
    int selectedColor = room.color;
    bool hasDoor = room.hasDoor;
    int doorWall = room.doorWall;
    double doorOffset = room.doorOffset;
    double rotation = room.rotation;
    String selectedMaterial = room.materialName;
    double currentCoeff = room.absorptionCoeff;
    double currentTL = room.wallTransmissionLoss;

    await showDialog(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) {
          final tempWidthM =
              double.tryParse(widthController.text) ?? room.physicalWidth;
          final tempHeightM =
              double.tryParse(heightController.text) ?? room.physicalHeight;
          final tempRoom = room.copyWith(
            physicalWidth: tempWidthM,
            physicalHeight: tempHeightM,
            absorptionCoeff: currentCoeff,
          );
          final estimatedRt60 = tempRoom.estimatedRt60;

          return AlertDialog(
            backgroundColor: AppColors.cardSurface,
            title: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                const Text(
                  '룸 구역 설정 (Room Settings & Tuning)',
                  style: TextStyle(
                    color: Colors.white,
                    fontWeight: FontWeight.bold,
                    fontSize: 16,
                  ),
                ),
                IconButton(
                  icon: const Icon(Icons.close, color: Colors.white54),
                  onPressed: () {
                    setState(() => _selectedRoomId = null);
                    Navigator.pop(context);
                  },
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
                      labelText: '룸 이름 (Room Name / Label)',
                      labelStyle: TextStyle(
                        color: AppColors.primaryNeon,
                        fontWeight: FontWeight.bold,
                      ),
                      enabledBorder: UnderlineInputBorder(
                        borderSide: BorderSide(color: Colors.white24),
                      ),
                      focusedBorder: UnderlineInputBorder(
                        borderSide: BorderSide(color: AppColors.primaryNeon),
                      ),
                    ),
                  ),
                  const SizedBox(height: 16),
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: widthController,
                          style: const TextStyle(color: Colors.white),
                          keyboardType: const TextInputType.numberWithOptions(
                            decimal: true,
                          ),
                          decoration: const InputDecoration(
                            labelText: '너비 (m)',
                            labelStyle: TextStyle(color: Colors.white70),
                            enabledBorder: UnderlineInputBorder(
                              borderSide: BorderSide(color: Colors.white24),
                            ),
                            focusedBorder: UnderlineInputBorder(
                              borderSide: BorderSide(
                                color: AppColors.primaryNeon,
                              ),
                            ),
                          ),
                          onChanged: (_) => setDialogState(() {}),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: TextField(
                          controller: heightController,
                          style: const TextStyle(color: Colors.white),
                          keyboardType: const TextInputType.numberWithOptions(
                            decimal: true,
                          ),
                          decoration: const InputDecoration(
                            labelText: '높이 (m)',
                            labelStyle: TextStyle(color: Colors.white70),
                            enabledBorder: UnderlineInputBorder(
                              borderSide: BorderSide(color: Colors.white24),
                            ),
                            focusedBorder: UnderlineInputBorder(
                              borderSide: BorderSide(
                                color: AppColors.primaryNeon,
                              ),
                            ),
                          ),
                          onChanged: (_) => setDialogState(() {}),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 20),
                  const Text(
                    '음향 표면 재질 프리셋',
                    style: TextStyle(
                      color: AppColors.primaryNeon,
                      fontSize: 13,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  const SizedBox(height: 8),
                  DropdownButton<RoomMaterialPreset>(
                    value: RoomMaterialPreset.presets.firstWhere(
                      (p) => p.name == selectedMaterial,
                      orElse: () => RoomMaterialPreset.presets[1],
                    ),
                    dropdownColor: AppColors.cardSurface,
                    isExpanded: true,
                    style: const TextStyle(color: Colors.white, fontSize: 13),
                    items: RoomMaterialPreset.presets.map((preset) {
                      return DropdownMenuItem<RoomMaterialPreset>(
                        value: preset,
                        child: Text(preset.name),
                      );
                    }).toList(),
                    onChanged: (preset) {
                      if (preset != null) {
                        setDialogState(() {
                          selectedMaterial = preset.name;
                          currentCoeff = preset.averageAlpha;
                        });
                      }
                    },
                  ),
                  const SizedBox(height: 8),
                  Container(
                    padding: const EdgeInsets.all(10),
                    decoration: BoxDecoration(
                      color: Colors.black38,
                      borderRadius: BorderRadius.circular(6),
                      border: Border.all(color: Colors.white12),
                    ),
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Text(
                          'Absorption Coeff (α): ${currentCoeff.toStringAsFixed(2)}',
                          style: const TextStyle(
                            color: Colors.white70,
                            fontSize: 12,
                          ),
                        ),
                        Text(
                          'Sabine RT60: ${estimatedRt60.toStringAsFixed(2)}s',
                          style: const TextStyle(
                            color: AppColors.primaryNeon,
                            fontSize: 12,
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: 20),
                  Text(
                    '차음 성능 (Transmission Loss): ${currentTL.toInt()} dB',
                    style: const TextStyle(
                      color: Colors.white,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Slider(
                    value: currentTL,
                    min: 10.0,
                    max: 80.0,
                    activeColor: AppColors.primaryNeon,
                    onChanged: (val) => setDialogState(() => currentTL = val),
                  ),
                  const SizedBox(height: 20),
                  const Text(
                    '테마 색상',
                    style: TextStyle(color: Colors.white70, fontSize: 13),
                  ),
                  const SizedBox(height: 10),
                  Wrap(
                    spacing: 10,
                    runSpacing: 10,
                    children: AppColors.roomAccents.map((color) {
                      final isSelected = color.toARGB32() == selectedColor;
                      return GestureDetector(
                        onTap: () => setDialogState(
                          () => selectedColor = color.toARGB32(),
                        ),
                        child: Container(
                          width: 36,
                          height: 36,
                          decoration: BoxDecoration(
                            color: color,
                            shape: BoxShape.circle,
                            border: Border.all(
                              color: isSelected
                                  ? Colors.white
                                  : Colors.transparent,
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
                      Text(
                        '회전 (Rotation): ${rotation.toInt()}°',
                        style: const TextStyle(
                          color: Colors.white,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
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
                      const Text(
                        '입구 마커 (Door)',
                        style: TextStyle(
                          color: Colors.white,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
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
                        const Text(
                          '문 위치 벽면: ',
                          style: TextStyle(color: Colors.white70, fontSize: 13),
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          child: DropdownButton<int>(
                            value: doorWall,
                            dropdownColor: AppColors.cardSurface,
                            isExpanded: true,
                            style: const TextStyle(color: Colors.white),
                            items: const [
                              DropdownMenuItem(value: 0, child: Text('상단 벽')),
                              DropdownMenuItem(value: 1, child: Text('우측 벽')),
                              DropdownMenuItem(value: 2, child: Text('하단 벽')),
                              DropdownMenuItem(value: 3, child: Text('좌측 벽')),
                            ],
                            onChanged: (val) {
                              if (val != null) {
                                setDialogState(() => doorWall = val);
                              }
                            },
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 12),
                    Text(
                      '문 위치 비율: ${(doorOffset * 100).toInt()}%',
                      style: const TextStyle(
                        color: Colors.white70,
                        fontSize: 13,
                      ),
                    ),
                    Slider(
                      value: doorOffset,
                      min: 0.0,
                      max: 1.0,
                      activeColor: AppColors.primaryNeon,
                      onChanged: (val) =>
                          setDialogState(() => doorOffset = val),
                    ),
                  ],
                ],
              ),
            ),
            actions: [
              TextButton(
                onPressed: () {
                  ref.read(roomZoneProvider.notifier).removeRoomZone(room.id);
                  setState(() => _selectedRoomId = null);
                  Navigator.pop(context);
                },
                child: const Text(
                  '룸 구역 삭제',
                  style: TextStyle(color: Colors.redAccent),
                ),
              ),
              TextButton(
                onPressed: () {
                  setState(() => _selectedRoomId = null);
                  Navigator.pop(context);
                },
                child: const Text(
                  '취소',
                  style: TextStyle(color: Colors.white70),
                ),
              ),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.primaryNeon,
                  foregroundColor: Colors.black,
                ),
                onPressed: () {
                  final newWidthM =
                      double.tryParse(widthController.text) ??
                      room.physicalWidth;
                  final newHeightM =
                      double.tryParse(heightController.text) ??
                      room.physicalHeight;
                  final scale = blueprint.scale > 0 ? blueprint.scale : 40.0;

                  ref
                      .read(roomZoneProvider.notifier)
                      .updateRoomZone(
                        room.copyWith(
                          label: nameController.text.trim().isNotEmpty
                              ? nameController.text.trim()
                              : room.label,
                          color: selectedColor,
                          physicalWidth: newWidthM,
                          physicalHeight: newHeightM,
                          width: newWidthM * scale,
                          height: newHeightM * scale,
                          hasDoor: hasDoor,
                          doorWall: doorWall,
                          doorOffset: doorOffset,
                          rotation: rotation,
                          materialName: selectedMaterial,
                          absorptionCoeff: currentCoeff,
                          wallTransmissionLoss: currentTL,
                        ),
                        immediate: true,
                      );
                  _syncSpatialConfigRealtime();
                  setState(() => _selectedRoomId = null);
                  Navigator.pop(context);
                },
                child: const Text(
                  '설정 저장',
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
              ),
            ],
          );
        },
      ),
    );
  }

  Future<void> _editSpeaker(SpeakerNode node) async {
    double dispAngle = node.dispersionAngle;
    double dispDist = node.dispersionDistance;
    double heightZ = node.heightZ;
    double pitchTilt = node.pitchTilt;
    double rotation = node.rotation;
    int channel = node.channel;
    String selectedPreset = 'Custom';

    await showDialog(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) {
          return AlertDialog(
            backgroundColor: AppColors.cardSurface,
            title: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  '스피커 설정 [Ch ${channel + 1}]',
                  style: const TextStyle(
                    color: Colors.white,
                    fontWeight: FontWeight.bold,
                  ),
                ),
                IconButton(
                  icon: const Icon(Icons.close, color: Colors.white54),
                  onPressed: () {
                    setState(() => _selectedRoomId = null);
                    Navigator.pop(context);
                  },
                ),
              ],
            ),
            content: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text(
                    '채널 할당',
                    style: TextStyle(color: Colors.white70, fontSize: 13),
                  ),
                  const SizedBox(height: 8),
                  DropdownButton<int>(
                    value: channel,
                    dropdownColor: AppColors.cardSurface,
                    isExpanded: true,
                    style: const TextStyle(color: Colors.white),
                    items: List.generate(
                      64,
                      (index) => DropdownMenuItem(
                        value: index,
                        child: Text('Ch ${index + 1}'),
                      ),
                    ),
                    onChanged: (val) {
                      if (val != null) {
                        setDialogState(() => channel = val);
                      }
                    },
                  ),
                  const SizedBox(height: 16),
                  const Divider(color: Colors.white24),
                  const SizedBox(height: 8),
                  const Text(
                    '스피커 음향 프리셋',
                    style: TextStyle(color: Colors.white70, fontSize: 13),
                  ),
                  const SizedBox(height: 8),
                  DropdownButton<String>(
                    value: selectedPreset,
                    dropdownColor: AppColors.cardSurface,
                    isExpanded: true,
                    style: const TextStyle(color: Colors.white),
                    items: const [
                      DropdownMenuItem(
                        value: 'Custom',
                        child: Text('사용자 지정 설정'),
                      ),
                      DropdownMenuItem(
                        value: 'PointSource',
                        child: Text('Point Source (90° Beam, 3.5m Height)'),
                      ),
                      DropdownMenuItem(
                        value: 'LineArray',
                        child: Text(
                          'Line Array (60° Narrow Beam, 6.0m Height)',
                        ),
                      ),
                      DropdownMenuItem(
                        value: 'Subwoofer',
                        child: Text(
                          'Subwoofer (180° Omnidirectional, 0.5m Height)',
                        ),
                      ),
                      DropdownMenuItem(
                        value: 'CeilingAtmos',
                        child: Text(
                          'Ceiling Overhead (120° Wide Beam, 4.0m Height)',
                        ),
                      ),
                    ],
                    onChanged: (val) {
                      if (val != null) {
                        setDialogState(() {
                          selectedPreset = val;
                          if (val == 'PointSource') {
                            dispAngle = 90.0;
                            dispDist = 220.0;
                            heightZ = 3.5;
                            pitchTilt = 15.0;
                          } else if (val == 'LineArray') {
                            dispAngle = 60.0;
                            dispDist = 450.0;
                            heightZ = 6.0;
                            pitchTilt = 10.0;
                          } else if (val == 'Subwoofer') {
                            dispAngle = 180.0;
                            dispDist = 150.0;
                            heightZ = 0.5;
                            pitchTilt = 0.0;
                          } else if (val == 'CeilingAtmos') {
                            dispAngle = 120.0;
                            dispDist = 180.0;
                            heightZ = 4.0;
                            pitchTilt = 30.0;
                          }
                        });
                      }
                    },
                  ),
                  const SizedBox(height: 16),
                  const Divider(color: Colors.white24),
                  const SizedBox(height: 8),
                  Text(
                    '소리 지향 각도: ${dispAngle.toInt()}°',
                    style: const TextStyle(
                      color: Colors.white,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Slider(
                    value: dispAngle,
                    min: 30.0,
                    max: 180.0,
                    activeColor: AppColors.primaryNeon,
                    onChanged: (val) => setDialogState(() {
                      dispAngle = val;
                      selectedPreset = 'Custom';
                    }),
                  ),
                  const SizedBox(height: 12),
                  Text(
                    '최대 음향 도달 거리: ${dispDist.toInt()}px',
                    style: const TextStyle(
                      color: Colors.white,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Slider(
                    value: dispDist,
                    min: 50.0,
                    max: 800.0,
                    activeColor: AppColors.primaryNeon,
                    onChanged: (val) => setDialogState(() {
                      dispDist = val;
                      selectedPreset = 'Custom';
                    }),
                  ),
                  const SizedBox(height: 12),
                  Text(
                    '설치 높이 (Z축): ${heightZ.toStringAsFixed(1)}m',
                    style: const TextStyle(
                      color: Colors.white,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Slider(
                    value: heightZ,
                    min: 0.5,
                    max: 15.0,
                    activeColor: AppColors.primaryNeon,
                    onChanged: (val) => setDialogState(() {
                      heightZ = val;
                      selectedPreset = 'Custom';
                    }),
                  ),
                  const SizedBox(height: 12),
                  Text(
                    '하향 경사각 (Pitch): ${pitchTilt.toInt()}°',
                    style: const TextStyle(
                      color: Colors.white,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Slider(
                    value: pitchTilt,
                    min: 0.0,
                    max: 60.0,
                    activeColor: AppColors.primaryNeon,
                    onChanged: (val) => setDialogState(() {
                      pitchTilt = val;
                      selectedPreset = 'Custom';
                    }),
                  ),
                  const SizedBox(height: 12),
                  Text(
                    '회전 지향각 (Yaw): ${rotation.toInt()}°',
                    style: const TextStyle(
                      color: Colors.white,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Slider(
                    value: rotation,
                    min: 0.0,
                    max: 360.0,
                    activeColor: AppColors.primaryNeon,
                    onChanged: (val) => setDialogState(() => rotation = val),
                  ),
                ],
              ),
            ),
            actions: [
              TextButton(
                onPressed: () {
                  ref
                      .read(speakerLayoutProvider.notifier)
                      .removeSpeaker(node.id);
                  Navigator.pop(context);
                },
                child: const Text(
                  '스피커 삭제',
                  style: TextStyle(color: Colors.redAccent),
                ),
              ),
              TextButton(
                onPressed: () {
                  setState(() => _selectedRoomId = null);
                  Navigator.pop(context);
                },
                child: const Text(
                  '취소',
                  style: TextStyle(color: Colors.white70),
                ),
              ),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.primaryNeon,
                  foregroundColor: Colors.black,
                ),
                onPressed: () {
                  ref
                      .read(speakerLayoutProvider.notifier)
                      .updateSpeaker(
                        node.copyWith(
                          dispersionAngle: dispAngle,
                          dispersionDistance: dispDist,
                          heightZ: heightZ,
                          pitchTilt: pitchTilt,
                          rotation: rotation,
                          channel: channel,
                        ),
                        immediate: true,
                      );
                  _syncSpatialConfigRealtime();
                  setState(() => _selectedRoomId = null);
                  Navigator.pop(context);
                },
                child: const Text(
                  '설정 저장',
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
              ),
            ],
          );
        },
      ),
    );
  }

  Future<void> _editTrajectory(TrajectoryModel trajectory) async {
    String? currentAudioPath = trajectory.audioFilePath;
    String name = trajectory.name;

    await showDialog(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) {
          return AlertDialog(
            backgroundColor: AppColors.cardSurface,
            title: const Text(
              '궤도 설정',
              style: TextStyle(
                color: Colors.white,
                fontWeight: FontWeight.bold,
              ),
            ),
            content: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text(
                    '이름',
                    style: TextStyle(color: Colors.white70, fontSize: 13),
                  ),
                  const SizedBox(height: 8),
                  TextFormField(
                    initialValue: name,
                    style: const TextStyle(color: Colors.white),
                    decoration: const InputDecoration(
                      filled: true,
                      fillColor: Colors.black45,
                      border: OutlineInputBorder(),
                    ),
                    onChanged: (val) => name = val,
                  ),
                  const SizedBox(height: 16),
                  const Text(
                    '오디오 파일',
                    style: TextStyle(color: Colors.white70, fontSize: 13),
                  ),
                  const SizedBox(height: 8),
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          currentAudioPath != null
                              ? currentAudioPath!
                                    .split(Platform.pathSeparator)
                                    .last
                              : '선택된 파일 없음',
                          style: TextStyle(
                            color: currentAudioPath != null
                                ? AppColors.primaryNeon
                                : Colors.white54,
                            fontSize: 12,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      IconButton(
                        icon: const Icon(
                          Icons.file_upload,
                          color: Colors.white70,
                        ),
                        tooltip: '파일 선택',
                        onPressed: () async {
                          try {
                            final result = await FilePicker.pickFiles(
                              type: FileType.custom,
                              allowedExtensions: ['wav'],
                              allowMultiple: false,
                            );
                            if (result != null && result.files.isNotEmpty) {
                              setDialogState(() {
                                currentAudioPath = result.files.first.path;
                              });
                            }
                          } catch (e) {
                            debugPrint('File picker error: $e');
                          }
                        },
                      ),
                      if (currentAudioPath != null)
                        IconButton(
                          icon: const Icon(
                            Icons.clear,
                            color: Colors.redAccent,
                          ),
                          tooltip: '오디오 제거',
                          onPressed: () {
                            setDialogState(() {
                              currentAudioPath = null;
                            });
                          },
                        ),
                    ],
                  ),
                  const SizedBox(height: 24),
                  SizedBox(
                    width: double.infinity,
                    child: ElevatedButton.icon(
                      style: ElevatedButton.styleFrom(
                        backgroundColor: Colors.white12,
                        padding: const EdgeInsets.symmetric(vertical: 12),
                      ),
                      icon: const Icon(Icons.add_location_alt, size: 16),
                      label: const Text('경로점(Waypoint) 추가'),
                      onPressed: () {
                        if (trajectory.waypoints.isNotEmpty) {
                          final lastWp = trajectory.waypoints.last;
                          final newWp = Waypoint(
                            position: Offset(
                              lastWp.position.dx + 1.0,
                              lastWp.position.dy + 1.0,
                            ),
                          );
                          final wps = List<Waypoint>.from(trajectory.waypoints)
                            ..add(newWp);
                          ref
                              .read(trajectoryProvider.notifier)
                              .updateTrajectory(
                                trajectory.copyWith(waypoints: wps),
                              );
                          ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(
                              content: Text('경로점이 추가되었습니다.'),
                              duration: Duration(seconds: 1),
                            ),
                          );
                        }
                      },
                    ),
                  ),
                ],
              ),
            ),
            actions: [
              TextButton(
                onPressed: () {
                  setState(() => _selectedRoomId = null);
                  Navigator.pop(context);
                },
                child: const Text(
                  '취소',
                  style: TextStyle(color: Colors.white70),
                ),
              ),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.primaryNeon,
                  foregroundColor: Colors.black,
                ),
                onPressed: () {
                  ref
                      .read(trajectoryProvider.notifier)
                      .updateTrajectory(
                        trajectory.copyWith(
                          name: name,
                          audioFilePath: currentAudioPath,
                          audioTrackId: currentAudioPath != null
                              ? (trajectory.audioTrackId ?? const Uuid().v4())
                              : null,
                        ),
                        immediate: true,
                      );
                  Navigator.pop(context);
                },
                child: const Text(
                  '설정 저장',
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
              ),
            ],
          );
        },
      ),
    );
  }

  List<SpeakerNode> _getSpeakersInRoom(RoomZone room, List<SpeakerNode> nodes) {
    return nodes
        .where(
          (n) => room.containsPoint(
            n.x + _speakerSize / 2,
            n.y + _speakerSize / 2,
          ),
        )
        .toList();
  }

  Color? _getRoomColorForSpeaker(SpeakerNode speaker, List<RoomZone> rooms) {
    for (final room in rooms) {
      if (room.containsPoint(
        speaker.x + _speakerSize / 2,
        speaker.y + _speakerSize / 2,
      )) {
        return Color(room.color);
      }
    }
    return null;
  }

  Widget _buildLayerToggle(
    String tooltip,
    IconData icon,
    bool isActive,
    VoidCallback onTap,
  ) {
    return Tooltip(
      message: tooltip,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(8),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          decoration: BoxDecoration(
            color: isActive
                ? AppColors.primaryNeon.withValues(alpha: 0.2)
                : Colors.transparent,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: isActive ? AppColors.primaryNeon : Colors.white24,
            ),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                icon,
                color: isActive ? AppColors.primaryNeon : Colors.white54,
                size: 18,
              ),
              const SizedBox(width: 6),
              Text(
                tooltip.split(' ')[0],
                style: TextStyle(
                  color: isActive ? AppColors.primaryNeon : Colors.white54,
                  fontSize: 12,
                  fontWeight: FontWeight.bold,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _handleCanvasTapForDrawing(Offset canvasPos, WidgetRef ref) {
    final activeId = ref.read(activeTrajectoryIdProvider);
    if (activeId == null) return;

    final trajectories = ref.read(trajectoryProvider);
    final traj = trajectories.where((t) => t.id == activeId).firstOrNull;
    if (traj == null) return;

    final rooms = ref.read(roomZoneProvider);
    final targetRoom = rooms
        .where((r) => r.containsPoint(canvasPos.dx, canvasPos.dy))
        .firstOrNull;

    if (targetRoom != null) {
      final rx = (canvasPos.dx - targetRoom.x) / targetRoom.width;
      final ry = (canvasPos.dy - targetRoom.y) / targetRoom.height;
      final newWaypoint = Waypoint(position: Offset(rx, ry));

      final isNewRoom = traj.targetRoomZoneId != targetRoom.id;
      final newWaypoints = isNewRoom
          ? [newWaypoint]
          : [...traj.waypoints, newWaypoint];

      ref
          .read(trajectoryProvider.notifier)
          .updateTrajectory(
            traj.copyWith(
              targetRoomZoneId: targetRoom.id,
              waypoints: newWaypoints,
            ),
          );
    }
  }

  @override
  Widget build(BuildContext context) {
    final blueprint = ref.watch(blueprintProvider);

    return GestureDetector(
      onTap: () {
        _canvasFocusNode.requestFocus();
        if (!_isMeasuringScale) setState(() => _selectedRoomId = null);
      },
      child: Scaffold(
        backgroundColor: AppColors.background,
        appBar: AppBar(
          title: Row(
            children: [
              const Text('Exhibition Canvas'),
              const Spacer(),
              Consumer(
                builder: (context, ref, child) {
                  final isMasterMuted = ref.watch(
                    engineStateProvider.select(
                      (state) => state.masterMuteActive,
                    ),
                  );
                  if (!isMasterMuted) return const SizedBox.shrink();
                  return RepaintBoundary(
                    child: ClipRRect(
                      borderRadius: BorderRadius.circular(4),
                      child: BackdropFilter(
                        filter: ImageFilter.blur(sigmaX: 10, sigmaY: 10),
                        child: Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 12,
                            vertical: 4,
                          ),
                          decoration: BoxDecoration(
                            color: Colors.orange.shade800.withValues(
                              alpha: 0.8,
                            ),
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: const Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Icon(
                                Icons.warning_amber_rounded,
                                color: Colors.white,
                                size: 16,
                              ),
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
                tooltip: _isPlayingAutomation
                    ? 'Stop Automation'
                    : 'Play Automation',
                icon: Icon(
                  _isPlayingAutomation ? Icons.stop : Icons.play_arrow,
                  color: _isPlayingAutomation
                      ? Colors.redAccent
                      : AppColors.primaryNeon,
                ),
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
                        title: const Text(
                          'Set Physical Scale',
                          style: TextStyle(color: Colors.white),
                        ),
                        content: Column(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            const Text(
                              'Pixels per Meter',
                              style: TextStyle(color: Colors.white70),
                            ),
                            const SizedBox(height: 16),
                            StatefulBuilder(
                              builder: (context, setDialogState) {
                                return Row(
                                  children: [
                                    Text(
                                      '${currentScale.toInt()} px/m',
                                      style: const TextStyle(
                                        color: Colors.white,
                                      ),
                                    ),
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
                                          ref
                                              .read(blueprintProvider.notifier)
                                              .setScale(val);
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
                            child: const Text(
                              'Close',
                              style: TextStyle(color: AppColors.primaryNeon),
                            ),
                          ),
                        ],
                      );
                    },
                  );
                },
              ),
              const SizedBox(width: 8),
              Container(height: 24, width: 1, color: Colors.white24),
              const SizedBox(width: 8),
              Wrap(
                spacing: 8,
                children: [
                  _buildLayerToggle(
                    'Speaker Layer',
                    Icons.speaker,
                    _showSpeakers,
                    () {
                      setState(() => _showSpeakers = !_showSpeakers);
                    },
                  ),
                  _buildLayerToggle(
                    'Room Layer',
                    Icons.meeting_room,
                    _showRooms,
                    () {
                      setState(() => _showRooms = !_showRooms);
                    },
                  ),
                  _buildLayerToggle(
                    'Trajectory Layer',
                    Icons.route,
                    _showTrajectories,
                    () {
                      setState(() {
                        _showTrajectories = !_showTrajectories;
                        if (!_showTrajectories && _isPlayingAutomation) {
                          _toggleAutomation(); // Pauses if playing
                        }
                      });
                    },
                  ),
                  _buildLayerToggle(
                    'Heatmap Layer',
                    Icons.wb_sunny,
                    _showHeatmap,
                    () {
                      setState(() => _showHeatmap = !_showHeatmap);
                    },
                  ),
                ],
              ),
              const SizedBox(width: 8),
              Container(height: 24, width: 1, color: Colors.white24),
              const SizedBox(width: 8),
              Row(
                children: ['All', '125Hz', '500Hz', '1kHz', '4kHz'].map((oct) {
                  final isSelected = _selectedOctaveFilter == oct;
                  return Padding(
                    padding: const EdgeInsets.only(right: 4.0),
                    child: FilterChip(
                      label: Text(
                        oct,
                        style: TextStyle(
                          fontSize: 11,
                          color: isSelected ? Colors.black : Colors.white70,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      selected: isSelected,
                      selectedColor: AppColors.primaryNeon,
                      backgroundColor: Colors.black45,
                      checkmarkColor: Colors.black,
                      padding: const EdgeInsets.symmetric(
                        horizontal: 4,
                        vertical: 0,
                      ),
                      onSelected: (val) {
                        if (val) {
                          setState(() => _selectedOctaveFilter = oct);
                        }
                      },
                    ),
                  );
                }).toList(),
              ),
              const SizedBox(width: 8),
              Container(height: 24, width: 1, color: Colors.white24),
              const SizedBox(width: 8),
              IconButton(
                icon: const Icon(
                  Icons.settings_input_component,
                  color: Colors.cyanAccent,
                ),
                onPressed: () {
                  setState(() {
                    _isSidebarOpen = !_isSidebarOpen;
                  });
                },
                tooltip: 'Routing & Trajectories',
              ),
              IconButton(
                icon: const Icon(
                  Icons.route_outlined,
                  color: Colors.orangeAccent,
                ),
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
            return Stack(
              children: [
                InteractiveViewer(
                  transformationController: _transformationController,
                  panEnabled: !_isMeasuringScale,
                  scaleEnabled: !_isMeasuringScale,
                  boundaryMargin: const EdgeInsets.all(double.infinity),
                  minScale: 0.1,
                  maxScale: 2.0,
                  constrained: false,
                  child: RepaintBoundary(
                    child: SizedBox(
                      width: _canvasWidth,
                      height: _canvasHeight,
                      child: Stack(
                        clipBehavior: Clip.none,
                        children: [
                          Positioned.fill(
                            child: GestureDetector(
                              behavior: HitTestBehavior.opaque,
                              onTapUp: (details) {
                                if (ref.read(isDrawingModeProvider)) {
                                  final invertedMatrix =
                                      _transformationController.value
                                          .clone()
                                        ..invert();
                                  final canvasPos = MatrixUtils.transformPoint(
                                    invertedMatrix,
                                    details.localPosition,
                                  );
                                  _handleCanvasTapForDrawing(canvasPos, ref);
                                  return;
                                }
                                if (!_isMeasuringScale) {
                                  setState(() => _selectedRoomId = null);
                                }
                              },
                              child: RepaintBoundary(
                                child: CustomPaint(
                                  painter: _GridPainter(blueprint.scale),
                                ),
                              ),
                            ),
                          ),
                          if (blueprint.imagePath != null)
                            Positioned.fill(
                              child: IgnorePointer(
                                child: Image.file(
                                  File(blueprint.imagePath!),
                                  fit: BoxFit.contain,
                                  color: Colors.white.withValues(
                                    alpha: blueprint.opacity,
                                  ),
                                  colorBlendMode: BlendMode.modulate,
                                ),
                              ),
                            ),
                          if (_showHeatmap)
                            Consumer(
                              builder: (context, ref, _) {
                                final nodes = ref.watch(
                                  speakerLayoutProvider,
                                );
                                final rooms = ref.watch(roomZoneProvider);
                                return Positioned.fill(
                                  child: IgnorePointer(
                                    child: RepaintBoundary(
                                      child: CustomPaint(
                                        painter: _HeatmapPainter(
                                          nodes: nodes,
                                          rooms: rooms,
                                          selectedOctave:
                                              _selectedOctaveFilter,
                                        ),
                                      ),
                                    ),
                                  ),
                                );
                              },
                            ),
                          if (_showTrajectories)
                            Consumer(
                              builder: (context, ref, _) {
                                final trajectories = ref.watch(
                                  trajectoryProvider,
                                );
                                return Stack(
                                  children: [
                                    Positioned.fill(
                                      child: IgnorePointer(
                                        child: RepaintBoundary(
                                          child: CustomPaint(
                                            painter: TrajectoryLayerPainter(
                                              trajectories: trajectories,
                                              rooms: ref.watch(
                                                roomZoneProvider,
                                              ),
                                              speakers: ref.watch(
                                                speakerLayoutProvider,
                                              ),
                                              focusedTrajectoryId: ref.watch(
                                                activeTrajectoryIdProvider,
                                              ),
                                              scaleMeterToPixel: ref
                                                  .read(blueprintProvider)
                                                  .scale,
                                              repaint: Listenable.merge(
                                                trajectories,
                                              ),
                                            ),
                                          ),
                                        ),
                                      ),
                                    ),
                                    ...trajectories.map(
                                      (t) => _DraggableTrajectoryPathWidget(
                                        key: ValueKey('path_${t.id}'),
                                        trajectory: t,
                                        transformationController:
                                            _transformationController,
                                        onDragUpdate:
                                            _syncSpatialConfigRealtime,
                                        onLongPress: () => _editTrajectory(t),
                                      ),
                                    ),
                                    ...trajectories.expand((t) {
                                      return t.waypoints.asMap().entries.map((
                                        entry,
                                      ) {
                                        final idx = entry.key;
                                        final wp = entry.value;
                                         return _DraggableWaypointWidget(
                                           key: ValueKey('${t.id}_$idx'),
                                           trajectory: t,
                                           waypointIndex: idx,
                                           waypoint: wp,
                                           transformationController:
                                               _transformationController,
                                           onDragUpdate:
                                               _syncSpatialConfigRealtime,
                                         );
                                       });
                                     }),
                                   ],
                                 );
                               },
                             ),
                           if (_showRooms)
                             Consumer(
                               builder: (context, ref, _) {
                                 final rooms = ref.watch(roomZoneProvider);
                                 final nodes = ref.watch(
                                   speakerLayoutProvider,
                                 );
                                 return Positioned.fill(
                                   child: Stack(
                                     clipBehavior: Clip.none,
                                     children: rooms.map((room) {
                                       final containedSpeakers =
                                           _getSpeakersInRoom(room, nodes);
                                       return RoomZoneWidget(
                                         key: ValueKey(room.id),
                                         room: room,
                                         containedSpeakers: containedSpeakers,
                                         transformationController:
                                             _transformationController,
                                         isSelected:
                                             _selectedRoomId == room.id,
                                         onEdit: () => _editRoom(room),
                                         onDragUpdate:
                                             _syncSpatialConfigRealtime,
                                         onInteractionStart: () => setState(() {
                                           _isRoomInteracting = true;
                                           _selectedRoomId = room.id;
                                         }),
                                         onInteractionEnd: () => setState(
                                           () => _isRoomInteracting = false,
                                         ),
                                       );
                                     }).toList(),
                                   ),
                                 );
                               },
                             ),
                            if (_showSpeakers)
                              Consumer(
                                builder: (context, ref, _) {
                                  final nodes = ref.watch(
                                    speakerLayoutProvider,
                                  );
                                  final rooms = ref.watch(roomZoneProvider);
                                  return Stack(
                                    clipBehavior: Clip.none,
                                    children: nodes.map((node) {
                                      final isDuplicate = nodes
                                          .where(
                                            (n) =>
                                                n.id != node.id &&
                                                n.channel == node.channel,
                                          )
                                          .isNotEmpty;
                                      final roomColor = _getRoomColorForSpeaker(
                                        node,
                                        rooms,
                                      );
                                      return _DraggableSpeakerWidget(
                                        key: ValueKey(node.id),
                                        node: node,
                                        roomColor:
                                            roomColor ?? AppColors.primaryNeon,
                                        isDuplicate: isDuplicate,
                                        transformationController:
                                            _transformationController,
                                        onEdit: () => _editSpeaker(node),
                                      );
                                    }).toList(),
                                  );
                                },
                              ),
                            if (_isMeasuringScale)
                              Positioned.fill(
                                child: Stack(
                                  children: [
                                    MouseRegion(
                                      cursor: SystemMouseCursors.precise,
                                      child: GestureDetector(
                                        behavior: HitTestBehavior.opaque,
                                        onPanStart: (details) {
                                          setState(() {
                                            _measureStart =
                                                details.localPosition;
                                            _measureEnd = details.localPosition;
                                          });
                                        },
                                        onPanUpdate: (details) {
                                          setState(() {
                                            _measureEnd = details.localPosition;
                                          });
                                        },
                                        onPanEnd: (details) {
                                          if (_measureStart != null &&
                                              _measureEnd != null) {
                                            _finishMeasurement();
                                          }
                                        },
                                        child: CustomPaint(
                                          painter: _MeasurementPainter(
                                            _measureStart,
                                            _measureEnd,
                                          ),
                                          size: Size.infinite,
                                        ),
                                      ),
                                    ),
                                    if (_measureEnd != null)
                                      Positioned(
                                        left: _measureEnd!.dx - 40,
                                        top: _measureEnd!.dy - 90,
                                        child: IgnorePointer(
                                          child: RawMagnifier(
                                            decoration:
                                                const MagnifierDecoration(
                                                  shape: CircleBorder(
                                                    side: BorderSide(
                                                      color: Colors.cyanAccent,
                                                      width: 2,
                                                    ),
                                                  ),
                                                ),
                                            size: const Size(80, 80),
                                            magnificationScale: 2.0,
                                            focalPointOffset: const Offset(
                                              0,
                                              50,
                                            ),
                                          ),
                                        ),
                                      ),
                                  ],
                                ),
                              ),
                          ],
                        ),
                      ),
                    ),
                  ),
                if (_isSidebarOpen)
                  Positioned.fill(
                    child: TrajectorySidebarWidget(
                      onClose: () {
                        setState(() {
                          _isSidebarOpen = false;
                        });
                      },
                    ),
                  ),
                const Positioned.fill(child: TrajectoryEditorToolbar()),
              ],
            );
          },
        ),
        floatingActionButton: PopupMenuButton<String>(
          onSelected: (value) {
            if (value == 'speaker') {
              _addSpeaker();
            } else if (value == 'room') {
              _addRoom();
            } else if (value == 'trajectory') {
              _addTrajectory();
            } else if (value == 'measure') {
              setState(() {
                _isMeasuringScale = true;
                _measureStart = null;
                _measureEnd = null;
              });
              ScaffoldMessenger.of(context).showSnackBar(
                const SnackBar(content: Text('도면상에 두 점을 드래그하여 기준선을 그려주세요.')),
              );
            }
          },
          itemBuilder: (BuildContext context) => [
            const PopupMenuItem(value: 'speaker', child: Text('스피커 추가')),
            const PopupMenuItem(value: 'room', child: Text('룸 구역 추가')),
            const PopupMenuItem(value: 'trajectory', child: Text('오디오 궤도 추가')),
            const PopupMenuItem(value: 'measure', child: Text('도면 스케일 측정')),
          ],
          child: FloatingActionButton(
            onPressed: null,
            backgroundColor: AppColors.primaryNeon,
            child: const Icon(Icons.add, color: Colors.black),
          ),
        ),
        bottomNavigationBar: Container(
          height: 120,
          decoration: const BoxDecoration(
            color: AppColors.cardSurface,
            border: Border(top: BorderSide(color: Colors.white24)),
          ),
          child: Row(
            children: [
              Expanded(
                flex: 2,
                child: Padding(
                  padding: const EdgeInsets.all(8.0),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text(
                        'EBU R128 피크 미터',
                        style: TextStyle(
                          color: AppColors.primaryNeon,
                          fontWeight: FontWeight.bold,
                          fontSize: 13,
                        ),
                      ),
                      const SizedBox(height: 8),
                      Expanded(
                        child: Row(
                          mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                          children: List.generate(
                            8,
                            (index) => NeonVUMeter(outputChannel: index),
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ), // Row
        ), // Container
      ), // Scaffold
    ); // GestureDetector
  }
}

class _DraggableSpeakerWidget extends ConsumerStatefulWidget {
  final SpeakerNode node;
  final Color? roomColor;
  final bool isDuplicate;
  final TransformationController transformationController;
  final VoidCallback? onEdit;

  const _DraggableSpeakerWidget({
    super.key,
    required this.node,
    this.roomColor,
    required this.isDuplicate,
    required this.transformationController,
    this.onEdit,
  });

  @override
  ConsumerState<_DraggableSpeakerWidget> createState() =>
      _DraggableSpeakerWidgetState();
}

class _DraggableSpeakerWidgetState
    extends ConsumerState<_DraggableSpeakerWidget> {
  late double _localX;
  late double _localY;
  bool _isDragging = false;
  double _initialTouchAngle = 0.0;
  double _initialSpeakerRotation = 0.0;

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
    // The speaker icon center relative to the 100x120 container
    const originOffset = Offset(50, 43);

    return Positioned(
      left: _localX,
      top: _localY,
      child: Transform.rotate(
        angle: widget.node.rotation * math.pi / 180.0,
        alignment: Alignment.topLeft,
        origin: originOffset,
        child: SizedBox(
          width: 100,
          height: 120,
          child: Stack(
            clipBehavior: Clip.none,
            alignment: Alignment.center,
            children: [
              // Draggable Speaker Box Body
              Positioned.fill(
                child: GestureDetector(
                  onPanStart: (_) => setState(() => _isDragging = true),
                  onPanUpdate: (details) {
                    final scale = widget.transformationController.value
                        .getMaxScaleOnAxis();
                    final currentScale = scale > 0 ? scale : 1.0;
                    final rad = widget.node.rotation * math.pi / 180.0;
                    final localDx = details.delta.dx / currentScale;
                    final localDy = details.delta.dy / currentScale;

                    final globalDx =
                        localDx * math.cos(rad) - localDy * math.sin(rad);
                    final globalDy =
                        localDx * math.sin(rad) + localDy * math.cos(rad);

                    setState(() {
                      _localX = (_localX + globalDx).clamp(
                        0.0,
                        _canvasWidth - _speakerSize,
                      );
                      _localY = (_localY + globalDy).clamp(
                        0.0,
                        _canvasHeight - _speakerSize,
                      );
                    });
                    ref
                        .read(speakerLayoutProvider.notifier)
                        .updateSpeaker(
                          widget.node.copyWith(x: _localX, y: _localY),
                          immediate: false,
                        );
                  },
                  onPanEnd: (details) {
                    final snappedX =
                        (_localX / ref.read(blueprintProvider).scale).round() *
                        ref.read(blueprintProvider).scale;
                    final snappedY =
                        (_localY / ref.read(blueprintProvider).scale).round() *
                        ref.read(blueprintProvider).scale;
                    final updated = widget.node.copyWith(
                      x: snappedX.clamp(0.0, _canvasWidth - _speakerSize),
                      y: snappedY.clamp(0.0, _canvasHeight - _speakerSize),
                    );
                    setState(() {
                      _localX = updated.x;
                      _localY = updated.y;
                      _isDragging = false;
                    });
                    ref
                        .read(speakerLayoutProvider.notifier)
                        .updateSpeaker(updated, immediate: true);
                  },
                  child: SpeakerNodeWidget(
                    node: widget.node,
                    roomColor: widget.roomColor,
                    onChannelChanged: (ch) {
                      ref
                          .read(speakerLayoutProvider.notifier)
                          .updateSpeaker(widget.node.copyWith(channel: ch));
                    },
                    onDelete: () {
                      ref
                          .read(speakerLayoutProvider.notifier)
                          .removeSpeaker(widget.node.id);
                    },
                    onEdit: widget.onEdit,
                    isDuplicateChannel: widget.isDuplicate,
                  ),
                ),
              ),
              // Speaker Rotation Handle Knob
              Positioned(
                top: 4,
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onPanStart: (details) {
                    final renderBox = context.findRenderObject() as RenderBox?;
                    if (renderBox != null) {
                      final globalCenter = renderBox.localToGlobal(
                        originOffset,
                      );
                      _initialTouchAngle = math.atan2(
                        details.globalPosition.dy - globalCenter.dy,
                        details.globalPosition.dx - globalCenter.dx,
                      );
                      _initialSpeakerRotation = widget.node.rotation;
                    }
                  },
                  onPanUpdate: (details) {
                    final renderBox = context.findRenderObject() as RenderBox?;
                    if (renderBox != null) {
                      final globalCenter = renderBox.localToGlobal(
                        originOffset,
                      );
                      final currentTouchAngle = math.atan2(
                        details.globalPosition.dy - globalCenter.dy,
                        details.globalPosition.dx - globalCenter.dx,
                      );
                      final deltaAngle =
                          (currentTouchAngle - _initialTouchAngle) *
                          180 /
                          math.pi;
                      double newRotation =
                          (_initialSpeakerRotation + deltaAngle) % 360;
                      if (newRotation < 0) newRotation += 360;

                      ref
                          .read(speakerLayoutProvider.notifier)
                          .updateSpeaker(
                            widget.node.copyWith(rotation: newRotation),
                            immediate: false,
                          );
                    }
                  },
                  child: Container(
                    width: 22,
                    height: 22,
                    decoration: BoxDecoration(
                      color: AppColors.primaryNeon,
                      shape: BoxShape.circle,
                      border: Border.all(color: Colors.black, width: 2.0),
                      boxShadow: const [
                        BoxShadow(color: Colors.black54, blurRadius: 4),
                      ],
                    ),
                    child: const Icon(
                      Icons.rotate_right,
                      size: 12,
                      color: Colors.black,
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _DraggableWaypointWidget extends ConsumerStatefulWidget {
  final TrajectoryModel trajectory;
  final int waypointIndex;
  final Waypoint waypoint;
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
  ConsumerState<_DraggableWaypointWidget> createState() =>
      _DraggableWaypointWidgetState();
}

class _DraggableWaypointWidgetState
    extends ConsumerState<_DraggableWaypointWidget> {
  late double _localX;
  late double _localY;

  @override
  void initState() {
    super.initState();
    _localX = widget.waypoint.position.dx * ref.read(blueprintProvider).scale;
    _localY = widget.waypoint.position.dy * ref.read(blueprintProvider).scale;
  }

  @override
  void didUpdateWidget(covariant _DraggableWaypointWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    _localX = widget.waypoint.position.dx * ref.read(blueprintProvider).scale;
    _localY = widget.waypoint.position.dy * ref.read(blueprintProvider).scale;
  }

  @override
  Widget build(BuildContext context) {
    return Positioned(
      left: _localX - 20,
      top: _localY - 20,
      child: GestureDetector(
        onPanUpdate: (details) {
          final scale = widget.transformationController.value
              .getMaxScaleOnAxis();
          final currentScale = scale > 0 ? scale : 1.0;
          setState(() {
            _localX = (_localX + details.delta.dx / currentScale).clamp(
              0.0,
              _canvasWidth,
            );
            _localY = (_localY + details.delta.dy / currentScale).clamp(
              0.0,
              _canvasHeight,
            );
          });

          // 실시간으로 선이 따라가도록 Riverpod 업데이트
          final wps = List<Waypoint>.from(widget.trajectory.waypoints);
          wps[widget.waypointIndex] = Waypoint(
            position: Offset(
              _localX / ref.read(blueprintProvider).scale,
              _localY / ref.read(blueprintProvider).scale,
            ),
          );
          ref
              .read(trajectoryProvider.notifier)
              .updateTrajectory(
                widget.trajectory.copyWith(waypoints: wps),
                immediate: true,
              );
          widget.onDragUpdate();
        },
        onPanEnd: (details) {
          // 강제 동기화 보장 (Trailing Edge 유실 방지)
          widget.onDragUpdate();
        },
        onLongPress: () {
          // 길게 누르기시 삭제 (최소 2개의 점은 유지해야 함)
          if (widget.waypointIndex >= widget.trajectory.waypoints.length)
            return;

          if (widget.trajectory.waypoints.length > 2) {
            final wps = List<Waypoint>.from(widget.trajectory.waypoints);
            if (widget.waypointIndex < wps.length) {
              wps.removeAt(widget.waypointIndex);
            }
            ref
                .read(trajectoryProvider.notifier)
                .updateTrajectory(widget.trajectory.copyWith(waypoints: wps));
          } else {
            ref
                .read(trajectoryProvider.notifier)
                .removeTrajectory(widget.trajectory.id);
          }
        },
        onDoubleTap: () {
          // 더블 탭시 웨이포인트 삭제 (최소 2개의 점은 유지해야 함)
          if (widget.waypointIndex >= widget.trajectory.waypoints.length)
            return;

          if (widget.trajectory.waypoints.length > 2) {
            final wps = List<Waypoint>.from(widget.trajectory.waypoints);
            if (widget.waypointIndex < wps.length) {
              wps.removeAt(widget.waypointIndex);
            }
            ref
                .read(trajectoryProvider.notifier)
                .updateTrajectory(widget.trajectory.copyWith(waypoints: wps));
          } else {
            ref
                .read(trajectoryProvider.notifier)
                .removeTrajectory(widget.trajectory.id);
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

class _DraggableTrajectoryPathWidget extends ConsumerStatefulWidget {
  final TrajectoryModel trajectory;
  final TransformationController transformationController;
  final VoidCallback onDragUpdate;
  final VoidCallback? onLongPress;

  const _DraggableTrajectoryPathWidget({
    super.key,
    required this.trajectory,
    required this.transformationController,
    required this.onDragUpdate,
    this.onLongPress,
  });

  @override
  ConsumerState<_DraggableTrajectoryPathWidget> createState() =>
      _DraggableTrajectoryPathWidgetState();
}

class _DraggableTrajectoryPathWidgetState
    extends ConsumerState<_DraggableTrajectoryPathWidget> {
  @override
  Widget build(BuildContext context) {
    if (widget.trajectory.waypoints.isEmpty) return const SizedBox.shrink();

    double minX = double.infinity, minY = double.infinity;
    double maxX = -double.infinity, maxY = -double.infinity;

    for (var wp in widget.trajectory.waypoints) {
      final px = wp.position.dx * ref.read(blueprintProvider).scale;
      final py = wp.position.dy * ref.read(blueprintProvider).scale;
      if (px < minX) minX = px;
      if (py < minY) minY = py;
      if (px > maxX) maxX = px;
      if (py > maxY) maxY = py;
    }

    const padding = 16.0;
    minX = (minX - padding).clamp(0.0, _canvasWidth);
    minY = (minY - padding).clamp(0.0, _canvasHeight);
    maxX = (maxX + padding).clamp(minX + 32, _canvasWidth);
    maxY = (maxY + padding).clamp(minY + 32, _canvasHeight);

    final width = maxX - minX;
    final height = maxY - minY;

    return Positioned(
      left: minX,
      top: minY,
      width: width,
      height: height,
      child: MouseRegion(
        cursor: SystemMouseCursors.move,
        child: GestureDetector(
          behavior: HitTestBehavior.translucent,
          onPanUpdate: (details) {
            final scale = widget.transformationController.value
                .getMaxScaleOnAxis();
            final currentScale = scale > 0 ? scale : 1.0;
            final dxMeter =
                details.delta.dx /
                currentScale /
                ref.read(blueprintProvider).scale;
            final dyMeter =
                details.delta.dy /
                currentScale /
                ref.read(blueprintProvider).scale;

            final updatedWaypoints = widget.trajectory.waypoints.map((wp) {
              final newX = (wp.position.dx + dxMeter).clamp(
                0.0,
                _canvasWidth / ref.read(blueprintProvider).scale,
              );
              final newY = (wp.position.dy + dyMeter).clamp(
                0.0,
                _canvasHeight / ref.read(blueprintProvider).scale,
              );
              return Waypoint(position: Offset(newX, newY));
            }).toList();

            ref
                .read(trajectoryProvider.notifier)
                .updateTrajectory(
                  widget.trajectory.copyWith(waypoints: updatedWaypoints),
                  immediate: true,
                );
            widget.onDragUpdate();
            widget.onDragUpdate();
          },
          onPanEnd: (_) => widget.onDragUpdate(),
          onLongPress: widget.onLongPress,
          onDoubleTapDown: (details) {
            final renderBox = context.findRenderObject() as RenderBox?;
            if (renderBox != null) {
              final localPos = renderBox.globalToLocal(details.globalPosition);
              final canvasX = minX + localPos.dx;
              final canvasY = minY + localPos.dy;

              final newWaypoint = Waypoint(
                position: Offset(
                  canvasX / ref.read(blueprintProvider).scale,
                  canvasY / ref.read(blueprintProvider).scale,
                ),
              );
              final wps = List<Waypoint>.from(widget.trajectory.waypoints);
              wps.add(newWaypoint);

              ref
                  .read(trajectoryProvider.notifier)
                  .updateTrajectory(widget.trajectory.copyWith(waypoints: wps));
            }
          },
          child: Container(color: Colors.transparent),
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
    final double safeScale =
        (scale > 0 && !scale.isNaN && !scale.isInfinite) ? scale : 40.0;
    final paint = Paint()
      ..color = Colors.white12
      ..strokeWidth = 1.0;

    final textPainter = TextPainter(textDirection: TextDirection.ltr);

    for (double i = 0; i <= size.width; i += safeScale) {
      canvas.drawLine(Offset(i, 0), Offset(i, size.height), paint);

      if (i > 0 && i % (safeScale * 5) == 0) {
        final meters = (i / safeScale).toStringAsFixed(1);
        textPainter.text = TextSpan(
          text: '${meters}m',
          style: const TextStyle(color: Colors.white38, fontSize: 10),
        );
        textPainter.layout();
        textPainter.paint(canvas, Offset(i + 2, 2));
      }
    }
    for (double i = 0; i <= size.height; i += safeScale) {
      canvas.drawLine(Offset(0, i), Offset(size.width, i), paint);

      if (i > 0 && i % (safeScale * 5) == 0) {
        final meters = (i / safeScale).toStringAsFixed(1);
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
  bool shouldRepaint(covariant _GridPainter oldDelegate) =>
      oldDelegate.scale != scale;
}

class _HeatmapPainter extends CustomPainter {
  final List<SpeakerNode> nodes;
  final List<RoomZone> rooms;
  final String selectedOctave;

  _HeatmapPainter({
    required this.nodes,
    required this.rooms,
    this.selectedOctave = 'All',
  });

  @override
  void paint(Canvas canvas, Size size) {
    for (var node in nodes) {
      if (node.x.isNaN ||
          node.y.isNaN ||
          node.x.isInfinite ||
          node.y.isInfinite) {
        continue;
      }

      // Offset matches the visual center of the speaker icon inside the 100x120 container
      final center = Offset(node.x + 50, node.y + 43);
      final double clampedPitch = node.pitchTilt.clamp(-85.0, 85.0);
      final double rotRad =
          (node.rotation - 90.0) * math.pi / 180.0; // Front axis

      // 1. Octave-dependent directivity angle Q(f)
      final double effectiveDispAngle = (selectedOctave == 'All'
              ? node.dispersionAngle
              : node.getEffectiveDispersionAngle(selectedOctave))
          .clamp(5.0, 180.0);
      final double dispRad = effectiveDispAngle * math.pi / 180.0;

      // 2. 3D Pitch Tilt Projection (Elliptical Footprint)
      final double pitchRad = clampedPitch * math.pi / 180.0;
      final double tanVal = math.tan(pitchRad);
      if (tanVal.isNaN || tanVal.isInfinite) continue;

      final double lengthExtension =
          (node.heightZ * tanVal * 40.0).clamp(-2000.0, 2000.0);
      final Offset projectedCenter = center; // Anchor at the speaker
      final double dist =
          (node.dispersionDistance + lengthExtension).clamp(10.0, 3000.0);

      // 3. Outer Elliptical Dispersion Beam Contour (-6dB)
      canvas.save();
      canvas.translate(projectedCenter.dx, projectedCenter.dy);
      canvas.rotate(rotRad);

      final double axisX = (dist * math.cos(pitchRad)).clamp(5.0, 3000.0);
      final double axisY = (dist * math.sin(dispRad / 2)).clamp(5.0, 3000.0);

      if (axisX.isNaN ||
          axisX.isInfinite ||
          axisY.isNaN ||
          axisY.isInfinite) {
        canvas.restore();
        continue;
      }

      final Path ellipticalPath = Path()
        ..moveTo(0, 0)
        ..lineTo(axisX, -axisY)
        ..arcToPoint(
          Offset(axisX, axisY),
          radius: Radius.elliptical(axisX, axisY),
          clockwise: true,
        )
        ..close();

      final outerGradient = RadialGradient(
        colors: [
          AppColors.primaryNeon.withValues(alpha: 0.45),
          Colors.orangeAccent.withValues(alpha: 0.25),
          Colors.redAccent.withValues(alpha: 0.08),
          Colors.transparent,
        ],
        stops: const [0.0, 0.4, 0.75, 1.0],
      );

      final Paint conePaint = Paint()
        ..shader = outerGradient.createShader(
          Rect.fromLTWH(0, -axisY, axisX, axisY * 2),
        )
        ..style = PaintingStyle.fill;

      canvas.drawPath(ellipticalPath, conePaint);

      // On-Axis Inner High-SPL Core Beam (-3dB)
      final Path innerCorePath = Path()
        ..moveTo(0, 0)
        ..lineTo(axisX * 0.6, -axisY * 0.5)
        ..arcToPoint(
          Offset(axisX * 0.6, axisY * 0.5),
          radius: Radius.elliptical(axisX * 0.6, axisY * 0.5),
          clockwise: true,
        )
        ..close();

      final Paint innerCorePaint = Paint()
        ..color = AppColors.primaryNeon.withValues(alpha: 0.35)
        ..style = PaintingStyle.fill;

      canvas.drawPath(innerCorePath, innerCorePaint);

      // Iso-SPL Elliptical Contours (-3dB, -6dB, -12dB)
      final arcStrokePaint = Paint()
        ..color = AppColors.primaryNeon.withValues(alpha: 0.6)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.5;

      for (var ratio in [0.33, 0.66, 1.0]) {
        canvas.drawArc(
          Rect.fromCenter(
            center: Offset(axisX * ratio / 2, 0),
            width: axisX * ratio,
            height: axisY * 2 * ratio,
          ),
          -dispRad / 2,
          dispRad,
          false,
          arcStrokePaint,
        );
      }

      // Aiming axis line
      final axisPaint = Paint()
        ..color = Colors.white.withValues(alpha: 0.8)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.2;
      canvas.drawLine(Offset.zero, Offset(axisX, 0), axisPaint);

      canvas.restore();

      // 4. Wall Transmission Loss (TL) Cross-talk Leakage Visualization
      for (var room in rooms) {
        if (room.containsPoint(projectedCenter.dx, projectedCenter.dy)) {
          // Inside a room, draw subtle attenuation glow for wall leakage
          final tlFactor = math
              .pow(10, -room.wallTransmissionLoss / 20.0)
              .toDouble(); // TL attenuation
          final Paint leakagePaint = Paint()
            ..color = Colors.purpleAccent.withValues(alpha: 0.15 * tlFactor)
            ..style = PaintingStyle.stroke
            ..strokeWidth = 4.0;
          canvas.drawRect(
            Rect.fromLTWH(room.x, room.y, room.width, room.height),
            leakagePaint,
          );
        }
      }
    }
  }

  @override
  bool shouldRepaint(covariant _HeatmapPainter oldDelegate) => true;
}

class _MeasurementPainter extends CustomPainter {
  final Offset? start;
  final Offset? end;

  _MeasurementPainter(this.start, this.end);

  @override
  void paint(Canvas canvas, Size size) {
    if (start != null && end != null) {
      final paint = Paint()
        ..color = AppColors.primaryNeon
        ..strokeWidth = 3.0
        ..style = PaintingStyle.stroke;
      canvas.drawLine(start!, end!, paint);

      final circlePaint = Paint()
        ..color = Colors.white
        ..style = PaintingStyle.fill;
      canvas.drawCircle(start!, 5.0, circlePaint);
      canvas.drawCircle(end!, 5.0, circlePaint);
    }
  }

  @override
  bool shouldRepaint(covariant _MeasurementPainter oldDelegate) {
    return oldDelegate.start != start || oldDelegate.end != end;
  }
}
