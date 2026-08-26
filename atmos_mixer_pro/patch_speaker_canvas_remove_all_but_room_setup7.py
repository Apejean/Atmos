import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# The error is that I replaced `); // GestureDetector` with `); // GestureDetector` but the Scaffold inside it isn't properly closed!
# Let's fix the structure at the end of the `build` method.
# It should be:
#         ), // Stack
#       ), // Scaffold
#     ); // GestureDetector
#   } // build method
# } // _SpeakerCanvasScreenState

end_target = """          ],
        ),
        ); // GestureDetector
  }
}"""

end_replacement = """          ],
        ),
      ),
    );
  }
}"""

content = content.replace(end_target, end_replacement)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

