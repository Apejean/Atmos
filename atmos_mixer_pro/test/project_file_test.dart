import 'dart:convert';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:atmos_mixer_pro/core/state/project_file.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/speaker_node.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/bass_management_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/spatial_reverb_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/spatial_sync_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/trajectory_state.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';

/// 프로젝트 파일(.atmos)은 현장 컴퓨터로 설계를 통째로 옮기는 수단이다.
/// 엔진 설정만 담겨 있으면 스피커 배치·방·리버브 같은 설계가 새 컴퓨터에서 사라진다.
void main() {
  Future<void> settle() => Future.delayed(const Duration(milliseconds: 300));

  // 엔진이 쓰는 설정 파일 모양(실제 AppConfig의 일부).
  String engineConfigJson() => jsonEncode({
        'osc_port': 8000,
        'device_name': '[ASIO] RME Fireface',
        'buffer_size': 512,
        'rooms': [],
        'room_zones': [],
      });

  test('설계 데이터를 넣어도 엔진 설정 항목은 그대로 남는다', () {
    final merged = injectExhibitionSection(engineConfigJson(), {'tuning_state': '{}'});
    final decoded = jsonDecode(merged) as Map<String, dynamic>;

    expect(decoded['osc_port'], 8000);
    expect(decoded['device_name'], '[ASIO] RME Fireface');
    expect(decoded['buffer_size'], 512);
    expect(decoded[kExhibitionSectionKey], isA<Map<String, dynamic>>());
  });

  test('설계 데이터가 없는 예전 파일도 그대로 열린다', () {
    expect(readExhibitionSection(engineConfigJson()), isNull);
    expect(readExhibitionSection('{깨진 json'), isNull);
  });

  test('빈 컴퓨터에서도 파일만으로 설계가 복원된다', () async {
    // 설계용 컴퓨터: 스피커·방·리버브·베이스·튜닝·청사진이 저장된 상태.
    SharedPreferences.setMockInitialValues({
      'exhibition_speaker_layout': '[{"id":"spk_0","channel":0,"x":1.0,"y":2.0}]',
      'exhibition_room_zone_layout': '[{"id":"room_1","x":0.0,"y":0.0,"width":100.0,"height":100.0,"color":4278190080}]',
      'exhibition_trajectory_layout': '[]',
      'spatial_reverb_state': '{"selectedChannel":1}',
      'bass_management_state': '{"isEnabled":true,"lfeChannel":3,"crossoverFreq":120.0}',
      'tuning_state': '{"0":{}}',
      'blueprint_image_path': '/Users/me/plan.png',
      'blueprint_opacity': 0.4,
      'blueprint_scale': 50.0,
      'blueprint_width_m': 30.0,
      'blueprint_height_m': 25.0,
    });
    final design = await collectExhibitionData();
    final fileContent = injectExhibitionSection(engineConfigJson(), design);

    // 현장 컴퓨터: 아무것도 저장되어 있지 않다.
    SharedPreferences.setMockInitialValues({});
    final restored = readExhibitionSection(fileContent);
    expect(restored, isNotNull);
    await writeExhibitionDataToPrefs(restored!);

    final prefs = await SharedPreferences.getInstance();
    expect(prefs.getString('exhibition_speaker_layout'),
        '[{"id":"spk_0","channel":0,"x":1.0,"y":2.0}]');
    expect(prefs.getString('bass_management_state'),
        '{"isEnabled":true,"lfeChannel":3,"crossoverFreq":120.0}');
    expect(prefs.getString('tuning_state'), '{"0":{}}');
    expect(prefs.getString('blueprint_image_path'), '/Users/me/plan.png');
    expect(prefs.getDouble('blueprint_opacity'), 0.4);
    expect(prefs.getDouble('blueprint_width_m'), 30.0);
  });

  test('실제 파일로 저장했다가 다시 불러온다', () async {
    final dir = await Directory.systemTemp.createTemp('atmos_project_test');
    final path = '${dir.path}/project.atmos';
    // 엔진이 설정을 먼저 쓴 상태.
    await File(path).writeAsString(engineConfigJson());

    SharedPreferences.setMockInitialValues({
      'exhibition_speaker_layout': '[{"id":"spk_1","channel":2}]',
      'bass_management_state': '{"isEnabled":true,"lfeChannel":4,"crossoverFreq":90.0}',
      'blueprint_scale': 60.0,
    });
    await appendExhibitionDataToFile(path);

    // 엔진 설정은 그대로 남아 있어야 한다.
    final saved = jsonDecode(await File(path).readAsString()) as Map<String, dynamic>;
    expect(saved['device_name'], '[ASIO] RME Fireface');

    // 현장 컴퓨터(빈 저장소)에서 불러오기.
    SharedPreferences.setMockInitialValues({});
    expect(await restoreExhibitionDataFromFile(path), isTrue);
    final prefs = await SharedPreferences.getInstance();
    expect(prefs.getString('exhibition_speaker_layout'), '[{"id":"spk_1","channel":2}]');
    expect(prefs.getDouble('blueprint_scale'), 60.0);

    // 설계 데이터가 없는 예전 파일은 false(건너뜀)를 돌려준다.
    final oldPath = '${dir.path}/old.atmos';
    await File(oldPath).writeAsString(engineConfigJson());
    expect(await restoreExhibitionDataFromFile(oldPath), isFalse);

    await dir.delete(recursive: true);
  });

  test('방금 옮긴 스피커도 프로젝트 파일에 들어간다', () async {
    // 스피커 드래그·방 편집·리버브 조작은 저장을 300ms 미룬다(디스크 I/O 절약).
    // 저장을 미룬 상태에서 프로젝트를 저장하면 마지막 변경이 빠진다.
    SharedPreferences.setMockInitialValues({});
    final container = ProviderContainer();
    container.read(speakerLayoutProvider);
    await settle();

    final layout = container.read(speakerLayoutProvider.notifier);
    layout.addSpeaker(SpeakerNode(id: 'spk_a', x: 1.0, y: 1.0, channel: 0));
    await settle();
    // 드래그 직후처럼 저장이 미뤄진 상태를 만든다.
    layout.updateSpeaker(
      SpeakerNode(id: 'spk_a', x: 7.5, y: 2.5, channel: 0),
    );

    await flushExhibitionSaves(
      layout: layout,
      rooms: container.read(roomZoneProvider.notifier),
      trajectories: container.read(trajectoryProvider.notifier),
      reverb: container.read(spatialReverbProvider.notifier),
    );
    final design = await collectExhibitionData();
    final saved = jsonDecode(design['exhibition_speaker_layout'] as String) as List;

    expect(saved.single['x'], 7.5, reason: '미뤄진 저장이 프로젝트 파일에 반영되지 않았다');
    container.dispose();
  });

  test('다른 프로젝트를 불러오면 이전 프로젝트 값이 남지 않는다', () async {
    // 프로젝트 A가 저장된 컴퓨터(백업 키까지 있는 상태).
    SharedPreferences.setMockInitialValues({
      'exhibition_speaker_layout': '[{"id":"spk_old","channel":0}]',
      'exhibition_speaker_layout_backup': '[{"id":"spk_older","channel":1}]',
      'exhibition_room_zone_layout': '[{"id":"room_old","x":0.0,"y":0.0,"width":10.0,"height":10.0,"color":4278190080}]',
      'spatial_reverb_state': '{"selectedChannel":3}',
      'bass_management_state': '{"isEnabled":true,"lfeChannel":5,"crossoverFreq":150.0}',
    });

    // 프로젝트 B: 방만 있고 스피커·리버브·베이스 설정은 없다.
    await writeExhibitionDataToPrefs({
      'exhibition_room_zone_layout':
          '[{"id":"room_new","x":0.0,"y":0.0,"width":20.0,"height":20.0,"color":4278190080}]',
    });

    final prefs = await SharedPreferences.getInstance();
    expect(prefs.getString('exhibition_room_zone_layout'), contains('room_new'));
    expect(prefs.getString('exhibition_speaker_layout'), isNull,
        reason: '이전 프로젝트의 스피커가 남았다');
    expect(prefs.getString('exhibition_speaker_layout_backup'), isNull,
        reason: '백업이 이전 프로젝트 배치를 되살릴 수 있다');
    expect(prefs.getString('spatial_reverb_state'), isNull);
    expect(prefs.getString('bass_management_state'), isNull);

    // 상태도 기본값으로 돌아가야 한다.
    final container = ProviderContainer();
    final layout = container.read(speakerLayoutProvider.notifier);
    final bass = container.read(bassManagementProvider.notifier);
    container.read(speakerLayoutProvider);
    container.read(bassManagementProvider);
    await settle();
    await layout.reloadFromPrefs();
    await bass.reloadFromPrefs();
    await settle();
    expect(container.read(speakerLayoutProvider), isEmpty);
    expect(container.read(bassManagementProvider).crossoverFreq, 80.0,
        reason: '이전 프로젝트의 크로스오버(150Hz)가 남았다');
    container.dispose();
  });

  test('불러오기 후 화면 상태가 즉시 새 설계로 바뀐다', () async {
    // 앱이 이미 떠 있고(빈 상태), 그 상태에서 프로젝트를 불러오는 상황.
    SharedPreferences.setMockInitialValues({});
    final container = ProviderContainer();
    container.read(speakerLayoutProvider);
    container.read(roomZoneProvider);
    container.read(bassManagementProvider);
    await settle();
    expect(container.read(speakerLayoutProvider), isEmpty);

    final design = {
      'exhibition_speaker_layout': jsonEncode([
        SpeakerNode(id: 'spk_9', roomId: 'room_9', x: 3.0, y: 4.0, channel: 5).toJson(),
        SpeakerNode(
                id: 'sub_9', roomId: 'room_9', x: 1.0, y: 1.0, channel: 2, isSubwoofer: true)
            .toJson(),
      ]),
      'exhibition_room_zone_layout': jsonEncode([
        RoomZone(
          id: 'room_9',
          x: 0,
          y: 0,
          width: 100,
          height: 100,
          color: 0xFF000000,
          physicalWidth: 12.0,
          physicalHeight: 9.0,
        ).toJson(),
      ]),
      'bass_management_state': '{"crossoverFreq":100.0,"lfeBoostEnabled":false}',
    };
    await writeExhibitionDataToPrefs(design);
    // 앱이 부르는 것과 같은 형태로, 설계에 관여하는 상태를 전부 다시 읽는다.
    await reloadExhibitionProviders(
      layout: container.read(speakerLayoutProvider.notifier),
      rooms: container.read(roomZoneProvider.notifier),
      trajectories: container.read(trajectoryProvider.notifier),
      reverb: container.read(spatialReverbProvider.notifier),
      bass: container.read(bassManagementProvider.notifier),
      tuning: container.read(tuningStateProvider.notifier),
      blueprint: container.read(blueprintProvider.notifier),
      spatial: container.read(spatialSyncProvider.notifier),
    );
    await settle();

    final speakers = container.read(speakerLayoutProvider);
    expect(speakers.firstWhere((n) => n.id == 'spk_9').channel, 5);
    expect(speakers.firstWhere((n) => n.id == 'sub_9').isSubwoofer, isTrue,
        reason: '서브우퍼 지정이 프로젝트 파일로 옮겨지지 않았다');
    expect(container.read(roomZoneProvider).single.physicalWidth, 12.0);
    expect(container.read(bassManagementProvider).crossoverFreq, 100.0);
    container.dispose();
  });

  test('예전 프로젝트 파일(전역 LFE 지정)을 불러와도 서브가 그 채널 스피커로 옮겨진다', () async {
    // 방별 베이스 매니지먼트 이전 파일: 서브가 베이스 매니지먼트 설정의 lfeChannel 하나였다.
    SharedPreferences.setMockInitialValues({});
    final container = ProviderContainer();
    container.read(speakerLayoutProvider);
    container.read(bassManagementProvider);
    await settle();

    await writeExhibitionDataToPrefs({
      'exhibition_speaker_layout': jsonEncode([
        SpeakerNode(id: 'main', roomId: 'room_1', x: 1.0, y: 1.0, channel: 0).toJson()
          ..remove('is_subwoofer'),
        SpeakerNode(id: 'sub', roomId: 'room_1', x: 2.0, y: 1.0, channel: 4).toJson()
          ..remove('is_subwoofer'),
      ]),
      'bass_management_state': '{"isEnabled":true,"lfeChannel":4,"crossoverFreq":90.0}',
    });
    await reloadExhibitionProviders(
      layout: container.read(speakerLayoutProvider.notifier),
      rooms: container.read(roomZoneProvider.notifier),
      trajectories: container.read(trajectoryProvider.notifier),
      reverb: container.read(spatialReverbProvider.notifier),
      bass: container.read(bassManagementProvider.notifier),
      tuning: container.read(tuningStateProvider.notifier),
      blueprint: container.read(blueprintProvider.notifier),
      spatial: container.read(spatialSyncProvider.notifier),
    );
    await settle();

    final speakers = container.read(speakerLayoutProvider);
    expect(speakers.firstWhere((n) => n.id == 'sub').isSubwoofer, isTrue);
    expect(speakers.firstWhere((n) => n.id == 'main').isSubwoofer, isFalse);
    expect(container.read(bassManagementProvider).crossoverFreq, 90.0);
    container.dispose();
  });
}
