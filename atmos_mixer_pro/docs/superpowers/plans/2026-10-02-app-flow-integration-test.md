# 앱 흐름 통합 테스트와 재시작 재생 복원 — 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 실제 앱에서 재생/정지 → 스피커 이동 → FX → 서브 라우팅 → 재시작 복원 → 테마 시작을 자동 조작·검증하는 integration_test를 만들고, 그 테스트가 요구하는 "엔진 자기 재시작 뒤 멈춘 위치부터 자동 재개" 기능을 구현한다.

**Architecture:** 테스트는 가짜 path_provider와 메모리 SharedPreferences로 격리한 실제 앱을 띄운다. 위젯을 조작하고, 앱이 이미 가진 상태(재생 목록, 튜닝)와 채널별 출력 피크(VU)로 검증한다. 재개 기능은 네 부분이다.
- 오디오 스레드가 원자 슬롯에 재생 위치를 적는다.
- 감시 루프가 자기 재시작을 결정한 직후 스냅샷을 뜬다.
- 새 엔진이 뜨면 Dart가 재동기화하고 완료를 알린다.
- Rust가 커서 위치부터 재개한다. 스트리밍 트랙은 `DiskStreamer`가 시작 위치까지 디코딩해 버리고 시작한다.

**Tech Stack:** Flutter `integration_test`, Riverpod 3, flutter_rust_bridge 2.12.0, Rust(cpal 0.16, rtrb, symphonia, hound), macOS 실기(시스템 기본 출력 장치)

**Spec:** `docs/superpowers/specs/2026-10-01-app-flow-integration-test-design.md`

## Global Constraints

- DSP 3법칙: 오디오 스레드(`mixer.process`, 엔진 콜백)에는 할당·잠금·대기·`println!`이 없다. 원자 저장만 한다. 커서 표(`CURSOR_TABLE`)는 오디오 스레드가 처음 쓰기 전에 `AudioMixer::new`(오디오 스레드 밖)에서 초기화한다.
- Rust 주석은 한국어로 짧게 쓴다. 이유가 분명하지 않은 곳에만 단다.
- 테스트 config.json의 `device_name`은 `null`이다(시스템 기본 출력). 장치 이름을 하드코딩하지 않는다.
- 톤은 −30dBFS다. 소리가 실제로 난다.
- 트랙 `output_channel`은 0부터 센다(화면 CH2 = `1`). `api_play_track`이 이 값을 그대로 하드웨어 채널 번호로 쓴다.
- 크로스오버 80Hz·LR24는 바꾸지 않는다.
- `flutter_rust_bridge`는 `=2.12.0` 그대로 둔다. `rust/src/api/`를 바꾸면 `flutter_rust_bridge_codegen generate`로 바인딩을 다시 만든다.
- 결함 주입 심볼(`atmos_test_*`)은 `#[cfg(debug_assertions)]`로만 존재한다. FRB 공개 API(`rust/src/api/`)에 두지 않는다.
- 커밋은 사용자가 요청할 때만 한다. 각 작업 끝의 "체크포인트"는 변경 검토까지만 한다.
- 통합 테스트·전체 빌드 전에 확인할 것이 두 가지다.
  - 다른 Atmos 앱 인스턴스가 없어야 한다: `pgrep -fl atmos_mixer_pro.app` 결과가 비어야 한다.
  - 디스크 여유가 3GB 이상이어야 한다: `df -h /System/Volumes/Data`. 계획 작성 시점에는 2.5GB였다. 모자라면 사용자에게 공간 확보를 요청하고 캐시를 직접 지우지 않는다.
- 통합 테스트 실행 명령(앱 폴더 `atmos_mixer_pro/`에서): `flutter test integration_test/app_flow_test.dart -d macos`
- macOS 실기 전용이다. CI에 넣지 않는다.

## Review Focus

이 기능을 쓰는 사람이 가장 먼저 부딪힐 입력과 기대 동작이다. 각 항목의 테스트는 작업 12의 `test_restart_resume.rs`에 넣는다.

1. **재개 대기(최대 2초) 중 사용자가 트랙을 정지하거나 전체 정지한다.** 그 트랙은 다시 재생되지 않아야 한다.
2. **재개 대기 중 또 재시작한다(재시작 폭주).** 아직 재개하지 않은 항목을 잃지 않고 다음 재개에서 튼다.
3. **같은 효과음 트랙이 두 인스턴스로 겹쳐 재생 중이다.** 각자 자기 위치에서 재개한다.
4. **재시작 사이에 설정에서 트랙이 사라졌다.** 그 트랙만 건너뛰고 나머지는 재개한다.
5. **아무것도 재생하지 않을 때 재시작한다.** 아무것도 재생하지 않는다.

---

## 파일 구조

| 파일 | 책임 |
|---|---|
| `integration_test/app_flow_test.dart` (생성) | 시나리오 0~6단계 |
| `integration_test/support/isolation.dart` (생성) | 가짜 path_provider, 픽스처(톤 WAV·config.json·12m 방) |
| `integration_test/support/probes.dart` (생성) | 기다리기, VU·장치 이벤트 관찰, JS 노드 탭, 결함 주입 호출, 사전 확인 |
| `integration_test/simple_test.dart` (삭제) | FRB 템플릿 잔재 |
| `pubspec.yaml` (수정) | dev 의존성 2개 |
| `lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart` (수정) | 슬라이더·LFE 스위치 키 |
| `lib/core/state/engine_resync.dart` (수정) | `parseEngineRestartedSeq` |
| `lib/main.dart` (수정) | `EngineRestarted` 받으면 재동기화 후 완료 알림 |
| `rust/src/audio/playback_cursor.rs` (생성) | 재생 위치 원자 슬롯 표 |
| `rust/src/audio/player.rs` (수정) | 인스턴스 재생 위치 계산·시작 위치 |
| `rust/src/audio/streaming.rs` (수정) | 시작 위치 건너뛰기, 루프 길이 |
| `rust/src/audio/mixer.rs` (수정) | 블록마다 위치 게시, 청크 경계에서 기준 프레임 누적 |
| `rust/src/core/restart_resume.rs` (생성) | 스냅샷·대기열·취소·재동기화 대기·재개 |
| `rust/src/api/simple.rs` (수정) | 위치 조회·완료 알림 API, `play_track_from`, 스냅샷·재개 연결, 재시작 이벤트, 사용자 동작 시 대기열 취소 |
| `rust/src/test_hooks.rs` (생성) | 디버그 전용 결함 주입 C 심볼 |
| `rust/tests/test_playback_cursor_table.rs` 외 4개 (생성) | Rust 단위 검증 |

---

### Task 1: 격리 하네스와 0단계(부팅)

**Files:**
- Modify: `atmos_mixer_pro/pubspec.yaml` (dev_dependencies)
- Delete: `atmos_mixer_pro/integration_test/simple_test.dart`
- Create: `atmos_mixer_pro/integration_test/support/isolation.dart`
- Create: `atmos_mixer_pro/integration_test/support/probes.dart`
- Create: `atmos_mixer_pro/integration_test/app_flow_test.dart`

**Interfaces:**
- Produces: `Fixture` (`roomId`, `t200Loop`, `t50Loop`, `t200Long`, `t50Long`, `mainChannel`=1, `subChannel`=0, `create()`, `install()`), `pumpFor`, `pumpUntil`, `VuProbe.peakDb(ch, {window})`, `VuProbe.samplesDb(ch, from, to)`, `DeviceEventProbe.events`, `tapSpeakerNode(container, id)`, `expectNoOtherAppInstance()`, 상수 `soundDb`=−50, `silenceDb`=−90

- [ ] **Step 1: dev 의존성 추가, 템플릿 테스트 삭제**

`pubspec.yaml`의 `dev_dependencies`에서 `integration_test:` 블록 아래에 추가한다. 두 패키지는 이미 잠금 파일에 있는 전이 의존성이다.

```yaml
  integration_test:
    sdk: flutter
  path_provider_platform_interface: ^2.1.2
  plugin_platform_interface: ^2.1.8
```

```bash
cd atmos_mixer_pro && git rm -q integration_test/simple_test.dart && flutter pub get
```

- [ ] **Step 2: 격리·픽스처 작성** — `integration_test/support/isolation.dart`

```dart
// 통합 테스트 격리: 사용자의 실제 프로젝트(config.json, SharedPreferences)를 읽지도 쓰지도 않는다.
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';
import 'package:path_provider_platform_interface/path_provider_platform_interface.dart';
import 'package:plugin_platform_interface/plugin_platform_interface.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// 앱 지원 폴더 등을 테스트 임시 폴더로 돌린다.
class FakePathProvider extends PathProviderPlatform with MockPlatformInterfaceMixin {
  FakePathProvider(this.root);
  final String root;

  @override
  Future<String?> getApplicationSupportPath() async => root;
  @override
  Future<String?> getApplicationDocumentsPath() async => root;
  @override
  Future<String?> getApplicationCachePath() async => '$root/cache';
  @override
  Future<String?> getTemporaryPath() async => '$root/tmp';
  @override
  Future<String?> getLibraryPath() async => root;
  @override
  Future<String?> getDownloadsPath() async => '$root/downloads';
}

/// 톤 WAV 4개, config.json, 12m×12m 3D 방.
class Fixture {
  Fixture._(this.root, this.supportDir);
  final Directory root;
  final String supportDir;

  static const roomId = 'room_it';
  static const roomZoneId = 'room_it_zone';
  static const t200Loop = 't200_loop';
  static const t50Loop = 't50_loop';
  static const t200Long = 't200_long';
  static const t50Long = 't50_long';

  /// 화면 CH2 = 내부 채널 1(0부터 센다).
  static const mainChannel = 1;

  /// 화면 CH1 = 내부 채널 0.
  static const subChannel = 0;

  static Future<Fixture> create() async {
    final root = await Directory.systemTemp.createTemp('atmos_it_');
    final support = Directory('${root.path}/support')..createSync();
    final audio = Directory('${root.path}/audio')..createSync();
    String tone(String name, double hz, double seconds) {
      final path = '${audio.path}/$name.wav';
      writeToneWav(path, hz: hz, seconds: seconds);
      return path;
    }

    Map<String, Object?> track(String id, String path, {required bool loop}) => {
          'id': id,
          'name': id,
          'file_path': path,
          'volume': 1.0,
          'is_loop': loop,
          'is_streaming': false,
          'output_channel': mainChannel,
          'output_stereo': false,
        };

    final config = {
      'osc_port': 18000,
      // 시스템 기본 출력 장치(engine.rs start). 장치 이름을 하드코딩하지 않는다.
      'device_name': null,
      'buffer_size': 512,
      'rooms': [
        {
          'id': roomId,
          'name': 'IT Room',
          'color_hex': '#3B82F6',
          'volume': 1.0,
          'tracks': [
            track(t200Loop, tone('tone200_2s', 200, 2), loop: true),
            track(t50Loop, tone('tone50_2s', 50, 2), loop: true),
            track(t200Long, tone('tone200_20s', 200, 20), loop: false),
            track(t50Long, tone('tone50_20s', 50, 20), loop: false),
          ],
        },
      ],
    };
    File('${support.path}/config.json').writeAsStringSync(jsonEncode(config));
    return Fixture._(root, support.path);
  }

  /// 앱을 띄우기 전에 부른다. 경로·저장소를 가짜로 바꾸고 12m 방을 넣어 둔다.
  /// 방이 크면 스피커가 벽에서 멀어 200Hz에 경계면 보정이 거의 걸리지 않는다(3단계).
  void install() {
    PathProviderPlatform.instance = FakePathProvider(supportDir);
    const zone = RoomZone(
      id: roomZoneId,
      label: 'IT Zone',
      x: 0,
      y: 0,
      width: 12.0,
      height: 12.0,
      color: 0xFF0284C7,
      physicalWidth: 12.0,
      physicalHeight: 12.0,
      ceilingHeight: 4.0,
      earLevel: 1.2,
    );
    SharedPreferences.setMockInitialValues({
      'exhibition_room_zone_layout': jsonEncode([zone.toJson()]),
    });
  }
}

/// 48kHz 모노 16비트 PCM 톤. 정수 주기로 끝나는 길이라 루프 이음매에 계단이 없다.
void writeToneWav(String path,
    {required double hz, required double seconds, double dbfs = -30, int sampleRate = 48000}) {
  final n = (seconds * sampleRate).round();
  final amp = pow(10, dbfs / 20) * 32767;
  final dataLen = n * 2;
  final b = ByteData(44 + dataLen);
  void ascii(int off, String s) {
    for (var i = 0; i < s.length; i++) {
      b.setUint8(off + i, s.codeUnitAt(i));
    }
  }

  ascii(0, 'RIFF');
  b.setUint32(4, 36 + dataLen, Endian.little);
  ascii(8, 'WAVE');
  ascii(12, 'fmt ');
  b.setUint32(16, 16, Endian.little);
  b.setUint16(20, 1, Endian.little); // PCM
  b.setUint16(22, 1, Endian.little); // 모노
  b.setUint32(24, sampleRate, Endian.little);
  b.setUint32(28, sampleRate * 2, Endian.little);
  b.setUint16(32, 2, Endian.little);
  b.setUint16(34, 16, Endian.little);
  ascii(36, 'data');
  b.setUint32(40, dataLen, Endian.little);
  for (var i = 0; i < n; i++) {
    b.setInt16(44 + i * 2, (amp * sin(2 * pi * hz * i / sampleRate)).round(), Endian.little);
  }
  File(path).writeAsBytesSync(b.buffer.asUint8List());
}
```

