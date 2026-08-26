import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

class BlueprintData {
  final String? imagePath;
  final double opacity;
  final double scale; // Pixels per meter
  final double canvasWidthMeters;
  final double canvasHeightMeters;
  final double roomHeightMeters;
  final double listeningHeightMeters;

  const BlueprintData({
    this.imagePath,
    this.opacity = 0.5,
    this.scale = 50.0,
    this.canvasWidthMeters = 40.0,
    this.canvasHeightMeters = 40.0,
    this.roomHeightMeters = 5.0,
    this.listeningHeightMeters = 1.2,
  });

  BlueprintData copyWith({
    String? imagePath,
    double? opacity,
    double? scale,
    double? canvasWidthMeters,
    double? canvasHeightMeters,
    double? roomHeightMeters,
    double? listeningHeightMeters,
  }) {
    return BlueprintData(
      imagePath: imagePath ?? this.imagePath,
      opacity: opacity ?? this.opacity,
      scale: scale ?? this.scale,
      canvasWidthMeters: canvasWidthMeters ?? this.canvasWidthMeters,
      canvasHeightMeters: canvasHeightMeters ?? this.canvasHeightMeters,
      roomHeightMeters: roomHeightMeters ?? this.roomHeightMeters,
      listeningHeightMeters: listeningHeightMeters ?? this.listeningHeightMeters,
    );
  }
}

class BlueprintState extends Notifier<BlueprintData> {
  static const _kImagePathKey = 'blueprint_image_path';
  static const _kOpacityKey = 'blueprint_opacity';
  static const _kScaleKey = 'blueprint_scale';

  @override
  BlueprintData build() {
    _loadFromPrefs();
    return const BlueprintData();
  }

  Future<void> _loadFromPrefs() async {
    final prefs = await SharedPreferences.getInstance();
    final path = prefs.getString(_kImagePathKey);
    final opacity = prefs.getDouble(_kOpacityKey) ?? 0.5;
    final scale = prefs.getDouble(_kScaleKey) ?? 50.0;
    final widthM = prefs.getDouble('blueprint_width_m') ?? 40.0;
    final heightM = prefs.getDouble('blueprint_height_m') ?? 40.0;
    final roomHM = prefs.getDouble('blueprint_room_h_m') ?? 5.0;
    final listenHM = prefs.getDouble('blueprint_listen_h_m') ?? 1.2;
    state = BlueprintData(
      imagePath: path,
      opacity: opacity,
      scale: scale,
      canvasWidthMeters: widthM,
      canvasHeightMeters: heightM,
      roomHeightMeters: roomHM,
      listeningHeightMeters: listenHM,
    );
  }

  Future<void> setCanvasDimensions(double widthM, double heightM, {double? roomHM, double? listenHM}) async {
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
  }

  Future<void> setBlueprint(String path) async {
    state = state.copyWith(imagePath: path);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_kImagePathKey, path);
  }

  void updateDimensions({
    double? canvasWidthMeters,
    double? canvasHeightMeters,
    double? roomHeightMeters,
    double? listeningHeightMeters,
  }) {
    state = state.copyWith(
      canvasWidthMeters: canvasWidthMeters,
      canvasHeightMeters: canvasHeightMeters,
      roomHeightMeters: roomHeightMeters,
      listeningHeightMeters: listeningHeightMeters,
    );
  }

  Future<void> setOpacity(double opacity) async {
    state = state.copyWith(opacity: opacity);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setDouble(_kOpacityKey, opacity);
  }

  Future<void> setScale(double scale) async {
    final safeScale = scale < 0.0001 ? 0.0001 : scale;
    state = state.copyWith(scale: safeScale);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setDouble(_kScaleKey, safeScale);
  }

  Future<void> clearBlueprint() async {
    state = const BlueprintData(imagePath: null, opacity: 0.5, scale: 50.0);
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove(_kImagePathKey);
    await prefs.setDouble(_kOpacityKey, 0.5);
    await prefs.setDouble(_kScaleKey, 50.0);
  }
}

final blueprintProvider = NotifierProvider<BlueprintState, BlueprintData>(BlueprintState.new);
