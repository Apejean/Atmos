import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:atmos_mixer_pro/features/exhibition/state/bass_management_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/spatial_reverb_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/spatial_sync_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/trajectory_state.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';

/// 프로젝트 파일(.atmos)에 설계 데이터를 담는 항목 이름.
///
/// 파일 자체는 엔진 설정(Rust `AppConfig`)의 JSON이다. 엔진은 모르는 항목을
/// 무시하므로(serde 기본 동작), 같은 파일에 이 항목 하나를 더 얹어서 파일 하나로
/// 유지한다. 이 항목이 없는 예전 파일도 그대로 열린다.
const String kExhibitionSectionKey = 'exhibition_design';

/// 설계 단계에서 만들어져 이 컴퓨터에만 저장되던 값들.
///
/// 엔진 설정에는 오디오 장치·버퍼·채널 라우팅·트랙만 들어간다. 아래 값들은
/// 화면 쪽 저장소(SharedPreferences)에만 있어서, 예전에는 새 컴퓨터로 파일을
/// 옮기면 스피커 배치와 방 설계가 통째로 사라졌다.
const List<String> kExhibitionPrefsKeys = [
  'exhibition_speaker_layout', // 스피커 위치·각도·방·채널·초기반사·리버브 센드
  'exhibition_room_zone_layout', // 방 형상과 재질
  'exhibition_trajectory_layout', // 궤적
  'spatial_reverb_state', // 채널별 공간 리버브
  'bass_management_state', // 크로스오버·LFE +10dB(서브 지정은 스피커 배치에 있다)
  'tuning_state', // 채널별 딜레이·EQ·게인·위상(자동/수동 잠금 포함)
  'tuning_state_schema_version',
  'blueprint_image_path', // 도면 이미지 경로(이미지 파일 자체는 따로 옮겨야 한다)
  'blueprint_opacity',
  'blueprint_scale',
  'blueprint_width_m',
  'blueprint_height_m',
];

/// 현재 컴퓨터에 저장된 설계 데이터를 모은다.
Future<Map<String, dynamic>> collectExhibitionData() async {
  final prefs = await SharedPreferences.getInstance();
  final data = <String, dynamic>{};
  for (final key in kExhibitionPrefsKeys) {
    final value = prefs.get(key);
    if (value != null) data[key] = value;
  }
  return data;
}

/// 엔진이 쓴 설정 JSON에 설계 데이터를 얹는다.
String injectExhibitionSection(String configJson, Map<String, dynamic> data) {
  final decoded = jsonDecode(configJson) as Map<String, dynamic>;
  decoded[kExhibitionSectionKey] = data;
  return const JsonEncoder.withIndent('  ').convert(decoded);
}

/// 프로젝트 파일에서 설계 데이터를 꺼낸다. 없거나 깨졌으면 null.
Map<String, dynamic>? readExhibitionSection(String fileContent) {
  try {
    final decoded = jsonDecode(fileContent);
    if (decoded is! Map<String, dynamic>) return null;
    final section = decoded[kExhibitionSectionKey];
    if (section is! Map) return null;
    return Map<String, dynamic>.from(section);
  } catch (_) {
    return null;
  }
}

/// 저장 실패에 대비한 예비 사본 키. 프로젝트를 새로 불러올 때는 지운다.
/// 남겨두면 새 프로젝트에 스피커가 없을 때 예전 프로젝트의 배치가 되살아난다.
const List<String> _kExhibitionBackupKeys = [
  'exhibition_speaker_layout_backup',
  'exhibition_room_zone_layout_backup',
];

/// 불러온 설계 데이터를 이 컴퓨터의 저장소에 쓴다.
///
/// 불러온 파일에 없는 항목은 **지운다**. 그러지 않으면 다른 프로젝트를 열었을 때
/// 이전 프로젝트의 스피커나 리버브가 섞인 채로 남는다.
Future<void> writeExhibitionDataToPrefs(Map<String, dynamic> data) async {
  final prefs = await SharedPreferences.getInstance();
  for (final key in _kExhibitionBackupKeys) {
    await prefs.remove(key);
  }
  for (final key in kExhibitionPrefsKeys) {
    final value = data[key];
    if (value == null) {
      await prefs.remove(key);
      continue;
    }
    if (value is String) {
      await prefs.setString(key, value);
    } else if (value is bool) {
      await prefs.setBool(key, value);
    } else if (value is int) {
      await prefs.setInt(key, value);
    } else if (value is double) {
      await prefs.setDouble(key, value);
    } else if (value is num) {
      await prefs.setDouble(key, value.toDouble());
    }
  }
}