- [ ] **Step 3: 관찰 도구 작성** — `integration_test/support/probes.dart`

```dart
// 통합 테스트 관찰 도구. 앱이 이미 가진 스트림(VU, 장치 이벤트)을 같이 듣는다.
import 'dart:convert';
import 'dart:io';
import 'dart:math';

import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/three_js_engine_provider.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

/// 소리 있음 문턱(dBFS). 톤은 −30dBFS.
const soundDb = -50.0;

/// 무음 문턱(dBFS). 재생이 없으면 출력은 디지털 무음(0)이다.
const silenceDb = -90.0;

double toDb(double lin) => lin <= 0 ? double.negativeInfinity : 20 * log(lin) / ln10;

/// 실제 시간으로 기다리며 화면을 계속 그린다.
Future<void> pumpFor(WidgetTester tester, Duration d) async {
  final end = DateTime.now().add(d);
  while (DateTime.now().isBefore(end)) {
    await Future<void>.delayed(const Duration(milliseconds: 30));
    await tester.pump();
  }
}

/// [cond]가 참이 될 때까지 기다린다. [timeout]을 넘기면 [reason]으로 실패한다.
Future<void> pumpUntil(WidgetTester tester, bool Function() cond,
    {required Duration timeout, required String reason}) async {
  final end = DateTime.now().add(timeout);
  while (!cond()) {
    if (DateTime.now().isAfter(end)) fail(reason);
    await Future<void>.delayed(const Duration(milliseconds: 30));
    await tester.pump();
  }
}

class _VuSample {
  _VuSample(this.at, this.levels);
  final DateTime at;
  final List<double> levels;
}

/// 채널별 출력 피크. VU 세션은 전역에 하나라 새로 열지 않고 앱의 vuStreamProvider를 같이 듣는다.
class VuProbe {
  VuProbe(ProviderContainer container) {
    _sub = container.listen<AsyncValue<List<double>>>(vuStreamProvider, (_, next) {
      final v = next.value;
      if (v == null) return;
      _samples.add(_VuSample(DateTime.now(), List<double>.of(v)));
      if (_samples.length > 6000) _samples.removeRange(0, 2000);
    }, fireImmediately: true);
  }

  late final ProviderSubscription<AsyncValue<List<double>>> _sub;
  final _samples = <_VuSample>[];

  /// 최근 [window] 동안 채널 [ch](0부터)의 최대 피크(dBFS).
  double peakDb(int ch, {Duration window = const Duration(milliseconds: 300)}) {
    final from = DateTime.now().subtract(window);
    var peak = 0.0;
    for (final s in _samples) {
      if (s.at.isBefore(from) || ch >= s.levels.length) continue;
      peak = max(peak, s.levels[ch]);
    }
    return toDb(peak);
  }

  /// [from]~[to] 사이 채널 [ch]의 개별 표본(dBFS).
  List<double> samplesDb(int ch, DateTime from, DateTime to) => [
        for (final s in _samples)
          if (!s.at.isBefore(from) && !s.at.isAfter(to) && ch < s.levels.length) toDb(s.levels[ch]),
      ];

  void close() => _sub.close();
}

/// 앱의 장치 이벤트(EngineReady, EngineRestarted:<순번> 등)를 같이 듣는다.
class DeviceEventProbe {
  DeviceEventProbe(ProviderContainer container) {
    _sub = container.listen<AsyncValue<String>>(deviceEventStreamProvider, (_, next) {
      final v = next.value;
      if (v != null) events.add((DateTime.now(), v));
    });
  }

  late final ProviderSubscription<AsyncValue<String>> _sub;
  final events = <(DateTime, String)>[];

  void close() => _sub.close();
}

/// 3D 장면이 노드를 탭했을 때 보내는 메시지를 JS 채널에 그대로 넣는다.
/// Three.js의 클릭 판정(레이캐스팅)은 거치지 않는다(설계 10절 "알려진 한계").
Future<void> tapSpeakerNode(ProviderContainer container, String speakerId) async {
  final engine = container.read(threeJsEngineProvider);
  final message = jsonEncode({'type': 'SPEAKER_SELECTED', 'speakerId': speakerId});
  await engine.controller!.runJavaScript('SpeakerBridge.postMessage(${jsonEncode(message)});');
}

/// 0단계 사전 확인: 이 테스트 앱 말고 다른 Atmos 앱이 떠 있으면 실패한다.
Future<void> expectNoOtherAppInstance() async {
  final r = await Process.run('pgrep', ['-f', 'atmos_mixer_pro.app/Contents/MacOS/atmos_mixer_pro']);
  final others = (r.stdout as String)
      .split('\n')
      .map((s) => s.trim())
      .where((s) => s.isNotEmpty && s != '$pid')
      .toList();
  if (others.isNotEmpty) {
    fail('0단계: 다른 Atmos 앱 인스턴스(pid ${others.join(', ')})가 실행 중이다. 종료한 뒤 다시 실행하라.');
  }
}
```

- [ ] **Step 4: 0단계 테스트 작성** — `integration_test/app_flow_test.dart`

```dart
// 앱 흐름 통합 테스트(설계: docs/superpowers/specs/2026-10-01-app-flow-integration-test-design.md).
// 실제 앱을 띄워 시스템 기본 출력 장치로 −30dBFS 톤을 낸다. macOS 실기 전용, 수동 실행:
//   flutter test integration_test/app_flow_test.dart -d macos
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/features/dashboard/screens/dashboard_screen.dart';
import 'package:atmos_mixer_pro/main.dart' as app;
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
    fixture.install();
    await app.main();
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
    addTearDown(() async {
      vu.close();
      events.close();
      await rust_api.apiStopAll();
    });

    // ── 이후 단계는 작업 2~14에서 여기 아래에 순서대로 붙인다 ──
  }, timeout: const Timeout(Duration(minutes: 8)));
}
```

- [ ] **Step 5: 실행해서 통과 확인, 격리 확인**

사용자의 실제 설정 파일 수정 시각을 테스트 전후로 비교한다.

```bash
REAL="$HOME/Library/Application Support/com.example.atmosMixerPro/config.json"; stat -f %m "$REAL" 2>/dev/null > /tmp/atmos_cfg_before; cd atmos_mixer_pro && flutter test integration_test/app_flow_test.dart -d macos; stat -f %m "$REAL" 2>/dev/null | diff - /tmp/atmos_cfg_before && echo "격리 OK: 실제 config.json 그대로"
```

Expected: `All tests passed!`, 그리고 `격리 OK`.

- [ ] **Step 6: 체크포인트** — `git status`, `git diff --stat`로 범위를 확인한다(커밋은 사용자 요청 시).

---

### Task 2: 1단계 재생/정지

**Files:**
- Modify: `atmos_mixer_pro/integration_test/app_flow_test.dart`

**Interfaces:**
- Consumes: Task 1의 `Fixture`, `pumpUntil`, `VuProbe`, `soundDb`, `silenceDb`
- Produces: 없음(기존 동작 검증)

트랙 카드는 이미 `ValueKey(track.id)`를 갖고 있다(`room_card.dart` 731행). 재생 버튼은 툴팁 '재생'/'정지'로 바뀌는 토글이다(`track_card.dart` 221~238행). 그래서 앱 코드에 키를 달 필요가 없다.

- [ ] **Step 1: 1단계 작성** — 0단계 블록 아래에 붙인다.

```dart
    // 1단계: 재생/정지(대시보드 트랙 카드의 재생 버튼)
    final card200 = find.byKey(const ValueKey(Fixture.t200Loop));
    await tester.ensureVisible(card200);
    await tester.tap(find.descendant(of: card200, matching: find.byTooltip('재생')));
    await pumpUntil(tester,
        () => container.read(engineStateProvider).playingTrackIds.contains(Fixture.t200Loop),
        timeout: const Duration(seconds: 1), reason: '1단계: 재생을 눌렀는데 1초 안에 재생 목록에 없다');
    await pumpUntil(tester, () => vu.peakDb(Fixture.mainChannel) > soundDb,
        timeout: const Duration(seconds: 1), reason: '1단계: 재생을 눌렀는데 1초 안에 CH2에서 소리가 나지 않는다');
    await tester.tap(find.descendant(of: card200, matching: find.byTooltip('정지')));
    await pumpUntil(tester,
        () => !container.read(engineStateProvider).playingTrackIds.contains(Fixture.t200Loop),
        timeout: const Duration(seconds: 1), reason: '1단계: 정지를 눌렀는데 1초 안에 재생 목록에서 빠지지 않았다');
    await pumpUntil(tester,
        () => vu.peakDb(Fixture.mainChannel, window: const Duration(milliseconds: 150)) < silenceDb,
        timeout: const Duration(seconds: 1), reason: '1단계: 정지를 눌렀는데 1초 안에 CH2가 무음이 되지 않았다');
```

- [ ] **Step 2: 실행해서 통과 확인**

Run: `flutter test integration_test/app_flow_test.dart -d macos`
Expected: PASS

- [ ] **Step 3: 테스트가 실패할 수 있는지 확인(변형)**

`lib/features/dashboard/widgets/room_card.dart`의 `onStop:` 본문 `await rust_api.apiStopTrack(...)` 호출을 잠시 주석 처리하고 실행한다. 메시지 "1단계: 정지를 눌렀는데 … 재생 목록에서 빠지지 않았다"로 실패하는지 확인한 뒤 원복한다(`git diff lib/features/dashboard/widgets/room_card.dart`가 비어야 한다).

- [ ] **Step 4: 체크포인트** — `git diff --stat`.

---

### Task 3: 2단계 스피커 배치와 인스펙터 열기

**Files:**
- Modify: `atmos_mixer_pro/integration_test/app_flow_test.dart`

**Interfaces:**
- Consumes: `tapSpeakerNode`, `pumpFor`, `pumpUntil`
- Produces: 지역 변수 `ch1Id`, `ch2Id`(3·4단계가 씀)

"Add 3D Speaker" 버튼(`dynamic_3d_room.dart` 260~291행)은 스피커를 만들면서(첫 번째 CH1, 두 번째 CH2, 둘 다 방 크기의 1/4 지점) 그 스피커의 인스펙터를 연다. 인스펙터는 `_selectedInspectorSpeakerId`가 있을 때만 그려진다(`speaker_canvas_screen.dart` 218행).

- [ ] **Step 1: import 추가** — 파일 머리에 추가한다.

```dart
import 'package:atmos_mixer_pro/features/exhibition/screens/speaker_canvas_screen.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/speaker_layout_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/three_js_engine_provider.dart';
import 'package:atmos_mixer_pro/features/exhibition/widgets/hud/speaker_inspector_panel.dart';
```

- [ ] **Step 2: 2단계 작성** — 1단계 아래에 붙인다.

```dart
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
```

