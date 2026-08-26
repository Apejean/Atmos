import re

with open('lib/features/exhibition/models/room_zone.dart', 'r') as f:
    content = f.read()

# Add ceilingHeight and earLevel if not present
if 'ceilingHeight' not in content:
    content = content.replace(
        'final double physicalHeight;',
        'final double physicalHeight;\n  final double ceilingHeight;\n  final double earLevel;'
    )
    content = content.replace(
        'this.physicalHeight = 5.0,',
        'this.physicalHeight = 5.0,\n    this.ceilingHeight = 3.0,\n    this.earLevel = 1.2,'
    )
    content = content.replace(
        'final heightM = 3.0; // Standard room height assumption (3m)',
        'final heightM = ceilingHeight;'
    )
    content = content.replace(
        'double? physicalHeight,',
        'double? physicalHeight,\n    double? ceilingHeight,\n    double? earLevel,'
    )
    content = content.replace(
        'physicalHeight: physicalHeight ?? this.physicalHeight,',
        'physicalHeight: physicalHeight ?? this.physicalHeight,\n      ceilingHeight: ceilingHeight ?? this.ceilingHeight,\n      earLevel: earLevel ?? this.earLevel,'
    )
    content = content.replace(
        "'physicalHeight': physicalHeight,",
        "'physicalHeight': physicalHeight,\n      'ceiling_height': ceilingHeight,\n      'ear_level': earLevel,"
    )
    content = content.replace(
        "physicalHeight: (json['physical_height']",
        "ceilingHeight: (json['ceiling_height'] as num?)?.toDouble() ?? 3.0,\n      earLevel: (json['ear_level'] as num?)?.toDouble() ?? 1.2,\n      physicalHeight: (json['physical_height']"
    )

with open('lib/features/exhibition/models/room_zone.dart', 'w') as f:
    f.write(content)
