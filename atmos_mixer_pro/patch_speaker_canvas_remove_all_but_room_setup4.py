import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Replace the entire AppBar safely.

appbar_start_pattern = """        appBar: AppBar(
          title: Row(
            children: [
              const Text('Exhibition Canvas'),
              const SizedBox(width: 8),
              Expanded(child: SingleChildScrollView(scrollDirection: Axis.horizontal, child: Row(mainAxisSize: MainAxisSize.max, mainAxisAlignment: MainAxisAlignment.end, children: ["""

# It's better to just use regex to replace everything between `appBar: AppBar(` and `backgroundColor: Colors.black,`

content = re.sub(
    r'appBar: AppBar\(.*backgroundColor: Colors\.black,',
    r"appBar: AppBar(\n          title: const Text('Exhibition Canvas', style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold)),\n          backgroundColor: Colors.black,",
    content,
    flags=re.DOTALL
)

# And replace `floatingActionButton:`
content = re.sub(
    r'floatingActionButton: PopupMenuButton<String>\(.*?\); // GestureDetector',
    r'); // GestureDetector',
    content,
    flags=re.DOTALL
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