- [ ] **Step 3: 실행해서 통과 확인**

Run: `flutter test integration_test/app_flow_test.dart -d macos`
Expected: PASS

- [ ] **Step 4: 변형 확인**

`lib/features/exhibition/state/three_js_engine_provider.dart` 108행 `_speakerTappedController.add(id);`를 잠시 주석 처리하고 실행한다. "2단계: CH2 노드 탭 메시지로 인스펙터가 열리지 않았다"로 실패하는지 확인한 뒤 원복한다.

- [ ] **Step 5: 체크포인트**

---

### Task 4: 3단계 스피커 이동 → FX 반영

**Files:**
- Modify: `atmos_mixer_pro/lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart` (`_buildControlBox`의 `Slider`)
- Modify: `atmos_mixer_pro/integration_test/app_flow_test.dart`

**Interfaces:**
- Consumes: `ch2Id`(2단계), `tuningStateProvider`(`Map<int, ChannelTuningState>`, 필드 `gainDb`, `delay`)
- Produces: 위젯 키 `ValueKey('inspector_slider_<라벨>')`(예: `'inspector_slider_X Position'`)

**기대값의 근거:**
- 자동 게인 기준 거리는 방 치수로 고정된다(`acoustic_sync_provider.dart` 169행, `min(가로, 세로)/2` = 6m). 그래서 CH2 게인은 CH2 거리로만 바뀐다.
- 두 스피커는 (3, 3, 1.8)에서 시작하고, 청취 지점은 (6, 6, 1.2)다. CH2를 (1.5, 1.5)로 옮기면 거리가 4.29m에서 6.39m가 된다.
- 시간 정렬 기준은 가장 먼 스피커라, CH2가 가장 멀어지면 CH2 딜레이는 0으로 남고 **CH1 딜레이가 약 6ms 늘어난다**. 그래서 딜레이는 CH1 쪽을 본다. 설계 문서의 "CH2 딜레이가 바뀐다"를 물리에 맞게 고친 것이다.

- [ ] **Step 1: 3단계 작성**

```dart
    // 3단계: 스피커 이동 → FX 반영
    await rust_api.apiPlayTrack(roomId: Fixture.roomId, trackId: Fixture.t200Loop);
    await pumpFor(tester, const Duration(seconds: 1));
    final tuningBefore = container.read(tuningStateProvider);
    final g0 = tuningBefore[Fixture.mainChannel]?.gainDb ?? 0.0;
    final subDelay0 = tuningBefore[Fixture.subChannel]?.delay ?? 0.0;
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
    final tuningAfter = container.read(tuningStateProvider);
    final g1 = tuningAfter[Fixture.mainChannel]?.gainDb ?? 0.0;
    final subDelay1 = tuningAfter[Fixture.subChannel]?.delay ?? 0.0;
    final v1 = vu.peakDb(Fixture.mainChannel);
    debugPrint('3단계: CH2 게인 $g0→$g1 dB, CH1 딜레이 $subDelay0→$subDelay1 ms, CH2 피크 $v0→$v1 dBFS');
    expect((g1 - g0).abs(), greaterThanOrEqualTo(1.0),
        reason: '3단계: CH2를 옮겼는데 CH2 튜닝 게인이 바뀌지 않았다(FX가 스피커를 안 따라감)');
    expect(subDelay1 - subDelay0, greaterThanOrEqualTo(3.0),
        reason: '3단계: CH2가 가장 멀어졌는데 CH1 시간 정렬 딜레이가 늘지 않았다');
    expect(((v1 - v0) - (g1 - g0)).abs(), lessThanOrEqualTo(1.5),
        reason: '3단계: 튜닝 게인 변화(${(g1 - g0).toStringAsFixed(1)}dB)가 CH2 출력'
            '(${(v1 - v0).toStringAsFixed(1)}dB)에 반영되지 않았다');
    await rust_api.apiStopTrack(roomId: Fixture.roomId, trackId: Fixture.t200Loop);
    await pumpUntil(tester,
        () => vu.peakDb(Fixture.mainChannel, window: const Duration(milliseconds: 150)) < silenceDb,
        timeout: const Duration(seconds: 2), reason: '3단계: 정지 후 무음이 되지 않았다');
```

- [ ] **Step 2: 실행해서 실패 확인**

Run: `flutter test integration_test/app_flow_test.dart -d macos`
Expected: FAIL. 키가 없어 `find.byKey(...)`가 비고 `ensureVisible`에서 "Bad state: No element"로 실패한다.

- [ ] **Step 3: 슬라이더 키 추가** — `speaker_inspector_panel.dart`의 `_buildControlBox` 안 `Slider(`(메서드 시작에서 약 67행)

```dart
                child: Slider(
                  key: ValueKey('inspector_slider_$label'),
                  value: value.clamp(min, max),
```

- [ ] **Step 4: 실행해서 통과 확인**

Run: `flutter test integration_test/app_flow_test.dart -d macos`
Expected: PASS. 출력의 "3단계:" 줄에서 게인 변화가 약 +3.5dB, CH1 딜레이 증가가 약 6ms인지 본다. 피크 변화가 게인 변화와 1.5dB 넘게 다르면, 원인(경계면 EQ 등)을 출력으로 확인한 뒤 목표 위치를 조정한다. 문턱을 넓히지 않는다.

- [ ] **Step 5: 변형 확인(성공 기준 9절 1행)**

`lib/main.dart` 79행 `ref.watch(acousticSyncProvider);`를 잠시 주석 처리하고 실행한다. "3단계: … 튜닝 게인이 바뀌지 않았다"로 실패하는지 확인하고 원복한다.

- [ ] **Step 6: 체크포인트**

---

### Task 5: 4단계 서브 라우팅

**Files:**
- Modify: `atmos_mixer_pro/lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart` (LFE `Switch`, 742행 부근)
- Modify: `atmos_mixer_pro/integration_test/app_flow_test.dart`

**Interfaces:**
- Consumes: `ch1Id`, `tapSpeakerNode`, `tuningStateProvider`
- Produces: 위젯 키 `ValueKey('inspector_lfe_switch')`. 4단계 끝에서 대시보드로 돌아간다(5·6단계 전제).

LFE 스위치는 기본으로 접힌 'Bass Management' 패널 안에 있다(`initiallyExpanded: false`, 716행). 그래서 제목을 탭해 펼친다.

- [ ] **Step 1: 4단계 작성**

```dart
    // 4단계: 서브 라우팅
    await tapSpeakerNode(container, ch1Id);
    await pumpUntil(tester,
        () => tester
            .widgetList<SpeakerInspectorPanel>(find.byType(SpeakerInspectorPanel))
            .any((p) => p.speakerId == ch1Id),
        timeout: const Duration(seconds: 3), reason: '4단계: CH1 노드 탭 메시지로 인스펙터가 CH1로 바뀌지 않았다');
    await tester.ensureVisible(find.text('Bass Management'));
    await tester.tap(find.text('Bass Management'));
    await pumpFor(tester, const Duration(milliseconds: 400));
    final lfeSwitch = find.byKey(const ValueKey('inspector_lfe_switch'));
    await tester.ensureVisible(lfeSwitch);
    await tester.tap(lfeSwitch);
    await pumpUntil(tester,
        () => container.read(speakerLayoutProvider).firstWhere((s) => s.id == ch1Id).isSubwoofer,
        timeout: const Duration(seconds: 2), reason: '4단계: 스위치를 켰는데 CH1이 서브로 지정되지 않았다');
    await pumpFor(tester, const Duration(seconds: 1));

    // 채널 피크를 그 채널 튜닝 게인으로 나눠 비교한다(거리 보정이 레벨을 바꾸므로).
    double normDb(int ch) =>
        vu.peakDb(ch) - (container.read(tuningStateProvider)[ch]?.gainDb ?? 0.0);

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
        () => vu.peakDb(Fixture.mainChannel, window: const Duration(milliseconds: 150)) < silenceDb,
        timeout: const Duration(seconds: 2), reason: '4단계: 정지 후 무음이 되지 않았다');

    // 대시보드로 돌아간다(5·6단계의 버튼이 대시보드에 있다).
    Navigator.of(tester.element(find.byType(SpeakerCanvasScreen))).pop();
    await pumpUntil(tester, () => find.byType(SpeakerCanvasScreen).evaluate().isEmpty,
        timeout: const Duration(seconds: 3), reason: '4단계: 대시보드로 돌아가지 못했다');
```

- [ ] **Step 2: 실행해서 실패 확인**

Expected: FAIL. `inspector_lfe_switch` 키가 없어 `ensureVisible`에서 "No element"로 실패한다.

- [ ] **Step 3: LFE 스위치 키 추가** — `speaker_inspector_panel.dart` 742행 부근

```dart
                        Switch(
                          key: const ValueKey('inspector_lfe_switch'),
                          value: isLfe,
```

- [ ] **Step 4: 실행해서 통과 확인** — 출력의 "4단계:" 두 줄이 이론값 근처(약 32dB, 약 16dB)인지 본다.

- [ ] **Step 5: 변형 확인(성공 기준 9절 2행)**

`lib/features/exhibition/models/speaker_node.dart` 292행 `'is_subwoofer': node.isSubwoofer,`를 `'is_subwoofer': false,`로 잠시 바꾸고 실행한다. "4단계: 50Hz인데 서브(CH1)가 … 크지 않다"로 실패하는지 확인하고 원복한다.

- [ ] **Step 6: 체크포인트**

---

### Task 6: 6단계 테마 시작

**Files:**
- Modify: `atmos_mixer_pro/integration_test/app_flow_test.dart`

**Interfaces:**
- Consumes: 대시보드 화면(4단계 끝), `events`(재시작 이벤트는 작업 12부터 나온다. 그 전에는 항상 0개다)

6단계 코드는 4단계 바로 아래에 붙인다. 5단계는 작업 11에서 4단계와 6단계 사이에 끼워 넣는다.

워치독 판정은 설계의 스트림 상태(`WatchdogActive`·`HotReloading`) 대신 장치 이벤트 `EngineRestarted:<순번>`으로 한다. 스트림 상태 싱크는 전역에 하나뿐이라 테스트가 구독하면 대시보드의 구독을 빼앗기 때문이다. 자기 재시작이면 작업 12 이후 반드시 이 이벤트가 나온다.

- [ ] **Step 1: 6단계 작성**

```dart
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
      final dips = vu.samplesDb(ch, watchFrom, watchTo).where((db) => db < normal[ch]! - 6.0).length;
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
        () => vu.peakDb(Fixture.mainChannel, window: const Duration(milliseconds: 150)) < silenceDb,
        timeout: const Duration(seconds: 2), reason: '6단계: Emergency를 눌렀는데 소리가 멈추지 않았다');
```

- [ ] **Step 2: 실행해서 통과 확인**

- [ ] **Step 3: 변형 확인**

`rust/src/audio/streaming.rs` 138행 `if is_loop {`를 `if false {`로 잠시 바꾸고 실행한다. 루프가 끝나 소리가 끊기면서 "6단계: 루프 재생 중 … 끊겼다"로 실패하는지 확인하고 원복한다. Rust 변경이라 앱 재빌드에 몇 분 걸린다.

- [ ] **Step 4: 체크포인트**

---

### Task 7: 재생 위치 표(Rust)와 인스턴스 위치 게시

**Files:**
- Create: `atmos_mixer_pro/rust/src/audio/playback_cursor.rs`
- Modify: `atmos_mixer_pro/rust/src/audio/mod.rs` (`pub mod playback_cursor;`)
- Modify: `atmos_mixer_pro/rust/src/audio/player.rs` (`SoundInstance` 필드 2개·메서드 2개)
- Modify: `atmos_mixer_pro/rust/src/audio/mixer.rs` (`new`, 청크 교체 지점, 인스턴스 루프 뒤)
- Test: `atmos_mixer_pro/rust/tests/test_playback_cursor_table.rs`, `atmos_mixer_pro/rust/tests/test_instance_position.rs`

