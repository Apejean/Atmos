import re

with open('lib/features/exhibition/state/blueprint_state.dart', 'r') as f:
    content = f.read()

old_data = """class BlueprintData {
  final String? imagePath;
  final double opacity;
  final double scale; // Pixels per meter

  const BlueprintData({this.imagePath, this.opacity = 0.5, this.scale = 50.0});

  BlueprintData copyWith({String? imagePath, double? opacity, double? scale}) {
    return BlueprintData(
      imagePath: imagePath ?? this.imagePath,
      opacity: opacity ?? this.opacity,
      scale: scale ?? this.scale,
    );
  }
}"""

new_data = """class BlueprintData {
  final String? imagePath;
  final double opacity;
  final double scale; // Pixels per meter
  final double canvasWidthMeters;
  final double canvasHeightMeters;

  const BlueprintData({
    this.imagePath,
    this.opacity = 0.5,
    this.scale = 50.0,
    this.canvasWidthMeters = 40.0,
    this.canvasHeightMeters = 40.0,
  });

  BlueprintData copyWith({
    String? imagePath,
    double? opacity,
    double? scale,
    double? canvasWidthMeters,
    double? canvasHeightMeters,
  }) {
    return BlueprintData(
      imagePath: imagePath ?? this.imagePath,
      opacity: opacity ?? this.opacity,
      scale: scale ?? this.scale,
      canvasWidthMeters: canvasWidthMeters ?? this.canvasWidthMeters,
      canvasHeightMeters: canvasHeightMeters ?? this.canvasHeightMeters,
    );
  }
}"""

old_state = """  Future<void> _loadFromPrefs() async {
    final prefs = await SharedPreferences.getInstance();
    final path = prefs.getString(_kImagePathKey);
    final opacity = prefs.getDouble(_kOpacityKey) ?? 0.5;
    final scale = prefs.getDouble(_kScaleKey) ?? 50.0;
    state = BlueprintData(imagePath: path, opacity: opacity, scale: scale);
  }"""

new_state = """  Future<void> _loadFromPrefs() async {
    final prefs = await SharedPreferences.getInstance();
    final path = prefs.getString(_kImagePathKey);
    final opacity = prefs.getDouble(_kOpacityKey) ?? 0.5;
    final scale = prefs.getDouble(_kScaleKey) ?? 50.0;
    final widthM = prefs.getDouble('blueprint_width_m') ?? 40.0;
    final heightM = prefs.getDouble('blueprint_height_m') ?? 40.0;
    state = BlueprintData(
      imagePath: path,
      opacity: opacity,
      scale: scale,
      canvasWidthMeters: widthM,
      canvasHeightMeters: heightM,
    );
  }

  Future<void> setCanvasDimensions(double widthM, double heightM) async {
    state = state.copyWith(canvasWidthMeters: widthM, canvasHeightMeters: heightM);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setDouble('blueprint_width_m', widthM);
    await prefs.setDouble('blueprint_height_m', heightM);
  }"""

content = content.replace(old_data, new_data)
content = content.replace(old_state, new_state)

with open('lib/features/exhibition/state/blueprint_state.dart', 'w') as f:
    f.write(content)
