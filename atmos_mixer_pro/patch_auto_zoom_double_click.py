with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

old_positioned = """          Positioned.fill(
            child: _localGlbPath == null ? const Center(child: CircularProgressIndicator()) : ModelViewer("""

new_positioned = """          Positioned.fill(
            child: _localGlbPath == null ? const Center(child: CircularProgressIndicator()) : GestureDetector(
              onDoubleTap: () {
                // Double tap resets to Auto zoom and fits room perfectly
                setState(() {
                  _selectedView = 'Auto';
                  _cameraOrbit = '45deg 65deg ${orbitDist}m';
                });
              },
              child: ModelViewer("""

content = content.replace(old_positioned, new_positioned)

# Need to close the GestureDetector properly
old_mv_end = """              maxCameraOrbit: 'auto auto 2000m',
              minCameraOrbit: 'auto auto 1.5m',
              exposure: 1.1,
              shadowIntensity: 0.6,
              shadowSoftness: 0.8,
            ),
          ),"""

new_mv_end = """              maxCameraOrbit: 'auto auto 2000m',
              minCameraOrbit: 'auto auto 1.5m',
              exposure: 1.1,
              shadowIntensity: 0.6,
              shadowSoftness: 0.8,
            ),
            ),
          ),"""
          
content = content.replace(old_mv_end, new_mv_end)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
