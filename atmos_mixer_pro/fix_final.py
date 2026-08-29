with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# 1. Fix HTML syntax error that broke the 3D view
bad_html = """              // scale 속성 주입 (GLB 내부 렌더러에 의해 방 비율이 조정됨)
              innerModelViewerHtml: '<model-viewer scale="${roomWidth / 6.0} ${roomHeight / 3.0} ${roomDepth / 4.5}"',"""
content = content.replace(bad_html, '')

# 2. Fix the missing Room Setup button and badge
old_badge = """          // 2. Top-Left Room & Viewport Info Badge
          Positioned(
            top: 68,
            left: 16,
            child: Container(
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
                  const SizedBox(width: 10),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(
                      color: Colors.black.withValues(alpha: 0.3),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      'Zoom: Auto',
                      style: const TextStyle(
                        color: Colors.lightBlueAccent,
                        fontSize: 10,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),"""

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
          ),"""

content = content.replace(old_badge, new_badge)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
