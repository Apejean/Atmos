with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Replace Spacer() with SizedBox(width: 24) in the AppBar title Row
# But ONLY within that specific area to be safe.
# Let's just do a targeted replacement.
target_area = """          title: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                const Text('Exhibition Canvas'),
              
              const Spacer(),"""

replacement = """          title: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                const Text('Exhibition Canvas'),
              
              const SizedBox(width: 24),"""

content = content.replace(target_area, replacement)

# Another Spacer() after Export PDF
target2 = """              const Spacer(),
              Consumer(
                builder: (context, ref, child) {"""

replacement2 = """              const SizedBox(width: 24),
              Consumer(
                builder: (context, ref, child) {"""
content = content.replace(target2, replacement2)

# Also need to close SingleChildScrollView properly.
# The row ends right before `        body: Stack(` or `          ), // AppBar` or whatever follows it.
# Let's see what is after the Row in AppBar.
