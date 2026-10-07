// 통합 테스트 관찰 도구. 앱이 이미 가진 스트림(VU, 장치 이벤트)을 같이 듣는다.
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:math';

import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/core/utils/rust_library.dart' show windowsBundledRustDllPath;
import 'package:atmos_mixer_pro/features/exhibition/state/three_js_engine_provider.dart';
import 'package:flutter/foundation.dart' show setEquals;
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

  /// 최근 [window] 동안 채널 [ch](0부터)의 표본(dBFS). 표본이 1초 넘게 오지 않으면 VU 스트림이 멈춘
  /// 것이라 실패한다 — 멈춘 스트림을 무음으로 착각하지 않게 한다.
  List<double> _recent(int ch, Duration window) {
    final now = DateTime.now();
    if (_samples.isEmpty || now.difference(_samples.last.at) > const Duration(seconds: 1)) {
      fail('VU 표본이 1초 넘게 오지 않는다(VU 스트림이 멈췄다)');
    }
    final from = now.subtract(window);
    return [
      for (final s in _samples)
        if (!s.at.isBefore(from) && ch < s.levels.length) toDb(s.levels[ch]),
    ];
  }

  /// 최근 [window] 동안 채널 [ch](0부터)의 최대 피크(dBFS). 기다린 뒤 값을 잴 때 쓴다 — 표본이 없으면 실패한다.
  double peakDb(int ch, {Duration window = const Duration(milliseconds: 300)}) {
    final v = _recent(ch, window);
    if (v.isEmpty) fail('CH${ch + 1} VU 표본이 최근 ${window.inMilliseconds}ms 동안 없다');
    return v.reduce(max);
  }

  /// 기다림 조건: 최근 [window]에 표본이 있고 최대 피크가 소리 문턱([soundDb]) 위다.
  bool sounding(int ch, {required Duration window}) {
    final v = _recent(ch, window);
    return v.isNotEmpty && v.reduce(max) > soundDb;
  }

  /// 기다림 조건: 최근 [window]에 표본이 있고 모두 무음 문턱([silenceDb]) 아래다. 표본이 없는 창은
  /// 무음으로 치지 않는다(앱이 잠깐 바빠 VU가 늦게 들어온 것일 수 있다).
  bool silent(int ch, {required Duration window}) {
    final v = _recent(ch, window);
    return v.isNotEmpty && v.reduce(max) < silenceDb;
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

/// 재생 목록(엔진 상태 스트림)이 바뀐 시각을 남긴다. 받은 순서대로 쌓인다.
class PlayingProbe {
  PlayingProbe(ProviderContainer container) {
    _sub = container.listen<EngineState>(engineStateProvider, (_, next) {
      final ids = next.playingTrackIds.toSet();
      if (changes.isEmpty || !setEquals(changes.last.$2, ids)) changes.add((DateTime.now(), ids));
    }, fireImmediately: true);
  }

  late final ProviderSubscription<EngineState> _sub;
  final changes = <(DateTime, Set<String>)>[];

  /// [after] 뒤 재생 목록이 비었다가 처음 다시 채워진 시각. 없으면 null.
  /// 재시작 복원은 스냅샷에서 목록을 비우고, 재개할 때 목록에 먼저 올린 뒤 재생 명령을 보낸다.
  DateTime? refilledAfter(DateTime after) {
    var emptied = false;
    for (final (at, ids) in changes) {
      if (at.isBefore(after)) continue;
      if (ids.isEmpty) {
        emptied = true;
      } else if (emptied) {
        return at;
      }
    }
    return null;
  }

  void close() => _sub.close();
}

/// 3D 장면이 노드를 탭했을 때 보내는 메시지를 JS 채널에 그대로 넣는다.
/// Three.js의 클릭 판정(레이캐스팅)은 거치지 않는다(설계 10절 "알려진 한계").
Future<void> tapSpeakerNode(ProviderContainer container, String speakerId) async {
  final engine = container.read(threeJsEngineProvider);
  final message = jsonEncode({'type': 'SPEAKER_SELECTED', 'speakerId': speakerId});
  await engine.controller!.runJavaScript('SpeakerBridge.postMessage(${jsonEncode(message)});');
}

/// 디버그 빌드에만 있는 결함 주입 훅(rust/src/test_hooks.rs). 워치독과 같은 자기 재시작을 일으킨다.
/// FRB와 같은 Rust 프레임워크에서 심볼을 찾는다.
void requestEngineRecovery() {
  final lib = DynamicLibrary.open(
    Platform.isWindows
        ? windowsBundledRustDllPath(Platform.resolvedExecutable)
        : 'rust_lib_atmos_mixer_pro.framework/rust_lib_atmos_mixer_pro',
  );
  lib.lookupFunction<Void Function(), void Function()>('atmos_test_request_engine_recovery')();
}

/// 0단계 사전 확인: 이 테스트 앱 말고 다른 Atmos 앱이 떠 있으면 실패한다.
Future<void> expectNoOtherAppInstance() async {
  final others = Platform.isWindows ? await _windowsAppPids() : await _macosAppPids();
  if (others.isNotEmpty) {
    fail('0단계: 다른 Atmos 앱 인스턴스(pid ${others.join(', ')})가 실행 중이다. 종료한 뒤 다시 실행하라.');
  }
}

/// macOS: 테스트 자신을 뺀 Atmos 앱 프로세스 PID.
Future<List<String>> _macosAppPids() async {
  final r = await Process.run('pgrep', ['-f', 'atmos_mixer_pro.app/Contents/MacOS/atmos_mixer_pro']);
  return (r.stdout as String)
      .split('\n')
      .map((s) => s.trim())
      .where((s) => s.isNotEmpty && s != '$pid')
      .toList();
}

/// Windows: 테스트 자신을 뺀 앱·감시 프로세스 PID. tasklist CSV 한 줄이 `"이미지","PID",…`다.
/// 없으면 CSV가 아니라 안내 문장이 나오므로 따옴표로 시작하는 줄만 본다.
Future<List<String>> _windowsAppPids() async {
  final found = <String>[];
  for (final image in ['atmos_mixer_pro.exe', 'atmos_supervisor.exe']) {
    final r = await Process.run('tasklist', ['/FI', 'IMAGENAME eq $image', '/FO', 'CSV', '/NH']);
    for (final line in (r.stdout as String).split('\n')) {
      final fields = line.trim().split('","');
      if (!line.trim().startsWith('"') || fields.length < 2) continue;
      final p = fields[1].replaceAll('"', '');
      if (p != '$pid') found.add(p);
    }
  }
  return found;
}