**Interfaces:**
- Produces:
  - `audio::playback_cursor::{CursorTable, CURSOR_TABLE, SLOT_COUNT}`
  - `CursorTable::new(len: usize) -> CursorTable`
  - `publish(&self, idx: usize, instance_id: u64, seconds: f64)`
  - `clear(&self, idx: usize)`
  - `clear_all(&self)`
  - `snapshot(&self) -> Vec<(u64, f64)>`
  - `SoundInstance.position_base_frames: f64`
  - `SoundInstance.loop_len_frames: Option<f64>`
  - `SoundInstance::position_seconds(&self) -> f64`
  - `SoundInstance::set_start_position(&mut self, seconds: f64)`

- [ ] **Step 1: 실패하는 테스트 작성** — `rust/tests/test_playback_cursor_table.rs`

```rust
//! 재생 위치 표의 읽기·쓰기 규약: 오디오 스레드는 기다리지 않고, 읽는 쪽은 칸 주인이 바뀌는
//! 순간에 걸려도 (id, 위치) 짝을 섞지 않는다.
use rust_lib_atmos_mixer_pro::audio::playback_cursor::CursorTable;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn 쓰고_지우고_다시_쓰면_읽는_쪽에_그대로_보인다() {
    let t = CursorTable::new(8);
    t.publish(3, 77, 1.5);
    assert_eq!(t.snapshot(), vec![(77, 1.5)]);
    t.publish(3, 77, 2.0);
    assert_eq!(t.snapshot(), vec![(77, 2.0)]);
    t.clear(3);
    assert!(t.snapshot().is_empty());
    t.publish(3, 78, 0.25);
    t.publish(9, 99, 1.0); // 범위 밖 칸은 무시한다
    assert_eq!(t.snapshot(), vec![(78, 0.25)]);
    t.clear_all();
    assert!(t.snapshot().is_empty());
}

#[test]
fn 칸_주인이_바뀌는_중에도_id와_위치_짝이_섞이지_않는다() {
    let t = Arc::new(CursorTable::new(1));
    let stop = Arc::new(AtomicBool::new(false));
    let writer = {
        let (t, stop) = (t.clone(), stop.clone());
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                t.publish(0, 1, 1.0);
                t.publish(0, 2, 2.0);
            }
        })
    };
    let deadline = Instant::now() + Duration::from_millis(300);
    let mut seen = 0usize;
    while Instant::now() < deadline {
        for (id, s) in t.snapshot() {
            seen += 1;
            assert!((id == 1 && s == 1.0) || (id == 2 && s == 2.0), "짝이 섞였다: id {id}, 위치 {s}");
        }
    }
    stop.store(true, Ordering::Relaxed);
    writer.join().unwrap();
    assert!(seen > 0, "한 번도 읽지 못했다");
}
```

`rust/tests/test_instance_position.rs`:

```rust
//! 재생 위치(초)가 재시작 복원과 위치 조회가 쓰는 표에 정확히 올라가는지 본다.
//! 전역 표를 쓰므로 이 파일에는 테스트를 하나만 둔다.
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::playback_cursor::CURSOR_TABLE;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const FS: u32 = 48_000;
const CH: usize = 4;
const BLOCK: usize = 480; // 10ms

fn instance(id: u64, seconds_of_data: f32, is_loop: bool) -> SoundInstance {
    let n = (FS as f32 * seconds_of_data) as usize;
    let data = Arc::new(SoundData { samples: vec![0.1; n], channels: 1, sample_rate: FS });
    let mut inst = SoundInstance::new(
        id, 1, 1, "t".into(), Some(data), None, FS, 1, is_loop, 1.0, 1, false, None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.fade_weight = 1.0;
    inst
}

#[test]
fn 재생_위치가_표에_오르고_루프는_한_바퀴_안으로_접히고_끝나면_지워진다() {
    // 계산: 시작 위치 지정과 루프 접기
    let mut a = instance(11, 2.0, false);
    a.set_start_position(1.25);
    assert!((a.position_seconds() - 1.25).abs() < 1e-9);
    let mut l = instance(12, 2.0, true);
    l.cursor = FS as f64 * 2.5; // 한 바퀴(2초)를 넘김
    assert!((l.position_seconds() - 0.5).abs() < 1e-9, "루프 위치가 접히지 않았다: {}", l.position_seconds());

    // 믹서가 블록마다 위치를 표에 적는다
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    mixer.startup_ramp.current_gain = 1.0;
    let mut inst = instance(21, 3.0, false);
    inst.set_start_position(1.0);
    mixer.instances[5] = Some(inst);
    let mut buf = vec![0.0f32; CH * BLOCK];
    for _ in 0..50 {
        buf.fill(0.0);
        mixer.process(&mut buf, CH);
    } // 0.5초
    let pos = CURSOR_TABLE
        .snapshot()
        .into_iter()
        .find(|(id, _)| *id == 21)
        .map(|(_, s)| s)
        .expect("표에 위치가 없다");
    assert!((pos - 1.5).abs() < 0.02, "위치 {pos} (기대 1.5초)");

    // 끝난 인스턴스는 표에서 빠진다(남은 데이터 2초 + 페이드 0.3초)
    for _ in 0..300 {
        buf.fill(0.0);
        mixer.process(&mut buf, CH);
    }
    assert!(CURSOR_TABLE.snapshot().iter().all(|(id, _)| *id != 21), "끝난 인스턴스가 표에 남았다");
}
```

- [ ] **Step 2: 실행해서 실패 확인**

Run: `cd atmos_mixer_pro/rust && cargo test --test test_playback_cursor_table --test test_instance_position`
Expected: 컴파일 오류(`playback_cursor` 모듈, `set_start_position`, `position_seconds` 없음)

- [ ] **Step 3: 표 구현** — `rust/src/audio/playback_cursor.rs`

```rust
//! 재생 중인 인스턴스의 재생 위치를 오디오 스레드 밖으로 알리는 고정 크기 표.
//!
//! 오디오 스레드는 블록마다 원자 저장만 한다(할당·잠금 없음). 칸은 믹서 인스턴스 풀의 칸과 1:1이다.
//! 엔진 밖에 있어서 엔진이 재시작으로 사라져도 남는다 — 재시작 직전 위치를 여기서 읽는다
//! (core::restart_resume).
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};

/// 믹서 인스턴스 풀 크기(mixer.rs `instances`)와 같다.
pub const SLOT_COUNT: usize = 4096;

struct Slot {
    /// 0이면 빈 칸.
    instance_id: AtomicU64,
    seconds_bits: AtomicU64,
}

pub struct CursorTable {
    slots: Vec<Slot>,
}

impl CursorTable {
    pub fn new(len: usize) -> Self {
        Self {
            slots: (0..len)
                .map(|_| Slot { instance_id: AtomicU64::new(0), seconds_bits: AtomicU64::new(0) })
                .collect(),
        }
    }

    /// 오디오 스레드: 칸 `idx`에 있는 인스턴스의 위치를 적는다. 칸 주인이 바뀌면 id를 먼저
    /// 비우고 위치를 쓴 뒤 새 id를 적는다 — 읽는 쪽이 옛 주인의 위치를 새 주인과 짝짓지 않게.
    pub fn publish(&self, idx: usize, instance_id: u64, seconds: f64) {
        let Some(slot) = self.slots.get(idx) else { return };
        if slot.instance_id.load(Ordering::Relaxed) != instance_id {
            slot.instance_id.store(0, Ordering::Release);
            slot.seconds_bits.store(seconds.to_bits(), Ordering::Release);
            slot.instance_id.store(instance_id, Ordering::Release);
        } else {
            slot.seconds_bits.store(seconds.to_bits(), Ordering::Release);
        }
    }

    /// 오디오 스레드: 끝난 인스턴스의 칸을 비운다.
    pub fn clear(&self, idx: usize) {
        if let Some(slot) = self.slots.get(idx) {
            slot.instance_id.store(0, Ordering::Release);
        }
    }

    /// 오디오 스레드 밖에서 부른다(새 믹서 생성, 재시작 스냅샷).
    pub fn clear_all(&self) {
        for slot in &self.slots {
            slot.instance_id.store(0, Ordering::Release);
        }
    }

    /// 오디오 스레드 밖에서 읽는다. 읽는 사이 칸 주인이 바뀌면 그 칸은 건너뛴다.
    /// 다시 읽는 건 읽는 쪽뿐이고 오디오 스레드는 기다리지 않는다.
    pub fn snapshot(&self) -> Vec<(u64, f64)> {
        let mut out = Vec::new();
        for slot in &self.slots {
            let id1 = slot.instance_id.load(Ordering::Acquire);
            if id1 == 0 {
                continue;
            }
            let seconds = f64::from_bits(slot.seconds_bits.load(Ordering::Acquire));
            if slot.instance_id.load(Ordering::Acquire) == id1 {
                out.push((id1, seconds));
            }
        }
        out
    }
}

/// 엔진이 쓰는 표. 오디오 스레드가 처음 쓰기 전에 `AudioMixer::new`에서 초기화된다(할당이 거기서 일어난다).
pub static CURSOR_TABLE: Lazy<CursorTable> = Lazy::new(|| CursorTable::new(SLOT_COUNT));
```

`rust/src/audio/mod.rs` 끝에 `pub mod playback_cursor;`를 추가한다.

- [ ] **Step 4: 인스턴스 위치** — `rust/src/audio/player.rs`

구조체 `SoundInstance` 필드 끝(`spatial_gains_target` 아래)에 추가한다.

```rust
    /// 스트리밍: 지금 `stream_buffer` 앞까지 지나간 프레임 수(시작 위치 포함). 미리 로드한 데이터는 0.
    pub position_base_frames: f64,
    /// 루프 한 바퀴 길이(프레임). 모르면 None이라 위치를 바퀴 안으로 접지 못한다.
    pub loop_len_frames: Option<f64>,
```

`SoundInstance::new` 본문 첫 줄(`let mut smoother = ...` 앞)에 추가한다.

```rust
        let loop_len_frames = if is_loop {
            streamer.as_ref().and_then(|s| s.loop_len_frames).or_else(|| {
                data.as_ref().map(|d| (d.samples.len() / d.channels.max(1) as usize) as f64)
            })
        } else {
            None
        };
```

`Self { ... }` 초기화 끝에 `position_base_frames: 0.0, loop_len_frames,`를 넣는다. `impl SoundInstance` 블록 끝에 추가한다.

```rust
    /// 파일 기준 재생 위치(초). 루프면 한 바퀴 안의 위치다.
    pub fn position_seconds(&self) -> f64 {
        let mut frames = self.position_base_frames + self.cursor;
        if let Some(len) = self.loop_len_frames {
            if len > 0.0 {
                frames %= len;
            }
        }
        frames / self.stream_sample_rate.max(1) as f64
    }

    /// 파일의 `seconds` 지점부터 시작하게 한다(재시작 복원). 스트리밍이면
    /// `DiskStreamer::new_at`이 그만큼 건너뛰고 시작해야 위치가 맞는다.
    pub fn set_start_position(&mut self, seconds: f64) {
        let frames = seconds.max(0.0) * self.stream_sample_rate as f64;
        if self.stream_receiver.is_some() {
            self.position_base_frames = frames;
        } else {
            self.cursor = frames;
        }
    }
```

이 시점에 `DiskStreamer`에는 아직 `loop_len_frames`가 없다. 작업 8 전까지 컴파일되도록 `streaming.rs`의 `DiskStreamer` 구조체에 `pub loop_len_frames: Option<f64>,`를 추가하고, `Default`와 `new`의 `Ok(Self { ... })`에 `loop_len_frames: None,`를 넣는다. 실제 값은 작업 8에서 채운다.

- [ ] **Step 5: 믹서 게시** — `rust/src/audio/mixer.rs`

1. `AudioMixer::new`의 반환 직전(`for f in mixer.lfe_track_lpfs.iter_mut() {...}` 다음, `mixer` 반환 전)에 추가한다.

```rust
        // 새 엔진의 믹서다. 옛 엔진이 남긴 위치를 지우고, 표를 여기(오디오 스레드 밖)서 초기화해 둔다.
        crate::audio::playback_cursor::CURSOR_TABLE.clear_all();
```

2. 스트림 청크를 바꾸는 곳(`let frames_in_chunk = (instance.stream_buffer.len() / channels) as f64;` 바로 다음 줄)에 추가한다.

