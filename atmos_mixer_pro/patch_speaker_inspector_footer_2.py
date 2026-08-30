import re

def main():
    path = "lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart"
    with open(path, "r") as f:
        content = f.read()

    # The user asked for a 2-column layout for FIX ON/OFF and Remove buttons, 
    # and they said it wasn't showing. Wait, I did patch it earlier but maybe the hot restart missed it, or it was overridden?
    # Let's check if the footer has FIX ON/OFF.
    
    old_footer_search = """          // Footer
          Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Expanded(
                  child: ElevatedButton(
                    onPressed: () {
                      _updateSpeaker(speaker!, isFixed: !speaker.isFixed);
                    },"""

    if old_footer_search in content:
        print("Found the 2-column footer I made before.")
    else:
        print("Did NOT find the 2-column footer.")

main()
