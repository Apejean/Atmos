import re

with open('lib/features/exhibition/models/speaker_node.dart', 'r') as f:
    content = f.read()

content = content.replace('final String id;', 'final String id;\n  final String? roomId;')

content = content.replace('required this.id,', 'required this.id,\n    this.roomId,')

content = content.replace('String? id,', 'String? id,\n    String? roomId,')

content = content.replace('id: id ?? this.id,', 'id: id ?? this.id,\n      roomId: roomId ?? this.roomId,')

content = content.replace("'id': id,", "'id': id,\n      'room_id': roomId,")

content = content.replace("id: map['id'],", "id: map['id'],\n      roomId: map['room_id'],")

with open('lib/features/exhibition/models/speaker_node.dart', 'w') as f:
    f.write(content)
