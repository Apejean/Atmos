import re

def main():
    path = "lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart"
    with open(path, "r") as f:
        content = f.read()

    # Wrap Slider with GestureDetector to handle double tap on thumb/slider to reset to middle
    old_slider = """              child: Slider(
                value: value.clamp(min, max),
                min: min,
                max: max,
                onChanged: onChanged,
              ),"""

    new_slider = """              child: GestureDetector(
                onDoubleTap: () {
                  final middle = (min + max) / 2;
                  onChanged(middle);
                },
                child: Slider(
                  value: value.clamp(min, max),
                  min: min,
                  max: max,
                  onChanged: onChanged,
                ),
              ),"""

    content = content.replace(old_slider, new_slider)

    with open(path, "w") as f:
        f.write(content)

main()
