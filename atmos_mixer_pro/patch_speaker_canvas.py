import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# 1. 룸 셋업 윈도우 & 버튼을 topPinnedRoomTabs와 뱃지 영역으로 이동
# 우선 기존 하단 룸 셋업 윈도우와 토글 버튼 제거
remove_bottom_setup = """          // 3. 왼쪽 하단 룸 셋업 윈도우 (Bottom-Left Room Setup Window)
          if (_isRoomSetupOpen && activeRoom != null)
            Positioned(
              left: 16,
              bottom: 16,
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

          // 4. 왼쪽 하단 룸 셋업 토글 버튼 (Room Setup Toggle Button)
          if (!_isRoomSetupOpen)
            Positioned(
              left: 16,
              bottom: 24,
              child: OutlinedButton.icon(
                icon: const Icon(Icons.tune_rounded, size: 16, color: Colors.lightBlueAccent),
                label: Text(
                  'ROOM SETUP: ${activeRoom?.label ?? "Room"}',
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
                  shadowColor: Colors.black,
                  elevation: 6,
                ),
                onPressed: () => setState(() => _isRoomSetupOpen = true),
              ),
            ),"""

content = content.replace(remove_bottom_setup, '')


# _buildTopPinnedRoomTabs 옆에 Room Setup 버튼 배치
tab_code_old = """  Widget _buildTopPinnedRoomTabs(List<RoomZone> rooms, RoomZone? activeRoom) {
    if (rooms.isEmpty) return const SizedBox.shrink();

    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: Row(
        children: [
          for (final room in rooms) ...[
            _buildRoomTabItem(room, isSelected: room.id == activeRoom?.id),
            const SizedBox(width: 8),
          ],
        ],
      ),
    );
  }"""

tab_code_new = """  Widget _buildTopPinnedRoomTabs(List<RoomZone> rooms, RoomZone? activeRoom) {
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
  }"""
content = content.replace(tab_code_old, tab_code_new)

# Body Stack 맨 위에 Room Setup Window 띄우기 (위치 변경)
body_stack = """
          // 2. 우측 스피커 인스펙터 패널 (Glassmorphism right side)
"""
new_body_stack = """
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
"""
content = content.replace(body_stack, new_body_stack)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
