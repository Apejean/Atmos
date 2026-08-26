import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# We need to remove all buttons from the AppBar and floating action buttons EXCEPT Room Setup.
# Basically, the AppBar title row should only have "Exhibition Canvas" and nothing else.
# Wait, let's just make the AppBar title completely empty or minimal to fix the RenderFlex overflow and remove unwanted buttons.
# The user wants "Room Setup 기능 및 버튼 UI만 남기고 싹다 지워줘 새로 구현해야해"
# So I should remove everything from the AppBar EXCEPT the Room Setup button. But wait, Room Setup button is a floating button on the bottom left now!
# So we can remove everything from the AppBar.

target_appbar_start = """        appBar: AppBar(
          title: Row(
            children: [
              const Text('Exhibition Canvas'),
              const SizedBox(width: 8),
              Expanded(child: SingleChildScrollView(scrollDirection: Axis.horizontal, child: Row(mainAxisSize: MainAxisSize.max, mainAxisAlignment: MainAxisAlignment.end, children: ["""

target_appbar_end = """]))),
            ],
          ),
        ),
        backgroundColor: Colors.black,"""

# Let's find the `appBar:` block and replace it entirely.
import os
os.system("sed -n '1548,1870p' lib/features/exhibition/screens/speaker_canvas_screen.dart > temp_appbar.txt")
