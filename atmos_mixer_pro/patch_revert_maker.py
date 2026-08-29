import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Replace the body: _is3DMode ? ... : Stack( ... ) with just the Stack and Transform
# Let's use regex.
pattern = r"appBar: AppBar\(.*?body: _is3DMode.*? \: Stack\(\n          children: \[\n            Positioned\.fill\("

replacement = """appBar: AppBar(
          title: const Text('Exhibition Canvas', style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold)),
          backgroundColor: Colors.black,
        ),
        body: Stack(
          children: [
            Positioned.fill("""

content = re.sub(pattern, replacement, content, flags=re.DOTALL)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

