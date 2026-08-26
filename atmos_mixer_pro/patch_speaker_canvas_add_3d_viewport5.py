import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Replace body: Stack(
# With body: Column( children: [ Expanded(flex: 3, child: Stack(

content = content.replace(
    '''        body: Stack(
          children: [''',
    '''        body: Column(
          children: [
            Expanded(
              flex: 3,
              child: Stack(
                children: ['''
)

# And then find the end of the `Scaffold` body to close it.
# The Scaffold ends at `      ), // Scaffold` around line 2275.

scaffold_end_target = """      ),
    ); // GestureDetector"""

scaffold_end_replacement = """              ),
            ),
            Container(height: 2, color: const Color(0xFF3F556D)),
            const Expanded(
              flex: 2,
              child: Room3DViewport(),
            ),
          ],
        ),
      ),
    ); // GestureDetector"""

content = content.replace(scaffold_end_target, scaffold_end_replacement)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
