import re

with open("lib/features/dashboard/widgets/room_calibration_wizard_modal.dart", "r") as f:
    content = f.read()

# Change state variable
content = content.replace("String _selectedMic = 'System Default Microphone';", "int _selectedMicChannel = 1;")
content = content.replace("value: _selectedMic,", "value: _selectedMicChannel,")

# Change dropdown options
old_dropdown_items = "items: ['System Default Microphone', 'UMIK-1 (USB)', 'Earthworks M30'].map((m) => DropdownMenuItem(value: m, child: Text(m))).toList(),"
new_dropdown_items = "items: List.generate(32, (index) => index + 1).map((ch) => DropdownMenuItem(value: ch, child: Text('Input Ch $ch'))).toList(),"
content = content.replace(old_dropdown_items, new_dropdown_items)

# Change label
content = content.replace("const Text('Measurement Microphone', style: TextStyle(color: Colors.white70)),", "const Text('측정 마이크 입력 채널 (Input Channel)', style: TextStyle(color: Colors.white70)),")

# Change onChanged handler
old_onchanged = "onChanged: (v) => setState(() => _selectedMic = v!),"
new_onchanged = "onChanged: (v) => setState(() => _selectedMicChannel = v!),"
content = content.replace(old_onchanged, new_onchanged)

# If _selectedMic is used elsewhere, replace it (though it doesn't seem to be, let's just make sure)
content = content.replace("_selectedMic", "_selectedMicChannel")

with open("lib/features/dashboard/widgets/room_calibration_wizard_modal.dart", "w") as f:
    f.write(content)
