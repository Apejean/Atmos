with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

# Make sure if selectedView is Auto, _cameraOrbit updates dynamically with orbitDist
# Find where orbitDist is calculated
calc_logic = """    final maxDim = roomWidth > roomDepth ? roomWidth : roomDepth;
    final orbitDist = (maxDim * 1.5).toStringAsFixed(1);"""
    
new_calc_logic = """    final maxDim = roomWidth > roomDepth ? roomWidth : roomDepth;
    final orbitDist = (maxDim * 1.5).toStringAsFixed(1);
    
    // Auto update orbit if room size changes
    if (_selectedView == 'Auto' || !_cameraOrbit.contains('${orbitDist}m')) {
      final r = orbitDist;
      switch(_selectedView) {
        case 'Auto': _cameraOrbit = '45deg 65deg ${r}m'; break;
        case 'Front': _cameraOrbit = '0deg 80deg ${r}m'; break;
        case 'Back': _cameraOrbit = '180deg 80deg ${r}m'; break;
        case 'Side(L)': _cameraOrbit = '-90deg 80deg ${r}m'; break;
        case 'Side(R)': _cameraOrbit = '90deg 80deg ${r}m'; break;
        case 'Top': _cameraOrbit = '0deg 0deg ${r}m'; break;
      }
    }"""
    
content = content.replace(calc_logic, new_calc_logic)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
