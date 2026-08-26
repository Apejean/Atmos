import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';

class TrajectorySettingsModal extends StatefulWidget {
  const TrajectorySettingsModal({super.key});

  @override
  State<TrajectorySettingsModal> createState() => _TrajectorySettingsModalState();
}

class _TrajectorySettingsModalState extends State<TrajectorySettingsModal> {
  double _speed = 1.0;
  double _elevation = 0.5;
  double _spread = 0.2;
  bool _oscEnabled = false;
  String _preset = '원형 (Circle)';

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      backgroundColor: AppColors.background,
      title: const Text('3D Trajectory Settings', style: TextStyle(color: Colors.white)),
      content: SizedBox(
        width: 400,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Text('Preset:', style: TextStyle(color: Colors.white)),
                const SizedBox(width: 16),
                DropdownButton<String>(
                  value: _preset,
                  dropdownColor: AppColors.cardSurfaceSolid,
                  style: const TextStyle(color: Colors.white),
                  items: ['원형 (Circle)', '8자 (Figure-8)', '나선 (Spiral)'].map((p) => DropdownMenuItem(value: p, child: Text(p))).toList(),
                  onChanged: (v) => setState(() => _preset = v!),
                ),
              ],
            ),
            const SizedBox(height: 16),
            const Text('Flight Speed', style: TextStyle(color: Colors.white70)),
            Slider(value: _speed, min: 0.1, max: 5.0, activeThumbColor: AppColors.primaryNeon, onChanged: (v) => setState(() => _speed = v)),
            const Text('Ceiling Elevation', style: TextStyle(color: Colors.white70)),
            Slider(value: _elevation, min: 0.0, max: 1.0, activeColor: Colors.cyanAccent, onChanged: (v) => setState(() => _elevation = v)),
            const Text('Sound Spread (Size)', style: TextStyle(color: Colors.white70)),
            Slider(value: _spread, min: 0.0, max: 1.0, activeColor: Colors.amberAccent, onChanged: (v) => setState(() => _spread = v)),
            const SizedBox(height: 16),
            SwitchListTile(
              title: const Text('OSC Automation', style: TextStyle(color: Colors.white)),
              value: _oscEnabled,
              activeThumbColor: AppColors.primaryNeon,
              onChanged: (v) => setState(() => _oscEnabled = v),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.of(context).pop(), child: const Text('Close', style: TextStyle(color: AppColors.primaryNeon))),
      ],
    );
  }
}
