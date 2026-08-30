import re

def main():
    path = "lib/features/exhibition/screens/speaker_canvas_screen.dart"
    with open(path, "r") as f:
        content = f.read()

    old_code = """          Padding(
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
          ),"""

    content = content.replace(old_code, "")

    with open(path, "w") as f:
        f.write(content)

main()
