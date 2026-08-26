import re

with open('lib/features/exhibition/models/trajectory.dart', 'r') as f:
    content = f.read()

# I need to add size to TrajectoryModel.
old_decl = """  String? stemGroupId;
  String? targetRoomZoneId;

  double progress = 0.0;"""
new_decl = """  String? stemGroupId;
  String? targetRoomZoneId;
  double size;

  double progress = 0.0;"""
content = content.replace(old_decl, new_decl)

old_init = """    this.audioTrackId,
    this.stemGroupId,
    this.targetRoomZoneId,
  });"""
new_init = """    this.audioTrackId,
    this.stemGroupId,
    this.targetRoomZoneId,
    this.size = 0.2,
  });"""
content = content.replace(old_init, new_init)

old_to_json = """      'stemGroupId': stemGroupId,
      'targetRoomZoneId': targetRoomZoneId,
    };
  }"""
new_to_json = """      'stemGroupId': stemGroupId,
      'targetRoomZoneId': targetRoomZoneId,
      'size': size,
    };
  }"""
content = content.replace(old_to_json, new_to_json)

old_from_json = """      targetRoomZoneId: json['targetRoomZoneId'] as String?,
    );
  }"""
new_from_json = """      targetRoomZoneId: json['targetRoomZoneId'] as String?,
      size: (json['size'] as num?)?.toDouble() ?? 0.2,
    );
  }"""
content = content.replace(old_from_json, new_from_json)

old_copy = """    String? targetRoomZoneId,
  }) {
    return TrajectoryModel("""
new_copy = """    String? targetRoomZoneId,
    double? size,
  }) {
    return TrajectoryModel("""
content = content.replace(old_copy, new_copy)

old_copy_return = """      targetRoomZoneId: targetRoomZoneId ?? this.targetRoomZoneId,
    );
  }"""
new_copy_return = """      targetRoomZoneId: targetRoomZoneId ?? this.targetRoomZoneId,
      size: size ?? this.size,
    );
  }"""
content = content.replace(old_copy_return, new_copy_return)


with open('lib/features/exhibition/models/trajectory.dart', 'w') as f:
    f.write(content)
