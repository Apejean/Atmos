with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

target = """          title: Row(
            children: [
              const Text('Exhibition Canvas'),
              
              const Spacer(),"""

replacement = """          title: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                const Text('Exhibition Canvas'),
                const SizedBox(width: 24),"""

if target in content:
    content = content.replace(target, replacement)
else:
    print("target not found")

target2 = """              const Spacer(),
              Consumer(
                builder: (context, ref, child) {"""

replacement2 = """              const SizedBox(width: 24),
              Consumer(
                builder: (context, ref, child) {"""

if target2 in content:
    content = content.replace(target2, replacement2)
else:
    print("target2 not found")

target3 = """                },
              ),
              const SizedBox(width: 8),
            ],
          ),
        ),
        body: Stack("""

replacement3 = """                },
              ),
              const SizedBox(width: 8),
            ],
          ),
          ),
        ),
        body: Stack("""

if target3 in content:
    content = content.replace(target3, replacement3)
else:
    print("target3 not found")

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
