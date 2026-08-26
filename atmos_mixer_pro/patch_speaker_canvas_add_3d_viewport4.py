import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# We can replace the top-level `body: Stack(` instead.
# If we replace `body: Stack( children: [ Positioned.fill(child: LayoutBuilder(...)), ... (floating buttons) ] )`
# with:
# `body: Column( children: [ Expanded(flex: 3, child: Stack(children: [ Positioned.fill(child: LayoutBuilder(...)), ... (floating buttons) ])), Container(height: 2, color: Colors.blueAccent), Expanded(flex: 2, child: Room3DViewport()) ])`

# This is much easier and safer! We just wrap the ENTIRE `body: Stack(`

body_target = """        body: Stack(
          children: ["""

# Wait, `body: Stack(` is at line 1868.
# The `Scaffold` ends at line 2275.
import os
os.system('sed -n "1865,1875p" lib/features/exhibition/screens/speaker_canvas_screen.dart')

