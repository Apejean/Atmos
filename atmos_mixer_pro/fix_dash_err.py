with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

target = """            ],
          ),
        ],
        ),
      ),
    );
  }"""

replacement = """            ],
          ),
        ],
      ),
    );
  }"""

if target in content:
    content = content.replace(target, replacement)

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
