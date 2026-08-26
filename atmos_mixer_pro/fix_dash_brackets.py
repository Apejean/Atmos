import re

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

target = """                ],
              );
            },
          ),
        ],
      ),
      ),
    );
  }"""

replacement = """                ],
              );
            },
          ),
        ],
      ), // Row
      ), // SingleChildScrollView
      ], // Column children
      ), // Column
    ); // Container
  }"""

if target in content:
    content = content.replace(target, replacement)
else:
    print("target not found")

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
