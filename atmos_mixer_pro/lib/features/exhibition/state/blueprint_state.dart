import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

class BlueprintData {
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
    state = BlueprintData(imagePath: path, opacity: opacity, scale: scale);
  }

  Future<void> setBlueprint(String path) async {
    state = state.copyWith(imagePath: path);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_kImagePathKey, path);
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
