// 앱 흐름 통합 테스트(설계: docs/superpowers/specs/2026-10-01-app-flow-integration-test-design.md).
// 실제 앱을 띄워 시스템 기본 출력 장치로 −30dBFS 톤을 낸다. macOS·Windows 실기 전용, 수동 실행:
//   flutter test integration_test/app_flow_test.dart -d macos   (Windows: -d windows)
import 'dart:io';

import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/features/dashboard/screens/dashboard_screen.dart';
import 'package:atmos_mixer_pro/features/exhibition/screens/speaker_canvas_screen.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/three_js_engine_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';
import 'package:atmos_mixer_pro/main.dart' as app;
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

import 'support/isolation.dart';
import 'support/probes.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('앱 흐름: 재생/정지 → 스피커 이동 → FX → 서브 → 재시작 복원 → 테마 시작', (tester) async {
    // 0단계: 사전 확인과 부팅
    await expectNoOtherAppInstance();
    final fixture = await Fixture.create();
    addTearDown(() async {
      try {
        await fixture.root.delete(recursive: true); // 실행마다 임시 폴더에 쌓이지 않게
      } catch (_) {}
    });
    fixture.install();
    // FRB 기본 로더는 작업 디렉터리 기준 rust/target/release/의 dylib을 앱 번들보다 먼저 연다
    // (flutter_rust_bridge loader/_io.dart). macOS의 main.dart는 번들 프레임워크를 직접 열어 이를 피하지만,
    // 그 수정이 빠져도 테스트가 옛 cargo 빌드를 싣지 않도록 작업 디렉터리를 픽스처 폴더로 옮겨 둔다.
    expect(Platform.environment['FRB_DART_LOAD_EXTERNAL_LIBRARY_NATIVE_LIB_DIR'], isNull,
        reason: '0단계: FRB 라이브러리 경로를 덮어쓰는 환경 변수가 있으면 앱 번들의 Rust가 아닐 수 있다');
    final cwd = Directory.current;
    Directory.current = fixture.root;
    addTearDown(() => Directory.current = cwd);
    await app.main(const []);
    await pumpUntil(tester, () => find.byType(DashboardScreen).evaluate().isNotEmpty,
        timeout: const Duration(seconds: 30), reason: '0단계: 30초 안에 대시보드가 뜨지 않았다');
    final container = ProviderScope.containerOf(tester.element(find.byType(app.AtmosMixerProApp)));
    expect(await rust_api.apiIsEngineReady(), isTrue, reason: '0단계: 엔진이 준비되지 않았다');
    // 엔진 상태 스트림은 방송이 있어야 채널 수가 갱신된다. 전체 정지가 방송을 일으킨다.
    await rust_api.apiStopAll();
    await pumpUntil(tester, () => container.read(engineStateProvider).outputChannelCount > 0,
        timeout: const Duration(seconds: 3), reason: '0단계: 출력 채널 수를 받지 못했다');
    final channels = container.read(engineStateProvider).outputChannelCount;
    expect(channels, greaterThanOrEqualTo(2),
        reason: '0단계: 기본 출력 장치가 2채널 미만($channels)이라 CH1·CH2를 검증할 수 없다');
    final vu = VuProbe(container);
    final events = DeviceEventProbe(container);
    final playing = PlayingProbe(container);
    addTearDown(() async {
      vu.close();
      events.close();
      playing.close();
      await rust_api.apiStopAll();
    });

    // 1단계: 재생/정지(대시보드 트랙 카드의 재생 버튼)
    final card200 = find.byKey(const ValueKey(Fixture.t200Loop));
    await tester.ensureVisible(card200);
    await tester.tap(find.descendant(of: card200, matching: find.byTooltip('재생')));
    await pumpUntil(tester,
        () => container.read(engineStateProvider).playingTrackIds.contains(Fixture.t200Loop),
        timeout: const Duration(seconds: 1), reason: '1단계: 재생을 눌렀는데 1초 안에 재생 목록에 없다');
    await pumpUntil(tester, () => vu.sounding(Fixture.mainChannel, window: const Duration(milliseconds: 300)),
        timeout: const Duration(seconds: 1), reason: '1단계: 재생을 눌렀는데 1초 안에 CH2에서 소리가 나지 않는다');
    await tester.tap(find.descendant(of: card200, matching: find.byTooltip('정지')));
    await pumpUntil(tester,
        () => !container.read(engineStateProvider).playingTrackIds.contains(Fixture.t200Loop),
        timeout: const Duration(seconds: 1), reason: '1단계: 정지를 눌렀는데 1초 안에 재생 목록에서 빠지지 않았다');
    await pumpUntil(tester,
        () => vu.silent(Fixture.mainChannel, window: const Duration(milliseconds: 150)),
        timeout: const Duration(seconds: 1), reason: '1단계: 정지를 눌렀는데 1초 안에 CH2가 무음이 되지 않았다');

    // 2단계: 스피커 배치와 인스펙터 열기
    await tester.tap(find.widgetWithText(ElevatedButton, 'Speaker Layout'));
    await pumpUntil(tester, () => find.byType(SpeakerCanvasScreen).evaluate().isNotEmpty,
        timeout: const Duration(seconds: 10), reason: '2단계: 스피커 캔버스 화면으로 가지 못했다');
    await pumpUntil(tester, () => container.read(threeJsEngineProvider).isEngineReady,
        timeout: const Duration(seconds: 10), reason: '2단계: 3D 장면(WebView)이 10초 안에 준비되지 않았다');
    for (var i = 0; i < 2; i++) {
      await tester.tap(find.text('Add 3D Speaker'));
      await pumpFor(tester, const Duration(milliseconds: 300));
    }
    final speakers = container.read(speakerLayoutProvider);
    final ch1Id = speakers.firstWhere((s) => s.channel == Fixture.subChannel).id;
    final ch2Id = speakers.firstWhere((s) => s.channel == Fixture.mainChannel).id;
    // 추가 버튼이 연 인스펙터를 닫고, 노드 탭 메시지로 다시 연다.
    await tester.tap(find
        .descendant(of: find.byType(SpeakerInspectorPanel), matching: find.byIcon(Icons.close))
        .first);
    await pumpUntil(tester, () => find.byType(SpeakerInspectorPanel).evaluate().isEmpty,
        timeout: const Duration(seconds: 2), reason: '2단계: 인스펙터가 닫히지 않았다');
    await tapSpeakerNode(container, ch2Id);
    await pumpUntil(tester, () => find.byType(SpeakerInspectorPanel).evaluate().isNotEmpty,
        timeout: const Duration(seconds: 3), reason: '2단계: CH2 노드 탭 메시지로 인스펙터가 열리지 않았다');
    expect(tester.widget<SpeakerInspectorPanel>(find.byType(SpeakerInspectorPanel)).speakerId, ch2Id,
        reason: '2단계: 인스펙터가 CH2가 아닌 스피커를 열었다');

    // 3단계: 스피커 이동 → FX 반영
    // 튜닝 표는 채널을 1부터 센다(acoustic_sync_provider의 chKey = channel + 1).
    ChannelTuningState? tuningOf(int channel) => container.read(tuningStateProvider)[channel + 1];
    // 튜닝(게인 + EQ)이 200Hz에서 내는 크기(dB). 자동 EQ의 방 모드 보정 밴드가 위치를 따라 200Hz
    // 근처에 생길 수 있어 게인만으로는 출력 변화를 예측할 수 없다. 앱의 EQ 곡선 계산을 쓴다:
    // 20Hz~20kHz 로그 301점 중 100번째가 정확히 200Hz(20 × 1000^(1/3))다.
    double tuningAt200Hz(ChannelTuningState? t) {
      if (t == null) return 0.0; // 튜닝이 아직 없다(자동 계산이 돌지 않음)
      final bands = [
        for (var b = 0; b < t.freqs.length; b++)
          EqBand(
            enabled: t.bandEnabled[b],
            freq: t.freqs[b],
            gain: t.gains[b],
            qFactor: t.qs[b],
            filterType: t.bandTypes[b],
            slopeDbPerOct: t.bandSlopes[b],
          ),
      ];
      return t.gainDb +
          rust_api.apiCalculateEqResponseCurve(bands: bands, numPoints: BigInt.from(301), sampleRate: 48000)[100];
    }

    await rust_api.apiPlayTrack(roomId: Fixture.roomId, trackId: Fixture.t200Loop);
    await pumpFor(tester, const Duration(seconds: 1));
    final g0 = tuningOf(Fixture.mainChannel)?.gainDb ?? 0.0;
    final r0 = tuningAt200Hz(tuningOf(Fixture.mainChannel));
    final subDelay0 = tuningOf(Fixture.subChannel)?.delay ?? 0.0;
    final v0 = vu.peakDb(Fixture.mainChannel);
    expect(v0, greaterThan(soundDb), reason: '3단계: 이동 전 CH2에서 200Hz가 들리지 않는다');
    Future<void> setSlider(String label, double value) async {
      final finder = find.byKey(ValueKey('inspector_slider_$label'));
      await tester.ensureVisible(finder);
      tester.widget<Slider>(finder).onChanged!(value); // 슬라이더 자기 콜백(놓았을 때와 같은 값)
      await tester.pump();
    }

    await setSlider('X Position', 1.5);
    await setSlider('Y Position', 1.5);
    await pumpFor(tester, const Duration(milliseconds: 1500));
    final g1 = tuningOf(Fixture.mainChannel)?.gainDb ?? 0.0;
    final r1 = tuningAt200Hz(tuningOf(Fixture.mainChannel));
    final subDelay1 = tuningOf(Fixture.subChannel)?.delay ?? 0.0;
    final v1 = vu.peakDb(Fixture.mainChannel);
    debugPrint('3단계: CH2 게인 $g0→$g1 dB, CH2 튜닝 200Hz 응답 $r0→$r1 dB, '
        'CH1 딜레이 $subDelay0→$subDelay1 ms, CH2 피크 $v0→$v1 dBFS');
    expect((g1 - g0).abs(), greaterThanOrEqualTo(1.0),
        reason: '3단계: CH2를 옮겼는데 CH2 튜닝 게인이 바뀌지 않았다(FX가 스피커를 안 따라감)');
    expect(subDelay1 - subDelay0, greaterThanOrEqualTo(3.0),
        reason: '3단계: CH2가 가장 멀어졌는데 CH1 시간 정렬 딜레이가 늘지 않았다');
    expect((r1 - r0).abs(), greaterThanOrEqualTo(1.0),
        reason: '3단계: 이동으로 200Hz 튜닝 응답이 1dB 넘게 바뀌지 않아 엔진 반영을 판정할 수 없다(목표 위치를 조정할 것)');
    expect(((v1 - v0) - (r1 - r0)).abs(), lessThanOrEqualTo(0.5),
        reason: '3단계: 튜닝 200Hz 응답 변화(${(r1 - r0).toStringAsFixed(2)}dB)가 CH2 출력'
            '(${(v1 - v0).toStringAsFixed(2)}dB)에 반영되지 않았다');
    await rust_api.apiStopTrack(roomId: Fixture.roomId, trackId: Fixture.t200Loop);
    await pumpUntil(tester,
        () => vu.silent(Fixture.mainChannel, window: const Duration(milliseconds: 150)),
        timeout: const Duration(seconds: 2), reason: '3단계: 정지 후 무음이 되지 않았다');

    // 4단계: 서브 라우팅
    await tapSpeakerNode(container, ch1Id);
    await pumpUntil(tester,
        () => tester
            .widgetList<SpeakerInspectorPanel>(find.byType(SpeakerInspectorPanel))
            .any((p) => p.speakerId == ch1Id),
        timeout: const Duration(seconds: 3), reason: '4단계: CH1 노드 탭 메시지로 인스펙터가 CH1로 바뀌지 않았다');
    // 인스펙터는 ListView라 화면 밖 카드는 아직 만들어지지 않았다. 스크롤해서 꺼낸다.
    final inspectorScroll = find
        .descendant(of: find.byType(SpeakerInspectorPanel), matching: find.byType(Scrollable))
        .first;
    await tester.scrollUntilVisible(find.text('Bass Management'), 200, scrollable: inspectorScroll);
    await tester.tap(find.text('Bass Management'));
    await pumpFor(tester, const Duration(milliseconds: 400));
    final lfeSwitch = find.byKey(const ValueKey('inspector_lfe_switch'));
    await tester.scrollUntilVisible(lfeSwitch, 100, scrollable: inspectorScroll);
    // 기본 창(1024×768)에서는 하단 오버레이가 스위치를 덮어 탭이 닿지 않는다. 스위치 자기 콜백을 부른다.
    tester.widget<Switch>(lfeSwitch).onChanged!(true);
    await tester.pump();
    await pumpUntil(tester,
        () => container.read(speakerLayoutProvider).firstWhere((s) => s.id == ch1Id).isSubwoofer,
        timeout: const Duration(seconds: 2), reason: '4단계: 스위치를 켰는데 CH1이 서브로 지정되지 않았다');
    await pumpFor(tester, const Duration(seconds: 1));

    // 채널 피크를 그 채널 튜닝 게인으로 나눠 비교한다(거리 보정이 레벨을 바꾸므로).
    double normDb(int ch) => vu.peakDb(ch) - (tuningOf(ch)?.gainDb ?? 0.0);

    await rust_api.apiPlayTrack(roomId: Fixture.roomId, trackId: Fixture.t200Loop);
    await pumpFor(tester, const Duration(seconds: 1));
    final split200 = normDb(Fixture.mainChannel) - normDb(Fixture.subChannel);
    debugPrint('4단계: 200Hz 메인−서브 ${split200.toStringAsFixed(1)}dB (LR24 이론 약 32dB)');
    expect(split200, greaterThanOrEqualTo(20.0),
        reason: '4단계: 200Hz인데 서브(CH1)가 메인(CH2)보다 20dB 이상 작지 않다');
    await rust_api.apiStopTrack(roomId: Fixture.roomId, trackId: Fixture.t200Loop);
    await rust_api.apiPlayTrack(roomId: Fixture.roomId, trackId: Fixture.t50Loop);
    await pumpFor(tester, const Duration(seconds: 1));
    final split50 = normDb(Fixture.subChannel) - normDb(Fixture.mainChannel);
    debugPrint('4단계: 50Hz 서브−메인 ${split50.toStringAsFixed(1)}dB (LR24 이론 약 16dB)');
    expect(split50, greaterThanOrEqualTo(10.0),
        reason: '4단계: 50Hz인데 서브(CH1)가 메인(CH2)보다 10dB 이상 크지 않다(저역이 서브로 가지 않음)');
    await rust_api.apiStopTrack(roomId: Fixture.roomId, trackId: Fixture.t50Loop);
    await pumpUntil(tester,
        () => vu.silent(Fixture.mainChannel, window: const Duration(milliseconds: 150)),
        timeout: const Duration(seconds: 2), reason: '4단계: 정지 후 무음이 되지 않았다');

    // 대시보드로 돌아간다(5·6단계의 버튼이 대시보드에 있다).
    Navigator.of(tester.element(find.byType(SpeakerCanvasScreen))).pop();
    await pumpUntil(tester, () => find.byType(SpeakerCanvasScreen).evaluate().isEmpty,
        timeout: const Duration(seconds: 3), reason: '4단계: 대시보드로 돌아가지 못했다');

    // 5단계: 재시작 복원 — 자기 재시작 경로
    Future<Map<String, double>> positions() async => {
          for (final p in await rust_api.apiGetPlaybackPositions()) p.trackId: p.seconds,
        };
    await rust_api.apiPlayTrack(roomId: Fixture.roomId, trackId: Fixture.t50Long);
    await rust_api.apiPlayTrack(roomId: Fixture.roomId, trackId: Fixture.t200Long);
    await pumpFor(tester, const Duration(seconds: 6));
    final p0 = await positions();
    final recordedAt = DateTime.now();
    final pre = {
      for (final ch in [Fixture.subChannel, Fixture.mainChannel])
        ch: vu.peakDb(ch, window: const Duration(milliseconds: 500)),
    };
    expect(p0.keys, containsAll([Fixture.t50Long, Fixture.t200Long]),
        reason: '5단계: 재시작 전 재생 위치를 읽지 못했다');
    final eventsBefore = events.events.length;
    final injectedAt = DateTime.now();
    requestEngineRecovery();
    await pumpUntil(tester,
        () => events.events.skip(eventsBefore).any((e) => e.$2.startsWith('EngineRestarted:')),
        timeout: const Duration(seconds: 10), reason: '5단계: 결함 주입 뒤 10초 안에 엔진 재시작 이벤트가 오지 않았다');
    final restartedAt =
        events.events.skip(eventsBefore).firstWhere((e) => e.$2.startsWith('EngineRestarted:')).$1;
    // 재개 순간은 재생 목록으로 잰다(재개는 목록에 먼저 올리고 재생 명령을 보낸다). 새 엔진은 부팅 뮤트
    // 램프(3초, mixer.rs StartupMuteRamp)에서 시작해 재개 직후 레벨이 낮아 VU 문턱으로는 잡히지 않는다.
    await pumpUntil(tester, () => playing.refilledAfter(injectedAt) != null,
        timeout: const Duration(seconds: 5), reason: '5단계: 재시작 뒤 5초 안에 재생이 다시 이어지지 않았다');
    final resumedAt = playing.refilledAfter(injectedAt)!;
    expect(resumedAt.isAfter(restartedAt), isTrue,
        reason: '5단계: 재시작 알림보다 재개가 먼저였다(재동기화 전에 재생, R3)');
    // 재시작 이벤트 뒤부터 재개 전까지 디지털 무음(옛 DSP 소리·클릭 없음).
    for (final ch in [Fixture.subChannel, Fixture.mainChannel]) {
      // 대기 구간은 100ms 남짓이라 기계가 바쁘면(재동기화로 Dart가 잠깐 막힘) 그 안에 표본이 없을 수 있다.
      // VU 스트림이 살아 있는지는 앞뒤 0.5초까지 넓혀 본다(멈춘 스트림을 무음으로 착각하지 않게).
      final around = vu.samplesDb(ch, restartedAt.subtract(const Duration(milliseconds: 500)),
          resumedAt.add(const Duration(milliseconds: 500)));
      expect(around, isNotEmpty, reason: '5단계: 재시작 앞뒤로 CH${ch + 1} VU 표본이 없다(VU 스트림 멈춤)');
      final window = vu.samplesDb(ch, restartedAt, resumedAt.subtract(const Duration(milliseconds: 20)));
      final loud = window.where((db) => db > silenceDb).length;
      expect(loud, 0, reason: '5단계: 재시작 이벤트 후 재개 전 CH${ch + 1}에서 소리가 났다($loud개 표본)');
    }
    await pumpUntil(tester,
        () => vu.sounding(Fixture.mainChannel, window: const Duration(milliseconds: 100)),
        timeout: const Duration(seconds: 5), reason: '5단계: 재개 뒤 5초 안에 CH2 소리가 돌아오지 않았다');
    await pumpFor(tester, const Duration(milliseconds: 800)); // 재개 페이드 인(300ms)
    final p1 = await positions();
    final elapsed = DateTime.now().difference(recordedAt).inMilliseconds / 1000.0;
    for (final id in [Fixture.t50Long, Fixture.t200Long]) {
      expect(p1.containsKey(id), isTrue, reason: '5단계: 재시작 뒤 $id가 다시 재생되지 않았다');
      final a = p0[id]!, b = p1[id]!;
      debugPrint('5단계: $id 위치 ${a.toStringAsFixed(2)}s → ${b.toStringAsFixed(2)}s (흐른 시간 ${elapsed.toStringAsFixed(2)}s)');
      expect(b, greaterThanOrEqualTo(3.0), reason: '5단계: $id가 처음부터 다시 재생됐다(${b}s)');
      expect(b, inInclusiveRange(a, a + elapsed + 0.5),
          reason: '5단계: $id 재개 위치 ${b}s가 기록 ${a}s 기준 허용 범위를 벗어났다');
    }
    expect(container.read(engineStateProvider).playingTrackIds.toSet(), {Fixture.t50Long, Fixture.t200Long},
        reason: '5단계: 재생 목록이 실제 재생과 다르다(옛 인스턴스가 남았거나 빠짐)');
    // 레벨은 부팅 뮤트 램프가 끝난 뒤 비교한다. 램프는 새 엔진 첫 콜백부터 3초이고 재시작 이벤트는 그 뒤에 온다.
    await pumpFor(tester, restartedAt.add(const Duration(milliseconds: 3600)).difference(DateTime.now()));
    for (final ch in [Fixture.subChannel, Fixture.mainChannel]) {
      final post = vu.peakDb(ch, window: const Duration(milliseconds: 500));
      debugPrint('5단계: CH${ch + 1} ${pre[ch]!.toStringAsFixed(2)}→${post.toStringAsFixed(2)}dBFS '
          '(재시작 이벤트→재개 ${resumedAt.difference(restartedAt).inMilliseconds}ms)');
      expect((post - pre[ch]!).abs(), lessThanOrEqualTo(1.5),
          reason: '5단계: 재시작 뒤 CH${ch + 1} 레벨이 ${pre[ch]!.toStringAsFixed(1)}→'
              '${post.toStringAsFixed(1)}dBFS로 바뀌었다(서브 라우팅·FX 복원 실패)');
    }
    await rust_api.apiStopAll();
    await pumpUntil(tester,
        () => vu.silent(Fixture.mainChannel, window: const Duration(milliseconds: 150)),
        timeout: const Duration(seconds: 2), reason: '5단계: 전체 정지 후 무음이 되지 않았다');

    // 5단계(이어서): 재기동 창 안의 명령(세대 확인). 재시작 사이클 전체를 5번 반복한다.
    for (var i = 1; i <= 5; i++) {
      await rust_api.apiStopAudioEngine(); // 세대만 올린다. 옛 엔진은 0~100ms 안에 drop된다.
      await rust_api.apiPlayTrack(roomId: Fixture.roomId, trackId: Fixture.t200Loop);
      await rust_api.apiInitAudioSystem(deviceName: null);
      await pumpUntil(tester,
          () => vu.sounding(Fixture.mainChannel, window: const Duration(milliseconds: 100)),
          timeout: const Duration(seconds: 5),
          reason: '5단계 재기동 창 검사 $i회차: 재기동 직전에 보낸 재생 명령이 새 엔진에서 재생되지 않았다(옛 엔진이 가져감)');
      await rust_api.apiStopTrack(roomId: Fixture.roomId, trackId: Fixture.t200Loop);
      await pumpUntil(tester,
          () => vu.silent(Fixture.mainChannel, window: const Duration(milliseconds: 150)),
          timeout: const Duration(seconds: 2), reason: '5단계 재기동 창 검사 $i회차: 정지 후 무음이 되지 않았다');
    }

    // 6단계: 테마 시작(대시보드 Start)
    final themeStartAt = DateTime.now();
    await tester.tap(find.widgetWithText(ElevatedButton, 'Start'));
    await pumpUntil(tester, () {
      final ids = container.read(engineStateProvider).playingTrackIds;
      return ids.contains(Fixture.t200Loop) && ids.contains(Fixture.t50Loop);
    }, timeout: const Duration(seconds: 3), reason: '6단계: Start를 눌렀는데 첫 방의 루프 트랙이 재생 목록에 없다');
    await pumpFor(tester, const Duration(seconds: 1)); // 재생 페이드 인(300ms)
    final normal = {
      for (final ch in [Fixture.subChannel, Fixture.mainChannel])
        ch: vu.peakDb(ch, window: const Duration(milliseconds: 500)),
    };
    expect(normal[Fixture.mainChannel]!, greaterThan(soundDb),
        reason: '6단계: 테마 시작 후 CH2에서 소리가 나지 않는다');
    final watchFrom = DateTime.now();
    await pumpFor(tester, const Duration(seconds: 5)); // 2초 루프 두 바퀴 이상
    final watchTo = DateTime.now();
    for (final ch in normal.keys) {
      if (normal[ch]! <= soundDb) continue; // 서브 라우팅이 없는 상태면 CH1은 원래 조용하다
      final watched = vu.samplesDb(ch, watchFrom, watchTo);
      expect(watched.length, greaterThan(100), reason: '6단계: 5초 동안 CH${ch + 1} VU 표본이 ${watched.length}개뿐이다(VU 스트림 멈춤)');
      final dips = watched.where((db) => db < normal[ch]! - 6.0).length;
      expect(dips, 0,
          reason: '6단계: 루프 재생 중 CH${ch + 1} 출력이 $dips번 끊겼다'
              '(정상 ${normal[ch]!.toStringAsFixed(1)}dBFS보다 6dB 넘게 낮음)');
    }
    final restarts = events.events
        .where((e) => e.$1.isAfter(themeStartAt) && e.$2.startsWith('EngineRestarted:'))
        .length;
    expect(restarts, 0, reason: '6단계: 테마 재생 중 엔진이 스스로 재시작했다(워치독)');
    await tester.tap(find.widgetWithText(ElevatedButton, 'Emergency'));
    await pumpUntil(tester,
        () => vu.silent(Fixture.mainChannel, window: const Duration(milliseconds: 150)),
        timeout: const Duration(seconds: 2), reason: '6단계: Emergency를 눌렀는데 소리가 멈추지 않았다');
  }, timeout: const Timeout(Duration(minutes: 8)));
}
