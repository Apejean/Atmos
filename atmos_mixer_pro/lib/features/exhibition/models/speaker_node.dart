class SpeakerNode {
  final String id;
  final double x;
  final double y;
  final int channel;

  SpeakerNode({
    required this.id,
    required this.x,
    required this.y,
    required this.channel,
  });

  SpeakerNode copyWith({String? id, double? x, double? y, int? channel}) {
    return SpeakerNode(
      id: id ?? this.id,
      x: x ?? this.x,
      y: y ?? this.y,
      channel: channel ?? this.channel,
    );
  }

  Map<String, dynamic> toJson() {
    return {'id': id, 'x': x, 'y': y, 'channel': channel};
  }

  factory SpeakerNode.fromJson(Map<String, dynamic> map) {
    return SpeakerNode(
      id: map['id'],
      x: map['x'].toDouble(),
      y: map['y'].toDouble(),
      channel: map['channel'],
    );
  }
}
