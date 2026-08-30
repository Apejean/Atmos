import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/room_setup_window.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart';

class SpeakerCanvasScreen extends ConsumerStatefulWidget {
  const SpeakerCanvasScreen({super.key});

  @override
  ConsumerState<SpeakerCanvasScreen> createState() => _SpeakerCanvasScreenState();
}

class _SpeakerCanvasScreenState extends ConsumerState<SpeakerCanvasScreen> {
  String? _selectedRoomId;
  String? _selectedInspectorSpeakerId;
  bool _isRoomSetupOpen = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _syncDefaultRooms();
    });
  }

  void _syncDefaultRooms() {
    final rooms = ref.read(roomZoneProvider);

    if (rooms.isEmpty) {
      final defaultRoom = RoomZone(
        id: 'room_1',
        label: 'Room 1 (Main Hall)',
        x: 0,
        y: 0,
        width: 6.0,
        height: 4.5,
        color: 0xFF0284C7,
        physicalWidth: 6.0,
        physicalHeight: 4.5,
        ceilingHeight: 3.0,
        earLevel: 1.2,
      );
      ref.read(roomZoneProvider.notifier).addRoomZone(defaultRoom);
      setState(() {
        _selectedRoomId = defaultRoom.id;
      });
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
        title: const Text(
          'Exhibition Canvas (3D Space)',
          style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold, color: Colors.white),
        ),
        actions: [
          Padding(
            padding: const EdgeInsets.only(right: 16),
            child: OutlinedButton.icon(
              icon: const Icon(Icons.picture_as_pdf_rounded, size: 16, color: Colors.lightBlueAccent),
              label: const Text(
                'Export PDF Report',
                style: TextStyle(
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                  color: Colors.lightBlueAccent,
                ),
              ),
              style: OutlinedButton.styleFrom(
                side: BorderSide(color: Colors.lightBlueAccent.withValues(alpha: 0.6)),
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
                backgroundColor: const Color(0xFF161E28).withValues(alpha: 0.8),
              ),
              onPressed: () {
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(
                    content: Text('PDF 음향 리포트 내보내기 기능이 준비 중입니다.'),
                    duration: Duration(seconds: 2),
                    backgroundColor: Color(0xFF1E293B),
                  ),
                );
              },
            ),
          ),
        ],
        backgroundColor: const Color(0xFF0D1219),
        elevation: 4,
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(48),
          child: Container(
            height: 48,
            color: const Color(0xFF131923),
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
            alignment: Alignment.centerLeft,
            child: _buildTopPinnedRoomTabs(rooms, activeRoom),
          ),
        ),
      ),
      body: Stack(
        children: [
          // 1. 100% 3D WebGL Three.js Studio Engine
          Positioned.fill(
            child: Dynamic3DRoom(
              activeRoom: activeRoom,
              selectedSpeakerId: _selectedInspectorSpeakerId,
              onOpenRoomSetup: () => setState(() => _isRoomSetupOpen = true),
              onSpeakerTapped: (id) {
                setState(() {
                  _selectedInspectorSpeakerId = id;
                });
              },
            ),
          ),

          // 2. Left-Bottom Room Setup Toggle Button
          if (!_isRoomSetupOpen && activeRoom != null)
            Positioned(
              left: 16,
              bottom: 24,
              child: OutlinedButton.icon(
                icon: const Icon(Icons.tune_rounded, size: 16, color: Colors.lightBlueAccent),
                label: Text(
                  'ROOM SETUP: ${activeRoom.label}',
                  style: const TextStyle(
                    fontSize: 12,
                    fontWeight: FontWeight.bold,
                    letterSpacing: 0.5,
                  ),
                ),
                style: OutlinedButton.styleFrom(
                  foregroundColor: Colors.lightBlueAccent,
                  backgroundColor: const Color(0xFF161E28).withValues(alpha: 0.95),
                  side: BorderSide(color: Colors.lightBlueAccent.withValues(alpha: 0.7), width: 1.2),
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                  elevation: 6,
                ),
                onPressed: () => setState(() => _isRoomSetupOpen = true),
              ),
            ),

          // 3. Left-Bottom Room Setup Window Modal
          if (_isRoomSetupOpen && activeRoom != null)
            Positioned(
              left: 16,
              bottom: 24,
              child: RoomSetupWindow(
                room: activeRoom,
                onApply: (updated) {
                  final bp = ref.read(blueprintProvider);
                  final pixelW = updated.physicalWidth * bp.scale;
                  final pixelH = updated.physicalHeight * bp.scale;
                  final finalUpdated = updated.copyWith(width: pixelW, height: pixelH);

                  ref.read(roomZoneProvider.notifier).updateRoomZone(finalUpdated, immediate: true);
                  ref.read(blueprintProvider.notifier).setCanvasDimensions(
                        finalUpdated.physicalWidth,
                        finalUpdated.physicalHeight,
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

          // 4. Right Side Speaker Inspector Panel
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
}
