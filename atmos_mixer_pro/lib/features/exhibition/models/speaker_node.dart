class SpeakerNode {
  final String id;
  final double x;
  final double y;
  final int channel;
  final double rotation;

  SpeakerNode({
    required this.id,
    required this.x,
    required this.y,
    required this.channel,
    this.rotation = 0.0,
  });

  SpeakerNode copyWith({
    String? id,
    double? x,
    double? y,
    int? channel,
    double? rotation,
  }) {
    return SpeakerNode(
      id: id ?? this.id,
      x: x ?? this.x,
      y: y ?? this.y,
      channel: channel ?? this.channel,
      rotation: rotation ?? this.rotation,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'x': x,
      'y': y,
      'channel': channel,
      'rotation': rotation,
    };
  }

  factory SpeakerNode.fromJson(Map<String, dynamic> map) {
    return SpeakerNode(
      id: map['id'],
      x: map['x'].toDouble(),
      y: map['y'].toDouble(),
      channel: map['channel'],
      rotation: (map['rotation'] ?? 0.0).toDouble(),
    );
  }
}
