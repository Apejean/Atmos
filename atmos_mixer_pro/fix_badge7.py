import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# Replace ValueKey to rebuild on scale change
bad_key = "key: ValueKey('room_3d_viewer_${widget.activeRoom?.id ?? \"def\"}'), // Only recreate when room ID changes"
good_key = "key: ValueKey('room_3d_viewer_${widget.activeRoom?.id ?? \"def\"}_${roomWidth}_${roomDepth}_${roomHeight}'), // Recreate when scale changes"
content = content.replace(bad_key, good_key)

# Replace the badge
parts = content.split('          // 2. Top-Left Room & Viewport Info Badge')

if len(parts) > 1:
    end_marker = "          // 4. Bottom-Right Floating Action Button: Add Speaker"
    sub_parts = parts[1].split(end_marker)
    
    if len(sub_parts) > 1:
        new_badge = """          // 2. Top-Left Room Setup & Info Badge
          Positioned(
            top: 16,
            left: 16,
            child: Row(
              children: [
                OutlinedButton.icon(
                  icon: const Icon(Icons.tune_rounded, size: 16, color: Colors.lightBlueAccent),
                  label: const Text(
                    'ROOM SETUP',
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.bold,
                      letterSpacing: 0.5,
                    ),
                  ),
                  style: OutlinedButton.styleFrom(
                    foregroundColor: Colors.lightBlueAccent,
                    backgroundColor: const Color(0xFF161E28).withValues(alpha: 0.95),
                    side: BorderSide(color: Colors.lightBlueAccent.withValues(alpha: 0.7), width: 1.2),
                    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
                    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                    elevation: 6,
                  ),
                  onPressed: () {
                    if (widget.onOpenRoomSetup != null) {
                      widget.onOpenRoomSetup!();
                    }
                  },
                ),
                const SizedBox(width: 16),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                  decoration: BoxDecoration(
                    color: const Color(0xFF161E28).withValues(alpha: 0.85),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
                    boxShadow: [
                      BoxShadow(
                        color: Colors.black.withValues(alpha: 0.3),
                        blurRadius: 10,
                      ),
                    ],
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      const Icon(Icons.view_in_ar_rounded, size: 16, color: Colors.lightBlueAccent),
                      const SizedBox(width: 8),
                      Text(
                        '$roomLabel: ${roomWidth.toStringAsFixed(1)}m × ${roomDepth.toStringAsFixed(1)}m × ${roomHeight.toStringAsFixed(1)}m | 4×4 Grid',
                        style: const TextStyle(
                          color: Colors.white70,
                          fontSize: 12,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                      const SizedBox(width: 12),
                      Container(
                        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                        decoration: BoxDecoration(
                          color: Colors.black.withValues(alpha: 0.3),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: const Text(
                          'Zoom: Auto',
                          style: TextStyle(
                            color: Colors.lightBlueAccent,
                            fontSize: 11,
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),

"""
        content = parts[0] + new_badge + end_marker + sub_parts[1]
        
        with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
            f.write(content)
        print("Success badge replacement")
else:
    print("Could not find badge marker")

