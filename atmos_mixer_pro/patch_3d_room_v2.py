with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# 1. 룸 셋업 뱃지 추가 (좌측 상단, "룸 셋업 버튼 옆" 이라는 요구사항)
old_badge = """          // 2. Top-Left Room & Viewport Info Badge

          Positioned(
            top: 16,
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
                    '$roomLabel: ${roomWidth.toStringAsFixed(1)}m × ${roomDepth.toStringAsFixed(1)}m × ${roomHeight.toStringAsFixed(1)}m | Listener at (${(roomWidth / 2).toStringAsFixed(1)}, ${(roomDepth / 2).toStringAsFixed(1)}, 1.2m)',
                    style: const TextStyle(
                      color: Colors.white70,
                      fontSize: 12,
                      fontWeight: FontWeight.w500,
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
                // 룸 셋업 런처 버튼
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
                    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
                    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                    elevation: 6,
                  ),
                  onPressed: () {
                    // We need a callback to open it, or we can use a provider.
                    // For now, this requires hooking up to the parent.
                  },
                ),
                const SizedBox(width: 16), // 일정 간격 띄워서 배치
                
                // 룸 인포 뱃지 (첨부 이미지 디자인)
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                  decoration: BoxDecoration(
                    color: const Color(0xFF161E28).withValues(alpha: 0.85),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      const Icon(Icons.view_in_ar_rounded, size: 16, color: Colors.lightBlueAccent),
                      const SizedBox(width: 8),
                      Text(
                        '새로운 룸: ${roomWidth.toStringAsFixed(1)}m × ${roomDepth.toStringAsFixed(1)}m × ${roomHeight.toStringAsFixed(1)}m | 4x4 Grid',
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
                        child: Text(
                          'Zoom: ${_cameraDistance.toStringAsFixed(1)}m',
                          style: const TextStyle(
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

# 2. 줌인 줌아웃 UI 지우기 (우측 패널) - 부드러운 네이티브 ModelViewer zoom 사용 유도
zoom_ui = """          // Right-Side Zoom & Camera Navigation Control Pod
          Positioned(
            top: 68,
            right: widget.selectedSpeakerId != null ? 360 : 16,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 6),
              decoration: BoxDecoration(
                color: const Color(0xFF161E28).withValues(alpha: 0.90),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.white.withValues(alpha: 0.12)),
                boxShadow: [
                  BoxShadow(
                    color: Colors.black.withValues(alpha: 0.4),
                    blurRadius: 10,
                    offset: const Offset(0, 4),
                  ),
                ],
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  // Zoom In Button (+)
                  _buildNavIconButton(
                    icon: Icons.add_rounded,
                    tooltip: 'Zoom In',
                    onTap: _zoomIn,
                  ),
                  const SizedBox(height: 4),

                  // Current Distance
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                    child: Text(
                      '${_cameraDistance.toStringAsFixed(1)}m',
                      style: const TextStyle(
                        color: Colors.white60,
                        fontSize: 10,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ),
                  const SizedBox(height: 4),

                  // Zoom Out Button (-)
                  _buildNavIconButton(
                    icon: Icons.remove_rounded,
                    tooltip: 'Zoom Out',
                    onTap: _zoomOut,
                  ),

                  const Divider(color: Colors.white12, height: 12, indent: 4, endIndent: 4),

                  // Reset Camera View (⟲)
                  _buildNavIconButton(
                    icon: Icons.restart_alt_rounded,
                    tooltip: 'Reset View',
                    onTap: _resetCamera,
                  ),
                  const SizedBox(height: 4),

                  // Top-Down / 3D Toggle
                  _buildNavIconButton(
                    icon: _isTopView ? Icons.view_in_ar_rounded : Icons.grid_view_rounded,
                    tooltip: _isTopView ? 'Switch to 3D Orbit' : 'Switch to Top View',
                    color: _isTopView ? Colors.lightBlueAccent : Colors.white70,
                    onTap: _toggleTopView,
                  ),
                ],
              ),
            ),
          ),"""

content = content.replace(zoom_ui, '')

# 3. ModelViewer 스케일(Scale) 조절하여 방 크기 변경 반영
# scale은 string "x y z" 형태. 원본이 6.0 x 4.5 x 3.0 m 라고 가정.
scale_x = "roomWidth / 6.0"
scale_y = "roomDepth / 4.5"
scale_z = "roomHeight / 3.0"

model_viewer_old = """              src: 'assets/models/room_with_listener.glb',
              alt: '3D Room Space with Listener Mannequin',
              autoRotate: false,
              cameraControls: true,"""

model_viewer_new = f"""              src: 'assets/models/room_with_listener.glb',
              alt: '3D Room Space with Listener Mannequin',
              autoRotate: false,
              cameraControls: true,
              // scale 속성 주입 (GLB 내부 렌더러에 의해 비율이 조정됨)
              innerModelViewerHtml: '<model-viewer scale="${{{scale_x}}} ${{{scale_z}}} ${{{scale_y}}}"',"""

content = content.replace(model_viewer_old, model_viewer_new)

# Add onOpenRoomSetup callback to Dynamic3DRoom
sig_old = """  final RoomZone? activeRoom;
  final bool showHeatmap;

  const Dynamic3DRoom({
    super.key,
    this.onSpeakerTapped,
    this.selectedSpeakerId,
    this.activeRoom,
    this.showHeatmap = false,
  });"""
sig_new = """  final RoomZone? activeRoom;
  final bool showHeatmap;
  final VoidCallback? onOpenRoomSetup;

  const Dynamic3DRoom({
    super.key,
    this.onSpeakerTapped,
    this.selectedSpeakerId,
    this.activeRoom,
    this.showHeatmap = false,
    this.onOpenRoomSetup,
  });"""
content = content.replace(sig_old, sig_new)
content = content.replace('// We need a callback to open it, or we can use a provider.\n                    // For now, this requires hooking up to the parent.', 'if (widget.onOpenRoomSetup != null) widget.onOpenRoomSetup!();')

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
