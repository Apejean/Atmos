import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';

/// Professional Multitrack Timeline View Widget
class MultitrackTimelineWidget extends StatefulWidget {
  final List<String> trackNames;
  final double playbackPositionSeconds;
  final double totalDurationSeconds;

  const MultitrackTimelineWidget({
    super.key,
    this.trackNames = const ['Track 1 (BGM)', 'Track 2 (Ambience)', 'Track 3 (Effects)', 'Track 4 (Voice)'],
    this.playbackPositionSeconds = 12.5,
    this.totalDurationSeconds = 120.0,
  });

  @override
  State<MultitrackTimelineWidget> createState() =>
      _MultitrackTimelineWidgetState();
}

class _MultitrackTimelineWidgetState extends State<MultitrackTimelineWidget> {
  double _zoomLevel = 1.0;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: AppColors.cardSurface,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Colors.white12),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header Controls
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Row(
                children: [
                  const Icon(Icons.view_timeline, color: AppColors.primaryNeon, size: 18),
                  const SizedBox(width: 8),
                  const Text(
                    'Multitrack Timeline',
                    style: TextStyle(
                      color: Colors.white,
                      fontSize: 13,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                ],
              ),
              Row(
                children: [
                  IconButton(
                    icon: const Icon(Icons.zoom_out, size: 16, color: Colors.white70),
                    onPressed: () {
                      setState(() => _zoomLevel = (_zoomLevel - 0.2).clamp(0.5, 3.0));
                    },
                  ),
                  Text(
                    '${(_zoomLevel * 100).toInt()}%',
                    style: const TextStyle(color: Colors.white54, fontSize: 11),
                  ),
                  IconButton(
                    icon: const Icon(Icons.zoom_in, size: 16, color: Colors.white70),
                    onPressed: () {
                      setState(() => _zoomLevel = (_zoomLevel + 0.2).clamp(0.5, 3.0));
                    },
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 8),
          // Timeline Content
          Expanded(
            child: SingleChildScrollView(
              child: Column(
                children: widget.trackNames.asMap().entries.map((entry) {
                  final idx = entry.key;
                  final name = entry.value;
                  return _buildTrackTimelineRow(idx, name);
                }).toList(),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildTrackTimelineRow(int index, String name) {
    final colors = [
      Colors.cyanAccent,
      Colors.amberAccent,
      Colors.greenAccent,
      Colors.pinkAccent,
    ];
    final color = colors[index % colors.length];

    return Container(
      margin: const EdgeInsets.only(bottom: 6),
      height: 48,
      decoration: BoxDecoration(
        color: const Color(0xFF161622),
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: Colors.white10),
      ),
      child: Row(
        children: [
          // Track Name Column
          Container(
            width: 140,
            padding: const EdgeInsets.symmetric(horizontal: 10),
            decoration: const BoxDecoration(
              border: Border(right: BorderSide(color: Colors.white10)),
            ),
            child: Row(
              children: [
                Container(
                  width: 4,
                  height: 24,
                  decoration: BoxDecoration(
                    color: color,
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    name,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(color: Colors.white70, fontSize: 12),
                  ),
                ),
              ],
            ),
          ),
          // Clip Waveform Track Line
          Expanded(
            child: Container(
              padding: const EdgeInsets.all(4),
              child: Stack(
                children: [
                  // Clip Block
                  FractionallySizedBox(
                    widthFactor: 0.75 * _zoomLevel.clamp(0.2, 1.0),
                    child: Container(
                      decoration: BoxDecoration(
                        color: color.withValues(alpha: 0.2),
                        borderRadius: BorderRadius.circular(4),
                        border: Border.all(color: color.withValues(alpha: 0.6)),
                      ),
                      padding: const EdgeInsets.symmetric(horizontal: 8),
                      child: Row(
                        children: [
                          Icon(Icons.waves, size: 16, color: color),
                          const SizedBox(width: 6),
                          Text(
                            'Clip $index (Equal-Power Fade)',
                            style: TextStyle(color: color, fontSize: 10, fontWeight: FontWeight.bold),
                          ),
                        ],
                      ),
                    ),
                  ),
                  // Playhead line
                  Positioned(
                    left: 100 * _zoomLevel,
                    top: 0,
                    bottom: 0,
                    child: Container(
                      width: 2,
                      color: Colors.redAccent,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
