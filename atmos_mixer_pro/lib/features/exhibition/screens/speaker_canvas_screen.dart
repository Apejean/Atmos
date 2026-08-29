import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/room_setup_window.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';

class SpeakerCanvasScreen extends ConsumerStatefulWidget {
  const SpeakerCanvasScreen({super.key});

  @override
  ConsumerState<SpeakerCanvasScreen> createState() => _SpeakerCanvasScreenState();
}

class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {
  String? _selectedInspectorSpeakerId;
  String? _selectedRoomId;
  bool _isRoomSetupOpen = false;
  bool _showHeatmap = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _ensureDefaultRoom();
    });
  }

  void _ensureDefaultRoom() {
    final rooms = ref.read(roomZoneProvider);
    final config = ref.read(configProvider);

    if (rooms.isEmpty) {
      if (config != null && config.rooms.isNotEmpty) {
        for (final r in config.rooms) {
          final colorInt = int.tryParse(r.colorHex.replaceFirst('#', '0xFF')) ?? 0xFF2196F3;
          final newRoom = RoomZone(
            id: r.id,
            label: r.name,
            x: 0,
            y: 0,
            width: 300,
            height: 225,
            color: colorInt,
            physicalWidth: 6.0,
            physicalHeight: 4.5,
            ceilingHeight: 3.0,
            earLevel: 1.2,
          );
          ref.read(roomZoneProvider.notifier).addRoomZone(newRoom);
        }
        final updatedRooms = ref.read(roomZoneProvider);
        if (updatedRooms.isNotEmpty) {
          setState(() {
            _selectedRoomId = updatedRooms.first.id;
          });
        }
      } else {
        final defaultRoom = RoomZone(
          id: 'room_1',
          label: 'Room 1 (Main Hall)',
          x: 0,
          y: 0,
          width: 300,
          height: 225,
          color: 0xFF2196F3,
          physicalWidth: 6.0,
          physicalHeight: 4.5,
          ceilingHeight: 3.0,
          earLevel: 1.2,
        );
        ref.read(roomZoneProvider.notifier).addRoomZone(defaultRoom);
        setState(() {
          _selectedRoomId = defaultRoom.id;
        });
      }
    } else if (_selectedRoomId == null || !rooms.any((r) => r.id == _selectedRoomId)) {
      setState(() {
        _selectedRoomId = rooms.first.id;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final rooms = ref.watch(roomZoneProvider);
    final activeRoom = rooms.where((r) => r.id == _selectedRoomId).firstOrNull ?? rooms.firstOrNull;

    return Scaffold(
      backgroundColor: AppColors.background,
      appBar: AppBar(
        leading: IconButton(
          icon: const Icon(Icons.arrow_back, color: Colors.white),
          onPressed: () => Navigator.of(context).pop(),
        ),
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              'Exhibition Canvas (3D Space)',
              style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold, color: Colors.white),
            ),
            const SizedBox(height: 4),
            Row(
              children: [
                _buildAppBarButton(
                  icon: Icons.map_rounded,
                  label: 'SPL Heatmap',
                  isActive: _showHeatmap,
                  onTap: () => setState(() => _showHeatmap = !_showHeatmap),
                ),
                const SizedBox(width: 12),
                _buildAppBarButton(
                  icon: Icons.picture_as_pdf_rounded,
                  label: 'Export PDF Report',
                  onTap: () {},
                ),
                const SizedBox(width: 12),
                _buildAppBarButton(
                  icon: Icons.settings_input_antenna_rounded,
                  label: 'Apply 3D Calibration',
                  onTap: () {},
                ),
              ],
            ),
          ],
        ),
        toolbarHeight: 80,
        backgroundColor: const Color(0xFF0D1219),
        elevation: 4,
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(50),
          child: Container(
            height: 50,
            color: const Color(0xFF131923),
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
            alignment: Alignment.centerLeft,
            child: _buildTopPinnedRoomTabs(rooms, activeRoom),
          ),
        ),
      ),
      body: Stack(
        children: [
          // 1. 100% 3D Native Space
          Positioned.fill(
            child: Dynamic3DRoom(
              activeRoom: activeRoom,
              showHeatmap: _showHeatmap,
              selectedSpeakerId: _selectedInspectorSpeakerId,
              onOpenRoomSetup: () => setState(() => _isRoomSetupOpen = true),
              onSpeakerTapped: (id) {
                setState(() {
                  _selectedInspectorSpeakerId = id;
                });
              },
            ),
          ),

          // Room Setup Window (Now floating below tabs)
          if (_isRoomSetupOpen && activeRoom != null)
            Positioned(
              left: 16,
              top: 70,
              child: RoomSetupWindow(
                room: activeRoom,
                onApply: (updated) {
                  ref.read(roomZoneProvider.notifier).updateRoomZone(updated, immediate: true);
                  ref.read(blueprintProvider.notifier).setCanvasDimensions(
                        updated.physicalWidth,
                        updated.physicalHeight,
                      );
                  setState(() {
                    _isRoomSetupOpen = false;
                  });
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(
                      content: Text('${updated.label} 설정이 적용되었습니다.'),
                      duration: const Duration(seconds: 2),
                      backgroundColor: const Color(0xFF1E293B),
                    ),
                  );
                },
                onClose: () => setState(() => _isRoomSetupOpen = false),
              ),
            ),

          // 2. 우측 스피커 인스펙터 패널 (Glassmorphism right side)
          if (_selectedInspectorSpeakerId != null)
            Positioned(
              top: 0,
              bottom: 0,
              right: 0,
              child: SpeakerInspectorPanel(
                speakerId: _selectedInspectorSpeakerId!,
                onClose: () => setState(() => _selectedInspectorSpeakerId = null),
              ),
            ),


        ],
      ),
    );
  }

  Widget _buildTopPinnedRoomTabs(List<RoomZone> rooms, RoomZone? activeRoom) {
    if (rooms.isEmpty) return const SizedBox.shrink();

    return Row(
      children: [
        Expanded(
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                for (final room in rooms) ...[
                  _buildRoomTabItem(room, isSelected: room.id == activeRoom?.id),
                  const SizedBox(width: 8),
                ],
              ],
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildRoomTabItem(RoomZone room, {required bool isSelected}) {
    return InkWell(
      onTap: () {
        setState(() {
          _selectedRoomId = room.id;
        });
      },
      borderRadius: BorderRadius.circular(6),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 7),
        decoration: BoxDecoration(
          color: isSelected
              ? const Color(0xFF1E3A5F)
              : const Color(0xFF161E28).withValues(alpha: 0.8),
          borderRadius: BorderRadius.circular(6),
          border: Border.all(
            color: isSelected
                ? Colors.lightBlueAccent
                : Colors.white.withValues(alpha: 0.1),
            width: isSelected ? 1.5 : 1.0,
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.meeting_room_outlined,
              size: 16,
              color: isSelected ? Colors.lightBlueAccent : Colors.white60,
            ),
            const SizedBox(width: 8),
            Text(
              room.label,
              style: TextStyle(
                color: isSelected ? Colors.white : Colors.white70,
                fontSize: 12,
                fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
              ),
            ),
            const SizedBox(width: 8),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
              decoration: BoxDecoration(
                color: Colors.black.withValues(alpha: 0.35),
                borderRadius: BorderRadius.circular(4),
              ),
              child: Text(
                '${room.physicalWidth.toStringAsFixed(1)}×${room.physicalHeight.toStringAsFixed(1)}m',
                style: TextStyle(
                  color: isSelected ? Colors.lightBlueAccent : Colors.white38,
                  fontSize: 10,
                  fontWeight: FontWeight.w500,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildAppBarButton({required IconData icon, required String label, required VoidCallback onTap, bool isActive = false}) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(4),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        decoration: BoxDecoration(
          color: isActive ? Colors.lightBlueAccent.withValues(alpha: 0.2) : Colors.transparent,
          borderRadius: BorderRadius.circular(4),
          border: Border.all(
            color: isActive ? Colors.lightBlueAccent : Colors.white24,
            width: 1,
          ),
        ),
        child: Row(
          children: [
            Icon(icon, size: 14, color: isActive ? Colors.lightBlueAccent : Colors.white70),
            const SizedBox(width: 6),
            Text(
              label,
              style: TextStyle(
                fontSize: 11,
                color: isActive ? Colors.lightBlueAccent : Colors.white70,
                fontWeight: isActive ? FontWeight.bold : FontWeight.normal,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
