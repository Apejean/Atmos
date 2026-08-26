with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

target = """                },
              ),
              const SizedBox(width: 8),
            ],
          ),
        ),
        body: Stack("""

replacement = """                },
              ),
              const SizedBox(width: 8),
            ],
          ),
        ),
        ),
        body: Stack("""

content = content.replace(target, replacement)
with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