```rust
                                instance.position_base_frames += frames_in_chunk;
```

3. 프레임 루프가 끝난 뒤 `self.temp_vals = temp_vals;` 바로 앞에 추가한다.

```rust
        // 재생 위치를 표에 알린다(재시작 복원·위치 조회). 원자 저장뿐이다.
        let cursors = &*crate::audio::playback_cursor::CURSOR_TABLE;
        for &i in active_instances.iter() {
            match &self.instances[i] {
                Some(inst) if inst.is_playing => cursors.publish(i, inst.instance_id, inst.position_seconds()),
                _ => cursors.clear(i),
            }
        }
```

- [ ] **Step 6: 실행해서 통과 확인**

Run: `cd atmos_mixer_pro/rust && cargo test --test test_playback_cursor_table --test test_instance_position`
Expected: 3개 PASS

- [ ] **Step 7: 회귀 확인** — `cargo test --test test_bass_management_routing --test test_stale_engine_generation --test test_multichannel_passthrough`가 통과해야 한다.

- [ ] **Step 8: 체크포인트**

---

### Task 8: DiskStreamer 시작 위치와 루프 길이

**Files:**
- Modify: `atmos_mixer_pro/rust/src/audio/streaming.rs`
- Test: `atmos_mixer_pro/rust/tests/test_disk_streamer_start_offset.rs`

**Interfaces:**
- Produces:
  - `DiskStreamer::new_at(file_path: String, is_loop: bool, target_sample_rate: u32, start_seconds: f64) -> anyhow::Result<DiskStreamer>`
  - `DiskStreamer.loop_len_frames: Option<f64>`(출력 프레임 기준)
  - `new`는 `new_at(.., 0.0)`과 같다.

- [ ] **Step 1: 실패하는 테스트 작성** — `rust/tests/test_disk_streamer_start_offset.rs`

