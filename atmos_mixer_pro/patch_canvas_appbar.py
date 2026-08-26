with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

appbar_addition = """              // Phase 1: SPL Heatmap Mock Toggle
              Container(
                height: 34,
                padding: const EdgeInsets.symmetric(horizontal: 6),
                decoration: BoxDecoration(
                  color: Colors.grey.shade900,
                  borderRadius: BorderRadius.circular(17),
                  border: Border.all(
                    color: _showHeatmap ? AppColors.primaryNeon : Colors.white24,
                    width: 1.5,
                  ),
                ),
                child: InkWell(
                  onTap: () {
                    setState(() {
                      _showHeatmap = !_showHeatmap;
                    });
                  },
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        Icons.thermostat,
                        size: 16,
                        color: _showHeatmap ? AppColors.primaryNeon : Colors.white70,
                      ),
                      const SizedBox(width: 6),
                      Text(
                        '🌡️ SPL Heatmap',
                        style: TextStyle(
                          fontSize: 12,
                          fontWeight: FontWeight.bold,
                          color: _showHeatmap ? AppColors.primaryNeon : Colors.white70,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(width: 8),
              // Phase 1: Export Rigging Report Mock Button
              ElevatedButton.icon(
                style: ElevatedButton.styleFrom(
                  backgroundColor: Colors.white10,
                  foregroundColor: Colors.white,
                  minimumSize: const Size(0, 34),
                  padding: const EdgeInsets.symmetric(horizontal: 12),
                ),
                icon: const Icon(Icons.description, size: 16),
                label: const Text('📄 Export Rigging Report', style: TextStyle(fontSize: 12)),
                onPressed: () {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('Export Rigging Report functionality coming soon.')),
                  );
                },
              ),
              const SizedBox(width: 12),
"""

content = content.replace(
"""              IconButton(
                tooltip: _isPlayingAutomation""",
appbar_addition + """              IconButton(
                tooltip: _isPlayingAutomation"""
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
