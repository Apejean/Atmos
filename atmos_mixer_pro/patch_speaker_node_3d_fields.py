import re

with open('lib/features/exhibition/models/speaker_node.dart', 'r') as f:
    content = f.read()

# Add heightZ, pitchTilt, panDeg, and power to SpeakerNode
if 'heightZ' not in content:
    content = content.replace(
        'final String roomId;',
        'final String roomId;\n  final double heightZ;\n  final double pitchTilt;\n  final double panDeg;\n  final double powerW;'
    )
    content = content.replace(
        'this.roomId = \'\',',
        'this.roomId = \'\',\n    this.heightZ = 2.0,\n    this.pitchTilt = 0.0,\n    this.panDeg = 0.0,\n    this.powerW = 300.0,'
    )
    content = content.replace(
        'String? roomId,',
        'String? roomId,\n    double? heightZ,\n    double? pitchTilt,\n    double? panDeg,\n    double? powerW,'
    )
    content = content.replace(
        'roomId: roomId ?? this.roomId,',
        'roomId: roomId ?? this.roomId,\n      heightZ: heightZ ?? this.heightZ,\n      pitchTilt: pitchTilt ?? this.pitchTilt,\n      panDeg: panDeg ?? this.panDeg,\n      powerW: powerW ?? this.powerW,'
    )
    content = content.replace(
        "'room_id': roomId,",
        "'room_id': roomId,\n      'height_z': heightZ,\n      'pitch_tilt': pitchTilt,\n      'pan_deg': panDeg,\n      'power_w': powerW,"
    )
    content = content.replace(
        "roomId: json['room_id'] as String? ?? '',",
        "roomId: json['room_id'] as String? ?? '',\n      heightZ: (json['height_z'] as num?)?.toDouble() ?? 2.0,\n      pitchTilt: (json['pitch_tilt'] as num?)?.toDouble() ?? 0.0,\n      panDeg: (json['pan_deg'] as num?)?.toDouble() ?? 0.0,\n      powerW: (json['power_w'] as num?)?.toDouble() ?? 300.0,"
    )

with open('lib/features/exhibition/models/speaker_node.dart', 'w') as f:
    f.write(content)