/// 미뤄둔 저장(스피커 드래그·방 편집·리버브 노브는 300ms 디바운스)을 먼저 끝낸다.
/// 프로젝트 파일에 방금 한 변경이 빠지지 않게 하려면 저장 직전에 불러야 한다.
Future<void> flushExhibitionSaves({
  required SpeakerLayoutState layout,
  required RoomZoneState rooms,
  required TrajectoryState trajectories,
  required SpatialReverbNotifier reverb,
}) async {
  await layout.flushPendingSave();
  await rooms.flushPendingSave();
  await trajectories.flushPendingSave();
  await reverb.flushPendingSave();
}

/// 위젯에서 부르는 형태.
Future<void> flushExhibitionSavesFromWidgetRef(WidgetRef ref) => flushExhibitionSaves(
      layout: ref.read(speakerLayoutProvider.notifier),
      rooms: ref.read(roomZoneProvider.notifier),
      trajectories: ref.read(trajectoryProvider.notifier),
      reverb: ref.read(spatialReverbProvider.notifier),
    );

/// 저장이 끝난 프로젝트 파일에 설계 데이터를 덧붙인다.
/// 엔진이 파일을 쓴 **다음에** 부른다(엔진 저장이 파일을 통째로 덮어쓰기 때문).
Future<void> appendExhibitionDataToFile(String path) async {
  final file = File(path);
  final configJson = await file.readAsString();
  final merged = injectExhibitionSection(configJson, await collectExhibitionData());
  await file.writeAsString(merged);
}

/// 프로젝트 파일의 설계 데이터를 이 컴퓨터에 복원한다.
/// 설계 데이터가 없는 예전 파일이면 아무것도 하지 않고 false를 돌려준다.
Future<bool> restoreExhibitionDataFromFile(String path) async {
  try {
    final section = readExhibitionSection(await File(path).readAsString());
    if (section == null) return false;
    await writeExhibitionDataToPrefs(section);
    return true;
  } catch (e) {
    debugPrint('설계 데이터 복원 실패: $e');
    return false;
  }
}

/// 저장소에 복원된 설계를 화면 상태에 즉시 반영한다.
/// 각 상태가 저장소를 다시 읽고, 그 과정에서 엔진에도 값을 다시 보낸다.
Future<void> reloadExhibitionProviders({
  required SpeakerLayoutState layout,
  required RoomZoneState rooms,
  required TrajectoryState trajectories,
  required SpatialReverbNotifier reverb,
  required BassManagementNotifier bass,
  required TuningStateNotifier tuning,
  required BlueprintState blueprint,
  required SpatialSyncNotifier spatial,
}) async {
  // 방을 먼저 읽어야 스피커가 자기 방 치수로 자동 조준된다.
  await rooms.reloadFromPrefs();
  await blueprint.reloadFromPrefs();
  await layout.reloadFromPrefs();
  await trajectories.reloadFromPrefs();
  await reverb.reloadFromPrefs();
  await bass.reloadFromPrefs();
  await tuning.reloadFromPrefs();
  // 불러온 배치를 엔진에 한 번 확실히 보낸다(상태 변경 알림만 믿지 않는다).
  spatial.sendNow();
}

/// 위젯에서 부르는 형태.
Future<void> reloadExhibitionProvidersFromWidgetRef(WidgetRef ref) =>
    reloadExhibitionProviders(
      layout: ref.read(speakerLayoutProvider.notifier),
      rooms: ref.read(roomZoneProvider.notifier),
      trajectories: ref.read(trajectoryProvider.notifier),
      reverb: ref.read(spatialReverbProvider.notifier),
      bass: ref.read(bassManagementProvider.notifier),
      tuning: ref.read(tuningStateProvider.notifier),
      blueprint: ref.read(blueprintProvider.notifier),
      spatial: ref.read(spatialSyncProvider.notifier),
    );
