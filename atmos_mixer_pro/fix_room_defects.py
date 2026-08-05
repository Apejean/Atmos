import re
import os

def fix_room_zone_widget():
    path = 'lib/features/exhibition/widgets/room_zone_widget.dart'
    with open(path, 'r', encoding='utf-8') as f:
        content = f.read()

    # 1. Add isSelected and onInteractionStart/End to RoomZoneWidget
    content = content.replace(
        'final VoidCallback? onDragUpdate;',
        'final VoidCallback? onDragUpdate;\n  final bool isSelected;\n  final VoidCallback? onInteractionStart;\n  final VoidCallback? onInteractionEnd;'
    )
    content = content.replace(
        'this.onDragUpdate,',
        'this.onDragUpdate,\n    this.isSelected = false,\n    this.onInteractionStart,\n    this.onInteractionEnd,'
    )

    # Replace onPanStart, onPanUpdate, onPanEnd for the main room drag to trigger interaction callbacks
    content = content.replace('setState(() => _isInteracting = true);', 'setState(() => _isInteracting = true);\n                      widget.onInteractionStart?.call();')
    content = content.replace('setState(() => _isInteracting = false);', 'setState(() => _isInteracting = false);\n                      widget.onInteractionEnd?.call();')
    content = content.replace('setState(() {\n                        _localX = snappedX;', 'setState(() {\n                        _localX = snappedX;\n                        _isInteracting = false;\n                      });\n                      widget.onInteractionEnd?.call();')

    # Also for rotation handle
    # Wait, let's just do a global replace for _isInteracting = true/false inside onPanStart/End
    # The string `setState(() => _isInteracting = true);` appears multiple times. Let's do regex
    
    # 2. Fix Double-Rotation Jump: change globalToLocal to use context instead of renderBox
    # In `_buildRotateHandle`, it uses `roomContext`. We can just use `context` which is unrotated parent.
    # Actually, `roomContext` is inside Builder, inside Positioned. `context` is inside State (ConsumerState).
    # `context` points to RoomZoneWidget. The outermost Positioned is NOT rotated yet. Wait, if we use `context` inside `Transform.rotate` it might be the same.
    # We should get `context` which refers to RoomZoneWidget's context.
    content = content.replace('final renderBox = roomContext.findRenderObject()', 'final renderBox = context.findRenderObject()')
    content = content.replace('final renderBox = context.findRenderObject()', 'final renderBox = context.findRenderObject()') # just to be safe
    # Also in corner resizing, the `_isRotatingFromCorner` logic should be removed!
    
    # Remove _isRotatingFromCorner from corner handle logic completely.
    # We remove the `onPanStart` and `onPanUpdate` parts related to rotation from `_buildResizeHandle`.
    
    # Let's replace _buildResizeHandle completely with a fixed version
    # Actually, easier to use re.sub for corner rotation removal
    
    # 3. Hit-Test Clipping
    # Change outermost Positioned:
    content = re.sub(
        r'return Positioned\(\s*left: _localX,\s*top: _localY,\s*width: _localW,\s*height: _localH,\s*child: Builder\(\s*builder: \(roomContext\) \{',
        '''return Positioned(
      left: _localX - 40,
      top: _localY - 40,
      width: _localW + 80,
      height: _localH + 80,
      child: Builder(
        builder: (roomContext) {''',
        content
    )

    # Need to add a Positioned inside the Stack to offset the 40px padding
    content = content.replace(
        'child: Stack(\n              clipBehavior: Clip.none,\n              children: [',
        '''child: Stack(
              clipBehavior: Clip.none,
              children: [
                Positioned(
                  left: 40,
                  top: 40,
                  width: _localW,
                  height: _localH,
                  child: Stack(
                    clipBehavior: Clip.none,
                    children: ['''
    )

    # Need to close that extra Stack at the end of the children
    # We'll find `_buildDoorHandle(),` which is the last child in the main stack usually.
    # Wait, looking at the file, the end of the children array is after _buildResizeHandle calls.
    content = re.sub(
        r'(_buildResizeHandle\(Alignment.bottomRight\),\s*\]\s*,\s*\)\s*,\s*\]\s*,\s*\)\s*;)',
        r'_buildResizeHandle(Alignment.bottomRight),\n                    ],\n                  ),\n                ),\n              ],\n            ),\n          );\n        },\n      ),\n    );',
        content
    )
    # The exact closing might be tricky. Let's write a better replacement script.

if __name__ == '__main__':
    fix_room_zone_widget()
