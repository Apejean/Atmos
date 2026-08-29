import re

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# Add _cameraOrbit state to class
content = re.sub(r'class _Dynamic3DRoomState extends ConsumerState<Dynamic3DRoom> {', 
                 r"class _Dynamic3DRoomState extends ConsumerState<Dynamic3DRoom> {\n  String? _cameraOrbit;\n  String _selectedView = 'Auto';", 
                 content)

old_zoom = """                      Container(
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
                      ),"""

new_zoom = """                      Container(
                        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 0),
                        height: 24,
                        decoration: BoxDecoration(
                          color: Colors.black.withValues(alpha: 0.3),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: DropdownButtonHideUnderline(
                          child: DropdownButton<String>(
                            value: _selectedView,
                            icon: const Icon(Icons.arrow_drop_down, color: Colors.lightBlueAccent, size: 16),
                            dropdownColor: const Color(0xFF1B232E),
                            style: const TextStyle(color: Colors.lightBlueAccent, fontSize: 11, fontWeight: FontWeight.bold),
                            items: const [
                              DropdownMenuItem(value: 'Auto', child: Text('Zoom: Auto')),
                              DropdownMenuItem(value: 'Front', child: Text('Front View')),
                              DropdownMenuItem(value: 'Back', child: Text('Back View')),
                              DropdownMenuItem(value: 'Side(L)', child: Text('Left View')),
                              DropdownMenuItem(value: 'Side(R)', child: Text('Right View')),
                              DropdownMenuItem(value: 'Top', child: Text('Top View')),
                            ],
                            onChanged: (val) {
                              if (val == null) return;
                              final w = widget.activeRoom?.physicalWidth ?? 6.0;
                              final d = widget.activeRoom?.physicalHeight ?? 4.5;
                              final maxDim = w > d ? w : d;
                              final orbitDist = (maxDim * 1.5).toStringAsFixed(1);
                              setState(() {
                                _selectedView = val;
                                final r = orbitDist;
                                switch(val) {
                                  case 'Auto': _cameraOrbit = null; break;
                                  case 'Front': _cameraOrbit = '0deg 85deg ${r}m'; break;
                                  case 'Back': _cameraOrbit = '180deg 85deg ${r}m'; break;
                                  case 'Side(L)': _cameraOrbit = '90deg 85deg ${r}m'; break;
                                  case 'Side(R)': _cameraOrbit = '-90deg 85deg ${r}m'; break;
                                  case 'Top': _cameraOrbit = '0deg 0deg ${r}m'; break;
                                }
                              });
                            },
                          ),
                        ),
                      ),"""
                      
content = content.replace(old_zoom, new_zoom)

content = re.sub(r"cameraOrbit:\s*'(.*?)',", "cameraOrbit: _cameraOrbit ?? '\\g<1>',", content)

old_positioned = """          Positioned.fill(
            child: _localGlbPath == null ? const Center(child: CircularProgressIndicator()) : ModelViewer("""

new_positioned = """          Positioned.fill(
            child: _localGlbPath == null ? const Center(child: CircularProgressIndicator()) : GestureDetector(
              onDoubleTap: () {
                setState(() {
                  _selectedView = 'Auto';
                  _cameraOrbit = null; // Revert to dynamic auto calculation
                });
              },
              child: ModelViewer("""

content = content.replace(old_positioned, new_positioned)

old_mv_end = """              interactionPrompt: InteractionPrompt.none,

              

              
            ),
          ),"""

new_mv_end = """              interactionPrompt: InteractionPrompt.none,
            ),
            ),
          ),"""
          
content = content.replace(old_mv_end, new_mv_end)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