```rust
//! 재시작 복원은 스트리밍 트랙을 멈춘 지점부터 다시 튼다: DiskStreamer가 시작 위치까지 정확히 건너뛴다.
use rust_lib_atmos_mixer_pro::audio::streaming::DiskStreamer;
use std::time::{Duration, Instant};

const FS: u32 = 48_000;

/// i번째 샘플 = i × 1e-6 인 32비트 float 모노 WAV(값으로 위치를 알 수 있다).
fn ramp_wav(path: &std::path::Path, frames: usize) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: FS,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..frames {
        w.write_sample(i as f32 * 1e-6).unwrap();
    }
    w.finalize().unwrap();
}

fn first_sample(s: &mut DiskStreamer) -> f32 {
    let rx = s.chunk_receiver.as_mut().expect("수신기 없음");
    let t0 = Instant::now();
    loop {
        if let Ok(chunk) = rx.pop() {
            if let Some(&v) = chunk.first() {
                return v;
            }
        }
        assert!(t0.elapsed() < Duration::from_secs(3), "3초 안에 청크가 오지 않았다");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn 시작_위치만큼_건너뛰고_루프_길이를_알려준다() {
    let dir = std::env::temp_dir().join(format!("atmos_stream_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ramp.wav");
    let frames = FS as usize * 3;
    ramp_wav(&path, frames);
    let p = path.to_string_lossy().to_string();

    let mut s0 = DiskStreamer::new(p.clone(), true, FS).unwrap();
    assert_eq!(first_sample(&mut s0), 0.0);
    assert_eq!(s0.loop_len_frames, Some(frames as f64));

    let mut s1 = DiskStreamer::new_at(p.clone(), true, FS, 1.25).unwrap();
    assert_eq!(first_sample(&mut s1), 60_000f32 * 1e-6, "1.25초(60000프레임) 지점부터 시작하지 않았다");

    // 청크 여러 개를 건너뛰는 위치도 정확해야 한다
    let mut s2 = DiskStreamer::new_at(p, false, FS, 2.999).unwrap();
    let start2 = (2.999 * FS as f64).round() as usize;
    assert_eq!(first_sample(&mut s2), start2 as f32 * 1e-6);
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: 실행해서 실패 확인**

Run: `cargo test --test test_disk_streamer_start_offset`
Expected: 컴파일 오류(`new_at` 없음). 작업 7의 임시 `None` 때문에 `loop_len_frames` 비교도 이후 실패한다.

- [ ] **Step 3: 구현** — `rust/src/audio/streaming.rs`

1. `pub fn new(...)`의 본문을 `new_at`로 옮기고, `new`는 위임만 하게 한다.

```rust
    pub fn new(file_path: String, is_loop: bool, target_sample_rate: u32) -> anyhow::Result<Self> {
        Self::new_at(file_path, is_loop, target_sample_rate, 0.0)
    }

    /// `start_seconds`(파일 기준)부터 내보낸다. 재시작 복원이 멈춘 위치를 넘긴다.
    pub fn new_at(
        file_path: String,
        is_loop: bool,
        target_sample_rate: u32,
        start_seconds: f64,
    ) -> anyhow::Result<Self> {
        // (기존 new 본문)
```

2. `let output_sample_rate = ...;` 바로 다음에 추가한다. `track`은 그 위에서 얻은 기본 트랙이다.

```rust
        // 이 위치까지는 디코딩만 하고 버린다. 믹서가 세는 위치와 같게 출력(리샘플링 뒤) 프레임으로 센다.
        let mut skip_samples =
            (start_seconds.max(0.0) * output_sample_rate as f64).round() as usize * channels as usize;
        let loop_len_frames = track
            .codec_params
            .n_frames
            .map(|n| n as f64 * output_sample_rate as f64 / src_sample_rate as f64);
```

3. 파일 끝(모듈 수준)에 함수를 추가한다.

```rust
/// 시작 위치까지의 샘플은 버리고 나머지를 링버퍼로 보낸다. 링이 차 있으면 자리가 날 때까지 기다린다(디코더 스레드).
fn send_chunk(
    tx: &mut rtrb::Producer<Vec<f32>>,
    run_flag: &CachePadded<AtomicBool>,
    skip_samples: &mut usize,
    mut chunk: Vec<f32>,
) {
    if *skip_samples > 0 {
        if chunk.len() <= *skip_samples {
            *skip_samples -= chunk.len();
            return;
        }
        chunk.drain(..*skip_samples);
        *skip_samples = 0;
    }
    let mut item = chunk;
    loop {
        if !run_flag.value.load(Ordering::Relaxed) {
            break;
        }
        match tx.push(item) {
            Ok(_) => break,
            Err(rtrb::PushError::Full(returned)) => {
                std::thread::sleep(std::time::Duration::from_millis(1));
                item = returned;
            }
        }
    }
}
```

4. 디코더 스레드 안의 두 "Send chunk" 루프를 `send_chunk` 호출로 바꾼다.
   - 리샘플링 경로(`let mut chunk = Vec::with_capacity(out_frames * channels as usize);` 채운 다음): 기존 `let mut item = chunk; loop { ... }`를 `send_chunk(&mut tx, &run_flag, &mut skip_samples, chunk);`로 바꾼다.
   - 직접 경로(`chunk.extend_from_slice(buf.samples());` 다음): 같은 한 줄로 바꾼다.

5. 마지막 `Ok(Self { ... })`에 작업 7의 `loop_len_frames: None`을 `loop_len_frames,`로 바꾼다.

- [ ] **Step 4: 실행해서 통과 확인**

Run: `cargo test --test test_disk_streamer_start_offset --test test_instance_position`
Expected: PASS

- [ ] **Step 5: 체크포인트**

---

### Task 9: 재생 위치 조회 API

**Files:**
- Modify: `atmos_mixer_pro/rust/src/api/simple.rs` (구조체·함수 추가)
- Regenerate: `atmos_mixer_pro/lib/src/rust/**`, `atmos_mixer_pro/rust/src/frb_generated.rs`
- Test: `atmos_mixer_pro/rust/tests/test_playback_positions_api.rs`

**Interfaces:**
- Consumes: `CURSOR_TABLE.snapshot()`, `GLOBAL_STATE.playing_track_ids`
- Produces:
  - Rust: `pub struct PlaybackPosition { pub track_id: String, pub seconds: f64 }`, `pub fn api_get_playback_positions() -> Vec<PlaybackPosition>`
  - Dart: `Future<List<PlaybackPosition>> apiGetPlaybackPositions()`, 필드 `trackId`, `seconds`

- [ ] **Step 1: 실패하는 테스트 작성** — `rust/tests/test_playback_positions_api.rs`

```rust
//! 재생 위치 조회는 재생 목록에 있는 인스턴스의 위치만 트랙 이름으로 돌려준다(엔진이 버린 옛 칸은 무시).
//! 전역 상태를 쓰므로 테스트는 하나다.
use rust_lib_atmos_mixer_pro::api::simple::api_get_playback_positions;
use rust_lib_atmos_mixer_pro::audio::playback_cursor::CURSOR_TABLE;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;

#[test]
fn 재생_목록에_있는_인스턴스만_트랙별_위치로_돌려준다() {
    GLOBAL_STATE.clear_playing_tracks();
    CURSOR_TABLE.clear_all();
    GLOBAL_STATE.add_playing_track(1, "a".into());
    GLOBAL_STATE.add_playing_track(2, "b".into());
    CURSOR_TABLE.publish(0, 1, 4.5);
    CURSOR_TABLE.publish(1, 2, 0.75);
    CURSOR_TABLE.publish(2, 3, 9.0); // 재생 목록에 없는 옛 칸
    let got: Vec<(String, f64)> = api_get_playback_positions()
        .into_iter()
        .map(|p| (p.track_id, p.seconds))
        .collect();
    assert_eq!(got, vec![("a".to_string(), 4.5), ("b".to_string(), 0.75)]);
}
```

- [ ] **Step 2: 실행해서 실패 확인** — `cargo test --test test_playback_positions_api` → 컴파일 오류

- [ ] **Step 3: 구현** — `simple.rs`의 `api_is_engine_ready` 위에 추가한다.

```rust
/// 재생 중인 트랙의 파일 기준 재생 위치. 루프는 한 바퀴 안의 위치다.
#[derive(Debug, Clone)]
pub struct PlaybackPosition {
    pub track_id: String,
    pub seconds: f64,
}

/// 재생 중인 트랙별 재생 위치(초). 오디오 스레드가 원자 칸에 적어 둔 값을 읽기만 한다.
pub fn api_get_playback_positions() -> Vec<PlaybackPosition> {
    let playing = GLOBAL_STATE
        .playing_track_ids
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let mut out: Vec<PlaybackPosition> = crate::audio::playback_cursor::CURSOR_TABLE
        .snapshot()
        .into_iter()
        .filter_map(|(instance_id, seconds)| {
            playing
                .get(&instance_id)
                .map(|track_id| PlaybackPosition { track_id: track_id.clone(), seconds })
        })
        .collect();
    out.sort_by(|a, b| a.track_id.cmp(&b.track_id).then(a.seconds.total_cmp(&b.seconds)));
    out
}
```

- [ ] **Step 4: 통과 확인** — `cargo test --test test_playback_positions_api` → PASS

- [ ] **Step 5: 바인딩 생성**

```bash
cd atmos_mixer_pro && flutter_rust_bridge_codegen generate && grep -n "apiGetPlaybackPositions\|class PlaybackPosition" lib/src/rust/api/simple.dart && (cd rust && cargo check)
```

Expected: 두 이름이 보이고 `cargo check`가 오류 없이 끝난다.

- [ ] **Step 6: 체크포인트**

---

### Task 10: 디버그 전용 결함 주입 훅

**Files:**
- Create: `atmos_mixer_pro/rust/src/test_hooks.rs`
- Modify: `atmos_mixer_pro/rust/src/lib.rs`
- Modify: `atmos_mixer_pro/integration_test/support/probes.dart`

**Interfaces:**
- Produces:
  - C 심볼 `atmos_test_request_engine_recovery()`(디버그 빌드만)
  - Dart `void requestEngineRecovery()`

- [ ] **Step 1: 구현** — `rust/src/test_hooks.rs`

```rust
//! 통합 테스트 전용 결함 주입. 장치 오류 경로가 세우는 신호를 세워 워치독과 같은 자기 재시작 경로를 탄다.
//! FRB 공개 API(`api/`)가 아니고 디버그 빌드에만 있다(lib.rs의 `#[cfg(debug_assertions)]`).
//! cargokit은 Flutter profile·release 빌드를 `--release`로 빌드하므로 그 바이너리에는 이 심볼이 없다.
use std::sync::atomic::Ordering;

#[no_mangle]
pub extern "C" fn atmos_test_request_engine_recovery() {
    crate::core::state::GLOBAL_STATE
        .device_needs_reset
        .store(true, Ordering::Release);
}
```

`rust/src/lib.rs` 끝에 추가한다.

```rust
#[cfg(debug_assertions)]
pub mod test_hooks;
```

- [ ] **Step 2: Dart 호출부** — `integration_test/support/probes.dart`에 `import 'dart:ffi';`와 함수를 추가한다.

```dart
/// 디버그 빌드에만 있는 결함 주입 훅(rust/src/test_hooks.rs). 워치독과 같은 자기 재시작을 일으킨다.
/// FRB와 같은 Rust 프레임워크에서 심볼을 찾는다.
void requestEngineRecovery() {
  final lib = DynamicLibrary.open('rust_lib_atmos_mixer_pro.framework/rust_lib_atmos_mixer_pro');
  lib.lookupFunction<Void Function(), void Function()>('atmos_test_request_engine_recovery')();
}
```

- [ ] **Step 3: 컴파일 확인** — `cd atmos_mixer_pro/rust && cargo check && cargo check --release`. 둘 다 통과해야 한다. 릴리스에서는 모듈이 빠진다.

- [ ] **Step 4: 체크포인트**

---

### Task 11: 5단계 테스트를 먼저 쓴다(실패 확인)

**Files:**
- Modify: `atmos_mixer_pro/integration_test/app_flow_test.dart` (4단계와 6단계 사이)

**Interfaces:**
- Consumes:
  - `rust_api.apiGetPlaybackPositions()`(작업 9)
  - `requestEngineRecovery()`(작업 10)
  - `events`(`EngineRestarted:<순번>`, 작업 12)
  - `vu`

- [ ] **Step 1: 5단계 작성** — 4단계 끝(대시보드 복귀) 다음, 6단계 앞에 넣는다.

```dart
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
    requestEngineRecovery();
    await pumpUntil(tester,
        () => events.events.skip(eventsBefore).any((e) => e.$2.startsWith('EngineRestarted:')),
        timeout: const Duration(seconds: 10), reason: '5단계: 결함 주입 뒤 10초 안에 엔진 재시작 이벤트가 오지 않았다');
    final restartedAt =
        events.events.skip(eventsBefore).firstWhere((e) => e.$2.startsWith('EngineRestarted:')).$1;
    await pumpUntil(tester,
        () => vu.peakDb(Fixture.mainChannel, window: const Duration(milliseconds: 100)) > soundDb,
        timeout: const Duration(seconds: 5), reason: '5단계: 재시작 뒤 5초 안에 재생이 다시 이어지지 않았다');
    final resumedAt = DateTime.now();
    // 재시작 이벤트 뒤부터 재개 전까지 디지털 무음(옛 DSP 소리·클릭 없음). 재개 페이드 인 표본은 뺀다.
    for (final ch in [Fixture.subChannel, Fixture.mainChannel]) {
      final loud = vu
          .samplesDb(ch, restartedAt, resumedAt.subtract(const Duration(milliseconds: 150)))
          .where((db) => db > silenceDb)
          .length;
      expect(loud, 0, reason: '5단계: 재시작 이벤트 후 재개 전 CH${ch + 1}에서 소리가 났다($loud개 표본)');
    }
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
    for (final ch in [Fixture.subChannel, Fixture.mainChannel]) {
      final post = vu.peakDb(ch, window: const Duration(milliseconds: 500));
      expect((post - pre[ch]!).abs(), lessThanOrEqualTo(1.5),
          reason: '5단계: 재시작 뒤 CH${ch + 1} 레벨이 ${pre[ch]!.toStringAsFixed(1)}→'
              '${post.toStringAsFixed(1)}dBFS로 바뀌었다(서브 라우팅·FX 복원 실패)');
    }
    await rust_api.apiStopAll();
    await pumpUntil(tester,
        () => vu.peakDb(Fixture.mainChannel, window: const Duration(milliseconds: 150)) < silenceDb,
        timeout: const Duration(seconds: 2), reason: '5단계: 전체 정지 후 무음이 되지 않았다');
```

- [ ] **Step 2: 실행해서 실패 확인**

Run: `flutter test integration_test/app_flow_test.dart -d macos`
Expected: FAIL, "5단계: 결함 주입 뒤 10초 안에 엔진 재시작 이벤트가 오지 않았다". 재시작 이벤트와 재개가 아직 없다는, 의도한 실패다. 터미널 로그에 "ASIO 장치 핫리로드 요청 수신"(결함 주입이 감시 루프에 닿음)이 찍히는지도 본다. 안 찍히면 심볼 조회가 실패한 것이다.

- [ ] **Step 3: 체크포인트**

---

### Task 12: 재시작 스냅샷·재개(Rust)

**Files:**
- Create: `atmos_mixer_pro/rust/src/core/restart_resume.rs`
- Modify: `atmos_mixer_pro/rust/src/core/mod.rs` (`pub mod restart_resume;`)
- Modify: `atmos_mixer_pro/rust/src/api/simple.rs`
- Regenerate: FRB 바인딩(`api_ack_engine_restart`)
- Test: `atmos_mixer_pro/rust/tests/test_restart_resume.rs`

**Interfaces:**
- Consumes:
  - `CURSOR_TABLE`
  - `SoundInstance::set_start_position`, `SoundInstance::position_seconds`
  - `DiskStreamer::new_at`
  - `GLOBAL_STATE`
- Produces:
  - `core::restart_resume::{ResumeEntry, RESTART_SEQ, RESYNC_ACK, RESYNC_TIMEOUT}`
  - `take_snapshot()`, `cancel_all()`, `cancel_track(&str)`, `pending_entries() -> Vec<ResumeEntry>`
  - `ack(u32)`, `wait_for_ack(u32, Duration) -> bool`, `resume_pending()`, `resume_after_resync()`
  - `api::simple::play_track_from(String, String, f64) -> Result<(), AtmosError>`(`pub(crate)`)
  - `api_ack_engine_restart(seq: u32)`
  - 장치 이벤트 `"EngineRestarted:<순번>"`

- [ ] **Step 1: 실패하는 테스트 작성** — `rust/tests/test_restart_resume.rs`. Review Focus 1~5를 포함한다.

```rust
//! 재시작 복원(core::restart_resume): 스냅샷·취소·재개. 전역 상태를 쓰므로 테스트는 하나다.
use rust_lib_atmos_mixer_pro::audio::playback_cursor::CURSOR_TABLE;
use rust_lib_atmos_mixer_pro::audio::player::SoundData;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::restart_resume::{self, ResumeEntry};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn track(id: &str, path: &str) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: path.into(),
        volume: 1.0,
        is_loop: false,
        is_streaming: false,
        output_channel: 1,
        output_stereo: false,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
    }
}

fn drain() -> Vec<AudioCommand> {
    let mut v = Vec::new();
    while let Ok(c) = GLOBAL_STATE.command_receiver.try_recv() {
        v.push(c);
    }
    v
}

/// 재생 명령들의 (트랙, 시작 위치 초)
fn plays(cmds: &[AudioCommand]) -> Vec<(String, f64)> {
    cmds.iter()
        .filter_map(|c| match c {
            AudioCommand::PlayTrack { instance, .. } => {
                Some((instance.track_id_str.clone(), instance.position_seconds()))
            }
            _ => None,
        })
        .collect()
}

fn entry(track: &str, seconds: f64) -> ResumeEntry {
    ResumeEntry { room_id: "r1".into(), track_id: track.into(), seconds }
}

#[test]
fn 스냅샷을_떠서_멈춘_위치부터_재개하고_사용자_정지와_재시작_폭주를_처리한다() {
    let fs = 48_000u32;
    let data = Arc::new(SoundData { samples: vec![0.1; fs as usize * 20], channels: 1, sample_rate: fs });
    GLOBAL_STATE.preloaded_sounds.write().unwrap().insert("/sfx.wav".into(), data.clone());
    GLOBAL_STATE.preloaded_sounds.write().unwrap().insert("/bgm2.wav".into(), data);
    let mut config = AppConfig::default();
    config.rooms.push(RoomConfig {
        id: "r1".into(),
        name: "R1".into(),
        color_hex: "#ffffff".into(),
        volume: 1.0,
        volume_osc_address: String::new(),
        clear_osc_address: String::new(),
        tracks: vec![track("sfx", "/sfx.wav"), track("bgm2", "/bgm2.wav")],
    });
    *GLOBAL_STATE.config.write().unwrap() = Some(config);
    GLOBAL_STATE.clear_playing_tracks();
    CURSOR_TABLE.clear_all();
    drain();

    // 재생 중: sfx 두 인스턴스(3.0초, 7.5초), bgm2(2.0초), 설정에 없는 트랙
    for (inst, id, secs, slot) in [(101u64, "sfx", 3.0, 0usize), (102, "sfx", 7.5, 1), (103, "bgm2", 2.0, 2), (104, "gone", 1.0, 3)] {
        GLOBAL_STATE.add_playing_track(inst, id.into());
        CURSOR_TABLE.publish(slot, inst, secs);
    }
    GLOBAL_STATE.command_sender.send(AudioCommand::StopAll).unwrap(); // 옛 엔진용 미처리 명령

    // 1) 스냅샷: 같은 트랙 두 인스턴스는 각자 위치로, 설정에서 사라진 트랙은 빠진다(Focus 3·4)
    restart_resume::take_snapshot();
    assert_eq!(restart_resume::pending_entries(), vec![entry("bgm2", 2.0), entry("sfx", 3.0), entry("sfx", 7.5)]);
    assert!(GLOBAL_STATE.playing_track_ids.read().unwrap().is_empty(), "옛 인스턴스가 재생 목록에 남았다");
    assert!(CURSOR_TABLE.snapshot().is_empty(), "옛 위치가 표에 남았다");
    assert!(drain().is_empty(), "옛 엔진용 명령이 큐에 남았다");

    // 2) 재개 전에 또 재시작(재생 중인 것 없음): 대기열을 잃지 않는다(Focus 2)
    restart_resume::take_snapshot();
    assert_eq!(restart_resume::pending_entries().len(), 3);

    // 3) 기다리는 사이 사용자가 sfx를 정지하면 sfx는 재개하지 않는다(Focus 1)
    restart_resume::cancel_track("sfx");
    restart_resume::resume_pending();
    let started = plays(&drain());
    assert_eq!(started.len(), 1, "재개된 재생: {started:?}");
    assert_eq!(started[0].0, "bgm2");
    assert!((started[0].1 - 2.0).abs() < 1e-6, "재개 위치 {}", started[0].1);
    assert!(restart_resume::pending_entries().is_empty());
    assert!(GLOBAL_STATE.playing_track_ids.read().unwrap().values().any(|t| t == "bgm2"), "재개한 트랙이 재생 목록에 없다");

    // 4) 아무것도 재생 중이 아닐 때 재시작해도 아무것도 틀지 않는다(Focus 5)
    GLOBAL_STATE.clear_playing_tracks();
    CURSOR_TABLE.clear_all();
    restart_resume::take_snapshot();
    restart_resume::resume_pending();
    assert!(plays(&drain()).is_empty());

    // 5) 전체 정지는 대기열을 비운다(Focus 1)
    GLOBAL_STATE.add_playing_track(201, "bgm2".into());
    restart_resume::take_snapshot();
    restart_resume::cancel_all();
    assert!(restart_resume::pending_entries().is_empty());

    // 6) 재동기화 완료 신호: 오면 바로, 안 오면 시간 초과
    let seq = restart_resume::RESTART_SEQ.load(std::sync::atomic::Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        restart_resume::ack(seq);
    });
    let t0 = Instant::now();
    assert!(restart_resume::wait_for_ack(seq, Duration::from_secs(2)));
    assert!(t0.elapsed() < Duration::from_millis(500));
    assert!(!restart_resume::wait_for_ack(seq + 1, Duration::from_millis(100)));
}
```

- [ ] **Step 2: 실행해서 실패 확인** — `cargo test --test test_restart_resume` → 컴파일 오류(`restart_resume` 없음)

- [ ] **Step 3: 모듈 구현** — `rust/src/core/restart_resume.rs`

```rust
//! 엔진이 스스로 재시작할 때(워치독 콜백 공백, 장치 오류·재설정, 장치 목록 변화) 재생 중이던
//! 트랙을 새 엔진에서 멈춘 위치부터 다시 튼다.
//!
//! 순서는 다음과 같다.
//! 1. 감시 루프가 재시작을 결정한 직후 `take_snapshot`을 부른다.
//! 2. 옛 엔진을 drop하고 새 엔진을 띄운다.
//! 3. `resume_after_resync`가 Dart에 재시작을 알리고, 재동기화 완료를 기다린 뒤 재개한다.
//!
//! 기다리는 사이 사용자가 정지하면 그 트랙은 다시 틀지 않는다(`cancel_*`). 재개 전에 또
//! 재시작하면 남은 항목은 다음 재개로 넘어간다.
use crate::api::simple::play_track_from;
use crate::audio::engine::ENGINE_GENERATION;
use crate::audio::playback_cursor::CURSOR_TABLE;
use crate::core::state::GLOBAL_STATE;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub struct ResumeEntry {
    pub room_id: String,
    pub track_id: String,
    pub seconds: f64,
}

static PENDING: Mutex<Vec<ResumeEntry>> = Mutex::new(Vec::new());
/// 새 엔진이 준비된 자기 재시작의 순번. 장치 이벤트 스트림이 "EngineRestarted:<순번>"으로 알린다.
pub static RESTART_SEQ: AtomicU32 = AtomicU32::new(0);
/// Dart가 재동기화를 마쳤다고 알린 가장 큰 순번.
pub static RESYNC_ACK: AtomicU32 = AtomicU32::new(0);
/// Dart 재동기화를 기다리는 최대 시간. 넘기면 그대로 재개한다(무음보다 낫다).
pub const RESYNC_TIMEOUT: Duration = Duration::from_secs(2);
/// 완료 신호 뒤 재개까지 둔다. Dart 재동기화 호출은 응답을 기다리지 않고 FRB 워커 스레드에서 순서 없이
/// 실행되므로, 신호가 마지막 상태 명령보다 먼저 닿을 수 있다. 재개는 300ms 페이드 인으로 시작한다.
const SETTLE_AFTER_ACK: Duration = Duration::from_millis(100);

fn pending() -> MutexGuard<'static, Vec<ResumeEntry>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

/// 감시 루프가 자기 재시작을 결정한 직후, 옛 엔진을 drop하기 전에 부른다. 재생 목록과 커서 표로
/// 재개 항목을 만들어 대기열에 더하고 옛 엔진의 흔적(재생 목록, 커서 표, 미처리 명령)을 지운다.
/// 큐에 남은 재생 명령의 트랙은 이미 재생 목록에 있어 스냅샷에 들어갔다(위치 0) — 남겨 두면 새
/// 엔진에서 두 번 재생된다. 상태 명령은 Dart 재동기화가 다시 보낸다.
pub fn take_snapshot() {
    let playing = GLOBAL_STATE.playing_track_ids.read().unwrap_or_else(|e| e.into_inner()).clone();
    let cursors: HashMap<u64, f64> = CURSOR_TABLE.snapshot().into_iter().collect();
    let config = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner()).clone();
    let mut entries: Vec<ResumeEntry> = playing
        .iter()
        .filter_map(|(instance_id, track_id)| {
            let room = config
                .as_ref()?
                .rooms
                .iter()
                .find(|r| r.tracks.iter().any(|t| &t.id == track_id))?;
            Some(ResumeEntry {
                room_id: room.id.clone(),
                track_id: track_id.clone(),
                seconds: cursors.get(instance_id).copied().unwrap_or(0.0),
            })
        })
        .collect();
    entries.sort_by(|a, b| a.track_id.cmp(&b.track_id).then(a.seconds.total_cmp(&b.seconds)));
    pending().extend(entries);

    GLOBAL_STATE.clear_playing_tracks();
    CURSOR_TABLE.clear_all();
    while GLOBAL_STATE.command_receiver.try_recv().is_ok() {}
}

/// 사용자가 전체 정지하거나 엔진을 직접 다시 띄우면 재개 대기를 버린다.
pub fn cancel_all() {
    pending().clear();
}

/// 사용자가 트랙을 정지하면 그 트랙은 재개하지 않는다.
pub fn cancel_track(track_id: &str) {
    pending().retain(|e| e.track_id != track_id);
}

/// 지금 재개를 기다리는 항목(테스트·진단용).
pub fn pending_entries() -> Vec<ResumeEntry> {
    pending().clone()
}

/// Dart가 `seq`번 재시작의 재동기화를 마쳤다.
pub fn ack(seq: u32) {
    RESYNC_ACK.fetch_max(seq, Ordering::AcqRel);
}

/// `seq`번 완료 신호를 `timeout`까지 기다린다. 받았으면 true.
pub fn wait_for_ack(seq: u32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while RESYNC_ACK.load(Ordering::Acquire) < seq {
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    true
}

/// 대기열을 지금 재생한다. 실패한 항목(설정에서 사라진 트랙 등)은 기록하고 건너뛴다.
pub fn resume_pending() {
    let entries = std::mem::take(&mut *pending());
    for e in entries {
        if let Err(err) = play_track_from(e.room_id.clone(), e.track_id.clone(), e.seconds) {
            GLOBAL_STATE.log(format!("재시작 복원: {} 재개 실패: {}", e.track_id, err.message));
        }
    }
}

/// 새 엔진이 준비된 뒤 재기동 스레드에서 부른다. Dart에 재시작을 알리고 재동기화 완료(최대 2초)를
/// 기다린 뒤 재개한다. 기다리는 사이 세대가 바뀌면(사용자 재기동, 또 다른 재시작) 재개하지 않는다 —
/// 사용자 재기동은 대기열을 이미 버렸고, 또 다른 재시작은 대기열을 이어받는다.
pub fn resume_after_resync() {
    let generation = ENGINE_GENERATION.load(Ordering::SeqCst);
    let seq = RESTART_SEQ.fetch_add(1, Ordering::AcqRel) + 1;
    if wait_for_ack(seq, RESYNC_TIMEOUT) {
        std::thread::sleep(SETTLE_AFTER_ACK);
    }
    if ENGINE_GENERATION.load(Ordering::SeqCst) != generation {
        return;
    }
    resume_pending();
}
```

`rust/src/core/mod.rs`에 `pub mod restart_resume;`를 추가한다.

- [ ] **Step 4: `simple.rs` 연결 — 재생 함수**

1. 기존 `pub fn api_play_track(room_id: String, track_id: String) -> Result<(), AtmosError> { ... }` 본문을 그대로 새 함수로 옮긴다.

```rust
pub fn api_play_track(room_id: String, track_id: String) -> Result<(), AtmosError> {
    play_track_from(room_id, track_id, 0.0)
}

/// `start_seconds`(파일 기준)부터 재생한다. 재시작 복원(core::restart_resume)이 멈춘 위치를 넘긴다.
pub(crate) fn play_track_from(
    room_id: String,
    track_id: String,
    start_seconds: f64,
) -> Result<(), AtmosError> {
    // (기존 api_play_track 본문)
}

/// 재생 명령의 인스턴스를 `seconds` 지점부터 시작하게 한다(0이면 그대로).
fn with_start(mut cmd: AudioCommand, seconds: f64) -> AudioCommand {
    if seconds > 0.0 {
        if let AudioCommand::PlayTrack { instance, .. } = &mut cmd {
            instance.set_start_position(seconds);
        }
    }
    cmd
}
```

2. 옮긴 본문 안에서 두 가지를 바꾼다.
   - `crate::audio::streaming::DiskStreamer::new(track.file_path.clone(), track.is_loop, target_sr)`를 `crate::audio::streaming::DiskStreamer::new_at(track.file_path.clone(), track.is_loop, target_sr, start_seconds)`로 바꾼다.
   - `.send(build_play_track_command(` 네 곳을 모두 `.send(with_start(build_play_track_command(`로 바꾸고, 각 호출의 닫는 괄호 `))` 뒤에 `, start_seconds)`를 맞춰 넣는다.

- [ ] **Step 5: `simple.rs` 연결 — 사용자 동작 시 대기열 취소, 내부 기동 분리**

1. `api_stop_track` 본문 첫 줄에 `crate::core::restart_resume::cancel_track(&track_id);`를 넣는다. `api_stop_all` 본문 첫 줄에 `crate::core::restart_resume::cancel_all();`를 넣는다.

2. `api_init_audio_system`과 `api_stop_audio_engine`을 공개 진입점과 내부 함수로 나눈다. 재기동 스레드는 내부 함수만 써서 대기열을 지키고, 사용자 경로(FFI)만 대기열을 버린다.

```rust
pub fn api_init_audio_system(device_name: Option<String>) -> Result<(), AtmosError> {
    // 사용자가 엔진을 직접 다시 띄우면 재시작 복원 대기는 버린다(core::restart_resume).
    crate::core::restart_resume::cancel_all();
    init_audio_system(device_name)
}

fn init_audio_system(device_name: Option<String>) -> Result<(), AtmosError> {
    stop_audio_engine();
    // (기존 api_init_audio_system 본문에서 첫 줄 `api_stop_audio_engine();`을 뺀 나머지)
}
```

```rust
pub fn api_stop_audio_engine() {
    crate::core::restart_resume::cancel_all();
    stop_audio_engine();
}

fn stop_audio_engine() {
    let _ = ENGINE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    println!("✅ [디버깅] 백엔드 오디오 엔진 명시적 종료 지시 완료. (비동기 종료 진행)");
    broadcast_stream_status("Stopped".to_string());
}
```

- [ ] **Step 6: `simple.rs` 연결 — 스냅샷과 재개**

1. 감시 루프 뒤 `drop(engine);` 바로 앞(`[CMD] 엔진 종료 세대=` 로그 다음)에 넣는다.

```rust
                // 세대가 그대로면 자기 재시작(워치독·장치 오류·장치 목록 변화)이다. 옛 엔진을
                // 버리기 전에 재생 상태를 떠 둔다(core::restart_resume).
                if ENGINE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) == gen {
                    crate::core::restart_resume::take_snapshot();
                }
```

2. 재기동 스레드(`std::thread::spawn(move || { ... 자동 재연결(Hot-Reload) ... })`) 본문의 기동 부분을 바꾼다.

```rust
                        let started = match init_audio_system(restart_device) {
                            Ok(()) => true,
                            Err(_e) => {
                                // Emergency Failover
                                println!("🚨 [Failover] 장치 재연결 실패. WASAPI 기본 장치로 강제 비상 전환!");
                                crate::core::state::GLOBAL_STATE
                                    .is_failover_mode
                                    .store(true, std::sync::atomic::Ordering::Relaxed);
                                init_audio_system(None).is_ok() // None forces default OS device
                            }
                        };
                        if started {
                            crate::core::restart_resume::resume_after_resync();
                        }
```

- [ ] **Step 7: `simple.rs` 연결 — 재시작 이벤트와 완료 API**

1. `api_create_device_event_stream`의 스레드에서 `let mut last_ready = false;` 다음 줄에 추가한다.

```rust
        // 자기 재시작 순번. 바뀔 때마다 알린다 — 준비 상태 폴링과 달리 짧은 재기동도 놓치지 않고,
        // 순번이 매번 달라 Dart 쪽에서 같은 값으로 걸러지지 않는다.
        let mut last_restart_seq =
            crate::core::restart_resume::RESTART_SEQ.load(std::sync::atomic::Ordering::Acquire);
```

2. 루프 안의 `last_ready = ready;` 다음에 추가한다.

```rust
            let restart_seq =
                crate::core::restart_resume::RESTART_SEQ.load(std::sync::atomic::Ordering::Acquire);
            if restart_seq != last_restart_seq {
                last_restart_seq = restart_seq;
                if sink.add(format!("EngineRestarted:{restart_seq}")).is_err() {
                    break; // Stop thread if port is closed
                }
            }
```

3. 같은 루프의 `std::thread::sleep(std::time::Duration::from_millis(500));`를 `100`으로 줄인다. 재시작 알림 지연이 재개 전 무음 길이에 그대로 더해지기 때문이다.

4. `api_get_playback_positions` 아래에 추가한다.

```rust
/// Dart가 `seq`번 엔진 재시작 뒤 재동기화를 마쳤다고 알린다(core::restart_resume).
pub fn api_ack_engine_restart(seq: u32) {
    crate::core::restart_resume::ack(seq);
}
```

- [ ] **Step 8: 통과 확인**

Run: `cd atmos_mixer_pro/rust && cargo test --test test_restart_resume --test test_watchdog_restart_no_deadlock --test test_stale_engine_generation`
Expected: PASS

- [ ] **Step 9: 바인딩 생성**

```bash
cd atmos_mixer_pro && flutter_rust_bridge_codegen generate && grep -n "apiAckEngineRestart" lib/src/rust/api/simple.dart && ! grep -n "playTrackFrom" lib/src/rust/api/simple.dart && (cd rust && cargo check)
```

Expected: `apiAckEngineRestart`가 있고 `playTrackFrom`은 없다(`pub(crate)`라 공개되지 않음). codegen이 `pub(crate)` 함수를 거부하면 그 함수 위에 `#[flutter_rust_bridge::frb(ignore)]`를 붙이고 다시 생성한다.

- [ ] **Step 10: 체크포인트**

---

### Task 13: Dart가 재시작 이벤트를 받아 재동기화하고 알린다

**Files:**
- Modify: `atmos_mixer_pro/lib/core/state/engine_resync.dart`
- Modify: `atmos_mixer_pro/lib/main.dart`
- Test: `atmos_mixer_pro/test/engine_restart_event_test.dart`

**Interfaces:**
- Consumes: 장치 이벤트 `"EngineRestarted:<순번>"`, `apiAckEngineRestart({required int seq})`
- Produces: `int? parseEngineRestartedSeq(String event)`

- [ ] **Step 1: 실패하는 테스트 작성** — `test/engine_restart_event_test.dart`

```dart
import 'package:atmos_mixer_pro/core/state/engine_resync.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('EngineRestarted 이벤트에서 순번을 꺼내고, 다른 이벤트는 무시한다', () {
    expect(parseEngineRestartedSeq('EngineRestarted:7'), 7);
    expect(parseEngineRestartedSeq('EngineReady'), isNull);
    expect(parseEngineRestartedSeq('EngineRestarted:'), isNull);
    expect(parseEngineRestartedSeq('DeviceNotAvailable'), isNull);
  });
}
```

Run: `cd atmos_mixer_pro && flutter test test/engine_restart_event_test.dart` → FAIL(함수 없음)

- [ ] **Step 2: 구현** — `engine_resync.dart` 끝에 추가한다.

```dart
/// 장치 이벤트 "EngineRestarted:<순번>"(엔진이 스스로 재시작함)에서 순번을 꺼낸다. 아니면 null.
int? parseEngineRestartedSeq(String event) {
  const prefix = 'EngineRestarted:';
  if (!event.startsWith(prefix)) return null;
  return int.tryParse(event.substring(prefix.length));
}
```

`main.dart`의 `ref.listen(deviceEventStreamProvider, ...)` 블록을 바꾼다.

```dart
    ref.listen(deviceEventStreamProvider, (previous, next) {
      final event = next.value;
      if (event == 'EngineReady') {
        resyncEngineStateFromWidgetRef(ref);
      }
      // 엔진이 스스로 재시작했다(워치독·장치 오류). 설정을 다시 밀어 넣은 뒤 알려야 Rust가
      // 멈춘 위치부터 재생을 이어 튼다(rust core::restart_resume).
      final seq = event == null ? null : parseEngineRestartedSeq(event);
      if (seq != null) {
        resyncEngineStateFromWidgetRef(ref);
        apiAckEngineRestart(seq: seq);
      }
    });
```

- [ ] **Step 3: 통과 확인** — `flutter test test/engine_restart_event_test.dart` → PASS. 그리고 `flutter analyze`에서 새 이슈가 없어야 한다(기존 5건은 그대로).

- [ ] **Step 4: 체크포인트**

---

### Task 14: 5단계 통과, 재기동 창 검사 추가

**Files:**
- Modify: `atmos_mixer_pro/integration_test/app_flow_test.dart` (5단계 끝, 6단계 앞)

- [ ] **Step 1: 5단계 통과 확인**

Run: `flutter test integration_test/app_flow_test.dart -d macos`
Expected: PASS. 출력의 "5단계: … 위치 a → b" 줄에서 b가 a 이상, a+흐른 시간 이하인지 본다.

- [ ] **Step 2: 재기동 창 검사 추가** — 5단계의 `apiStopAll` 무음 확인 다음에 붙인다.

```dart
    // 5단계(이어서): 재기동 창 안의 명령(세대 확인). 재시작 사이클 전체를 5번 반복한다.
    for (var i = 1; i <= 5; i++) {
      await rust_api.apiStopAudioEngine(); // 세대만 올린다. 옛 엔진은 0~100ms 안에 drop된다.
      await rust_api.apiPlayTrack(roomId: Fixture.roomId, trackId: Fixture.t200Loop);
      await rust_api.apiInitAudioSystem(deviceName: null);
      await pumpUntil(tester,
          () => vu.peakDb(Fixture.mainChannel, window: const Duration(milliseconds: 100)) > soundDb,
          timeout: const Duration(seconds: 5),
          reason: '5단계 재기동 창 검사 $i회차: 재기동 직전에 보낸 재생 명령이 새 엔진에서 재생되지 않았다(옛 엔진이 가져감)');
      await rust_api.apiStopTrack(roomId: Fixture.roomId, trackId: Fixture.t200Loop);
      await pumpUntil(tester,
          () => vu.peakDb(Fixture.mainChannel, window: const Duration(milliseconds: 150)) < silenceDb,
          timeout: const Duration(seconds: 2), reason: '5단계 재기동 창 검사 $i회차: 정지 후 무음이 되지 않았다');
    }
```

- [ ] **Step 3: 실행해서 통과 확인** — 전체 시나리오 PASS

- [ ] **Step 4: 체크포인트**

---

### Task 15: 성공 기준 검증과 인계

**Files:**
- Modify: `atmos_mixer_pro/docs/HANDOFF.md`
- Modify: 메모리 `field-workflow-and-plan.md`(5단계 진행 상태)

- [ ] **Step 1: 3회 연속 통과** — 통합 테스트를 세 번 연달아 실행한다. 모두 PASS여야 한다.

- [ ] **Step 2: 되돌리면 실패(성공 기준 9절)** — 하나씩 바꿔 실행하고, 지정한 메시지로 실패하는지 확인한 뒤 원복한다. 원복 후 `git diff`로 해당 파일이 원래대로인지 본다.

| 변형 | 기대 실패 메시지 |
|---|---|
| `lib/main.dart` 79행 `ref.watch(acousticSyncProvider);` 주석 | "3단계: … 튜닝 게인이 바뀌지 않았다" |
| `speaker_node.dart` 292행 `'is_subwoofer': false,` | "4단계: 50Hz인데 서브(CH1)가 … 크지 않다" |
| `restart_resume.rs` `resume_after_resync`의 `resume_pending();` 삭제 | "5단계: 재시작 뒤 5초 안에 재생이 다시 이어지지 않았다" |
| `resume_pending`에서 `e.seconds` 대신 `0.0` | "5단계: … 처음부터 다시 재생됐다" |
| `main.dart` 리스너의 `resyncEngineStateFromWidgetRef(ref);` 두 곳 모두 삭제 | "5단계: 재시작 뒤 CH… 레벨이 … 바뀌었다" |
| `engine.rs` `begin_callback`의 `if ENGINE_GENERATION.load(Ordering::Acquire) != generation { return false; }` 주석 | "5단계 재기동 창 검사 N회차: …"(5회 중 한 번 이상, 약 99.9%) |

- [ ] **Step 3: 릴리스 심볼 확인(성공 기준 9절 4번)** — 디스크 여유 3GB 이상을 확인한 뒤 실행한다.

```bash
cd atmos_mixer_pro && flutter build macos --release && nm -gU build/macos/Build/Products/Release/atmos_mixer_pro.app/Contents/Frameworks/rust_lib_atmos_mixer_pro.framework/rust_lib_atmos_mixer_pro | grep -c atmos_test_ ; nm -gU build/macos/Build/Products/Debug/atmos_mixer_pro.app/Contents/Frameworks/rust_lib_atmos_mixer_pro.framework/rust_lib_atmos_mixer_pro | grep -c atmos_test_
```

Expected: 첫 숫자(릴리스)는 `0`, 둘째(디버그, 통합 테스트 빌드)는 `1`.

- [ ] **Step 4: 전체 검사**

```bash
cd atmos_mixer_pro && flutter analyze && flutter test && cd rust && cargo test && cargo clippy --all-targets 2>&1 | grep -E "^\s+--> " | grep -E "playback_cursor|restart_resume|test_hooks|streaming\.rs|player\.rs|test_playback|test_instance_position|test_disk_streamer|test_restart_resume"
```

Expected: analyze는 기존 5건만, flutter test와 cargo test는 전부 통과, 마지막 grep은 출력 없음(새 clippy 경고 없음). 전체 `cargo test`는 디스크를 많이 쓰니 여유를 감시하며 돌린다.

- [ ] **Step 5: 인계 문서 갱신** — `docs/HANDOFF.md`
  - "최근 끝난 일"에 재시작 복원과 통합 테스트(실행 명령 포함)를 추가한다.
  - "남은 일"에서 통합 테스트와 재시작 복원 구현 항목을 지운다.
  - Windows 미검증 목록에 "재시작 복원(ASIO/WASAPI 재기동 경로)"을 추가한다.
  - 설계 10절 "알려진 한계"를 가리키는 줄을 넣는다.

- [ ] **Step 6: 메모리 갱신** — `field-workflow-and-plan.md`의 5단계 진행 상태를 갱신한다.

- [ ] **Step 7: 최종 체크포인트** — 전체 `git status`·`git diff --stat`을 사용자에게 보고하고, 커밋 여부를 묻는다.

---

## 계획 자기 검토

- **설계 반영 범위:**
  - 설계 4절 격리 → 작업 1.
  - 5절 0~6단계 → 작업 1~6, 11, 14.
  - 6절 요구 R1~R5, 구조 1~6 → 작업 7~13.
  - 6절 재개 대기 무음 → 작업 11의 무음 검사.
  - 위치 읽기 논블로킹 → 작업 7.
  - 결함 주입 비노출 → 작업 10, 15.
  - 7절 코드 변경 → 작업 1·4·5·12·13.
  - 9절 성공 기준 → 작업 15.
  - 10절 한계 → 인계 문서 갱신.
- **설계와 다르게 정한 것:**
  - 3단계 딜레이 검사는 CH2가 아니라 CH1 딜레이로 한다(시간 정렬 기준 때문, 작업 4).
  - 6단계 워치독 판정은 스트림 상태 대신 `EngineRestarted` 이벤트로 한다(전역 단일 싱크 때문, 작업 6).
  - 재생 위치 조회는 재생 목록과 맞춰 걸러 낸다(엔진이 버린 옛 칸을 숨김, 작업 9).
  - 재개 대기는 전역 대기열로 바꿨다(사용자 정지·재시작 폭주 처리, 작업 12).
  - 스트리밍(루프 포함) 커서 복원은 시크 대신 "디코딩해서 버리기"로 정했다. 형식과 무관하게 샘플 단위로 정확하고, 루프는 바퀴 안 위치라 건너뛸 양이 짧다(작업 8).
- **알아 둘 것:** 루프 트랙은 항상 스트리밍으로 재생된다(`api_play_track`). 그래서 테마 루프의 재개는 작업 8에 달려 있다.
