import re

def main():
    path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
    with open(path, "r") as f:
        content = f.read()

    # Need to add state variable _isSnapEnabled
    if "bool _isSnapEnabled" not in content:
        content = content.replace(
            "String? _selectedInspectorSpeakerId;",
            "String? _selectedInspectorSpeakerId;\n  bool _isSnapEnabled = false;"
        )

    # Need to add button in AppBar actions
    old_actions = """        actions: [
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
              ),"""

    new_actions = """        actions: [
          Padding(
            padding: const EdgeInsets.only(right: 8),
            child: OutlinedButton.icon(
              icon: Icon(
                _isSnapEnabled ? Icons.grid_on : Icons.grid_off,
                size: 16,
                color: _isSnapEnabled ? Colors.orangeAccent : Colors.white70,
              ),
              label: Text(
                'Snap: ${_isSnapEnabled ? "ON" : "OFF"}',
                style: TextStyle(
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                  color: _isSnapEnabled ? Colors.orangeAccent : Colors.white70,
                ),
              ),
              style: OutlinedButton.styleFrom(
                foregroundColor: _isSnapEnabled ? Colors.orangeAccent : Colors.white70,
                side: BorderSide(color: _isSnapEnabled ? Colors.orangeAccent : Colors.white30),
              ),
              onPressed: () {
                setState(() {
                  _isSnapEnabled = !_isSnapEnabled;
                });
                ref.read(threeJsEngineProvider).setSnapEnabled(_isSnapEnabled);
              },
            ),
          ),
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
              ),"""
              
    content = content.replace(old_actions, new_actions)

    with open(path, "w") as f:
        f.write(content)

main()
