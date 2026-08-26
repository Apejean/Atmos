import re

with open('lib/features/exhibition/models/room_zone.dart', 'r') as f:
    content = f.read()

# Add ceilingHeight and earLevel
content = re.sub(
    r'(final double wallTransmissionLoss;.*)',
    r'\1\n  final double ceilingHeight;\n  final double earLevel;',
    content
)

# Add to constructor
content = re.sub(
    r'(this\.wallTransmissionLoss = 35\.0,)',
    r'\1\n    this.ceilingHeight = 3.0,\n    this.earLevel = 1.2,',
    content
)

# Update copyWith
content = re.sub(
    r'(double\? wallTransmissionLoss,)',
    r'\1\n    double? ceilingHeight,\n    double? earLevel,',
    content
)

content = re.sub(
    r'(wallTransmissionLoss: wallTransmissionLoss \?\? this\.wallTransmissionLoss,)',
    r'\1\n      ceilingHeight: ceilingHeight ?? this.ceilingHeight,\n      earLevel: earLevel ?? this.earLevel,',
    content
)

# Update toJson
content = re.sub(
    r"('wall_transmission_loss': wallTransmissionLoss,)",
    r"\1\n      'ceiling_height': ceilingHeight,\n      'ear_level': earLevel,",
    content
)

# Update fromJson
content = re.sub(
    r"(wallTransmissionLoss: \(map\['wall_transmission_loss'\] \?\? 35\.0\)\.toDouble\(\),)",
    r"\1\n      ceilingHeight: (map['ceiling_height'] ?? 3.0).toDouble(),\n      earLevel: (map['ear_level'] ?? 1.2).toDouble(),",
    content
)

# Update volume calculation
content = content.replace(
    'final heightM = 3.0; // Standard room height assumption (3m)',
    'final heightM = ceilingHeight;'
)

with open('lib/features/exhibition/models/room_zone.dart', 'w') as f:
    f.write(content)
