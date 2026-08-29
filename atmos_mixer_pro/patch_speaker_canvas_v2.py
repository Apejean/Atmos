import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Dynamic3DRoom 에 onOpenRoomSetup 전달
old = """              activeRoom: activeRoom,
              showHeatmap: _showHeatmap,
              selectedSpeakerId: _selectedInspectorSpeakerId,
              onSpeakerTapped: (id) {
                setState(() {
                  _selectedInspectorSpeakerId = id;
                });
              },
            ),"""
new = """              activeRoom: activeRoom,
              showHeatmap: _showHeatmap,
              selectedSpeakerId: _selectedInspectorSpeakerId,
              onOpenRoomSetup: () => setState(() => _isRoomSetupOpen = true),
              onSpeakerTapped: (id) {
                setState(() {
                  _selectedInspectorSpeakerId = id;
                });
              },
            ),"""
content = content.replace(old, new)

# 하단 CH1~CH7 탭 삭제를 위해 바닥부분 UI 제거 확인
# 하단부에 특별한 탭이 있는지 grep 확인해보자.

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
