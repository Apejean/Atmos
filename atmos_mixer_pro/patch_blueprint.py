import re

with open('lib/features/exhibition/state/blueprint_state.dart', 'r') as f:
    content = f.read()

content = content.replace(
"""  final double canvasWidthMeters;
  final double canvasHeightMeters;""",
"""  final double canvasWidthMeters;
  final double canvasHeightMeters;
  final double roomHeightMeters;
  final double listeningHeightMeters;"""
)

content = content.replace(
"""    this.canvasWidthMeters = 40.0,
    this.canvasHeightMeters = 40.0,
  });""",
"""    this.canvasWidthMeters = 40.0,
    this.canvasHeightMeters = 40.0,
    this.roomHeightMeters = 5.0,
    this.listeningHeightMeters = 1.2,
  });"""
)

content = content.replace(
"""    double? canvasWidthMeters,
    double? canvasHeightMeters,""",
"""    double? canvasWidthMeters,
    double? canvasHeightMeters,
    double? roomHeightMeters,
    double? listeningHeightMeters,"""
)

content = content.replace(
"""      canvasWidthMeters: canvasWidthMeters ?? this.canvasWidthMeters,
      canvasHeightMeters: canvasHeightMeters ?? this.canvasHeightMeters,""",
"""      canvasWidthMeters: canvasWidthMeters ?? this.canvasWidthMeters,
      canvasHeightMeters: canvasHeightMeters ?? this.canvasHeightMeters,
      roomHeightMeters: roomHeightMeters ?? this.roomHeightMeters,
      listeningHeightMeters: listeningHeightMeters ?? this.listeningHeightMeters,"""
)

content = content.replace(
"""    final widthM = prefs.getDouble('blueprint_width_m') ?? 40.0;
    final heightM = prefs.getDouble('blueprint_height_m') ?? 40.0;""",
"""    final widthM = prefs.getDouble('blueprint_width_m') ?? 40.0;
    final heightM = prefs.getDouble('blueprint_height_m') ?? 40.0;
    final roomHM = prefs.getDouble('blueprint_room_h_m') ?? 5.0;
    final listenHM = prefs.getDouble('blueprint_listen_h_m') ?? 1.2;"""
)

content = content.replace(
"""      canvasWidthMeters: widthM,
      canvasHeightMeters: heightM,""",
"""      canvasWidthMeters: widthM,
      canvasHeightMeters: heightM,
      roomHeightMeters: roomHM,
      listeningHeightMeters: listenHM,"""
)

content = content.replace(
"""  Future<void> setCanvasDimensions(double widthM, double heightM) async {
    state = state.copyWith(canvasWidthMeters: widthM, canvasHeightMeters: heightM);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setDouble('blueprint_width_m', widthM);
    await prefs.setDouble('blueprint_height_m', heightM);
  }""",
"""  Future<void> setCanvasDimensions(double widthM, double heightM, {double? roomHM, double? listenHM}) async {
    state = state.copyWith(
      canvasWidthMeters: widthM, 
      canvasHeightMeters: heightM,
      roomHeightMeters: roomHM ?? state.roomHeightMeters,
      listeningHeightMeters: listenHM ?? state.listeningHeightMeters,
    );
    final prefs = await SharedPreferences.getInstance();
    await prefs.setDouble('blueprint_width_m', widthM);
    await prefs.setDouble('blueprint_height_m', heightM);
    if (roomHM != null) await prefs.setDouble('blueprint_room_h_m', roomHM);
    if (listenHM != null) await prefs.setDouble('blueprint_listen_h_m', listenHM);
  }"""
)

with open('lib/features/exhibition/state/blueprint_state.dart', 'w') as f:
    f.write(content)
