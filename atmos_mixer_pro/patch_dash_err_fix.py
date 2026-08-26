with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

target = """            ],
          ),
          Consumer(
            builder: (context, ref, child) {"""

replacement = """            ],
          ),
          ),
          Consumer(
            builder: (context, ref, child) {"""

if target in content:
    content = content.replace(target, replacement)
else:
    print("Not found")

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
