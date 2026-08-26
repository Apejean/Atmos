import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Wait, the user wants the ROOM SETUP button on the BOTTOM LEFT, not the top appbar!
# I put it in the AppBar by mistake (and it caused overflow).
# I will remove it from AppBar and put it inside the Stack on the bottom left (floating).

# 1. Remove from AppBar
appbar_button = """              const SizedBox(width: 8),
              OutlinedButton.icon(
                icon: const Icon(Icons.picture_in_picture_alt_outlined, size: 16),
                label: const Text('ROOM SETUP'),
                style: OutlinedButton.styleFrom(
                  foregroundColor: _isRoomSetupOpen ? Colors.white : Colors.lightBlueAccent,
                  backgroundColor: _isRoomSetupOpen ? Colors.lightBlueAccent.withValues(alpha: 0.2) : Colors.transparent,
                  side: BorderSide(color: Colors.lightBlueAccent.withValues(alpha: _isRoomSetupOpen ? 0.8 : 0.4)),
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 0),
                ),
                onPressed: () => setState(() => _isRoomSetupOpen = !_isRoomSetupOpen),
              ),
              const SizedBox(width: 8),"""

content = content.replace(appbar_button, '')

# 2. Add Floating button on bottom left in the Stack
floating_button_code = """
            if (!_isRoomSetupOpen)
              Positioned(
                left: 16,
                bottom: 16,
                child: OutlinedButton.icon(
                  icon: const Icon(Icons.picture_in_picture_alt_outlined, size: 16),
                  label: const Text('ROOM SETUP'),
                  style: OutlinedButton.styleFrom(
                    foregroundColor: Colors.lightBlueAccent,
                    backgroundColor: const Color(0xFF2C394B).withValues(alpha: 0.8),
                    side: const BorderSide(color: Colors.lightBlueAccent),
                    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                  ),
                  onPressed: () => setState(() => _isRoomSetupOpen = true),
                ),
              ),
"""

content = content.replace(
    '''            if (_isRoomSetupOpen)
              Positioned(
                left: 16,
                bottom: 16,
                child: RoomSetupWindow(
                  onClose: () => setState(() => _isRoomSetupOpen = false),
                ),
              ),''',
    '''            if (_isRoomSetupOpen)
              Positioned(
                left: 16,
                bottom: 16,
                child: RoomSetupWindow(
                  onClose: () => setState(() => _isRoomSetupOpen = false),
                ),
              ),''' + floating_button_code
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
