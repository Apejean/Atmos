import re

with open("lib/features/exhibition/models/room_zone.dart", "r") as f:
    content = f.read()

# Add ceilingHeight and earLevel
content = content.replace("final double physicalHeight;", "final double physicalHeight;\n  final double ceilingHeight;\n  final double earLevel;")
content = content.replace("this.physicalHeight = 5.0,", "this.physicalHeight = 5.0,\n    this.ceilingHeight = 3.0,\n    this.earLevel = 1.6,")
content = content.replace("double? physicalHeight,", "double? physicalHeight,\n    double? ceilingHeight,\n    double? earLevel,")
content = content.replace("physicalHeight: physicalHeight ?? this.physicalHeight,", "physicalHeight: physicalHeight ?? this.physicalHeight,\n      ceilingHeight: ceilingHeight ?? this.ceilingHeight,\n      earLevel: earLevel ?? this.earLevel,")
content = content.replace("'physicalHeight': physicalHeight,", "'physicalHeight': physicalHeight,\n      'ceilingHeight': ceilingHeight,\n      'earLevel': earLevel,")
content = content.replace("physicalHeight: (json['physical_height'] as num?)?.toDouble() ?? 5.0,", "physicalHeight: (json['physical_height'] as num?)?.toDouble() ?? 5.0,\n      ceilingHeight: (json['ceiling_height'] as num?)?.toDouble() ?? 3.0,\n      earLevel: (json['ear_level'] as num?)?.toDouble() ?? 1.6,")

# Update estimatedRt60 heightM
content = content.replace("final heightM = 3.0; // Standard room height assumption (3m)", "final heightM = ceilingHeight;")

with open("lib/features/exhibition/models/room_zone.dart", "w") as f:
    f.write(content)
