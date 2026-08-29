import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

find_double_tap = """
      body: Stack(
        children: [
          // 1. WebViewController WebGL Viewport
          Positioned.fill(
"""

replace_double_tap = """
      body: Stack(
        children: [
          // 1. WebViewController WebGL Viewport
          Positioned.fill(
            child: GestureDetector(
              onDoubleTap: () {
                _setCameraView(_selectedView);
              },
"""
# We must close the child of positioned, but wait, Positioned.fill doesn't take child directly like that, wait it does.
# But I can't just regex replace this blindly without matching the exact structure.
# Let's see the structure around WebViewController.
