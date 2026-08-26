with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

target = """          title: Row(
            children: [
              const Text('Exhibition Canvas'),"""

replacement = """          title: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                const Text('Exhibition Canvas'),"""

if target in content:
    content = content.replace(target, replacement)
else:
    print("target1 not found")


target2 = """              IconButton(
                icon: const Icon(Icons.delete_sweep, color: Colors.redAccent),
                onPressed: _clearCanvas,
                tooltip: 'Clear Canvas',
              ),
            ],
          ),
          backgroundColor: Colors.black,
        ),
        body: LayoutBuilder("""

replacement2 = """              IconButton(
                icon: const Icon(Icons.delete_sweep, color: Colors.redAccent),
                onPressed: _clearCanvas,
                tooltip: 'Clear Canvas',
              ),
            ],
          ),
          ),
          backgroundColor: Colors.black,
        ),
        body: LayoutBuilder("""

if target2 in content:
    content = content.replace(target2, replacement2)
else:
    print("target2 not found")


# Also replace `const Spacer(),` with `const SizedBox(width: 24),` in AppBar title row.
target_spacer1 = """              const Text('Exhibition Canvas'),
              
              const Spacer(),
              Row("""

replacement_spacer1 = """              const Text('Exhibition Canvas'),
              
              const SizedBox(width: 24),
              Row("""

if target_spacer1 in content:
    content = content.replace(target_spacer1, replacement_spacer1)
else:
    print("target_spacer1 not found")

target_spacer2 = """              ),
              const Spacer(),
              Consumer("""

replacement_spacer2 = """              ),
              const SizedBox(width: 24),
              Consumer("""

if target_spacer2 in content:
    content = content.replace(target_spacer2, replacement_spacer2)
else:
    print("target_spacer2 not found")

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
