import re

with open("lib/features/exhibition/screens/speaker_canvas_screen.dart", "r") as f:
    content = f.read()

# Add the buttons inside the Row children in the AppBar
buttons_to_add = """
              const Spacer(),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Text('SPL HEATMAP', style: TextStyle(fontSize: 12, fontWeight: FontWeight.bold, color: Colors.white70)),
                  const SizedBox(width: 8),
                  Switch(
                    value: false,
                    onChanged: (val) {},
                    activeColor: AppColors.primaryNeon,
                  ),
                  const SizedBox(width: 16),
                  ElevatedButton.icon(
                    onPressed: () {},
                    icon: const Icon(Icons.picture_as_pdf, size: 16),
                    label: const Text('EXPORT PDF REPORT'),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: AppColors.surfaceLighter,
                      foregroundColor: AppColors.primaryNeon,
                    ),
                  ),
                  const SizedBox(width: 16),
                ],
              ),"""

content = content.replace("const Spacer(),\n              Consumer(", buttons_to_add + "\n              const Spacer(),\n              Consumer(")

with open("lib/features/exhibition/screens/speaker_canvas_screen.dart", "w") as f:
    f.write(content)
