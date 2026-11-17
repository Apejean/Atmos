import 'dart:async';
import 'package:flutter/material.dart';

/// 엔진 상태 문자열이 "복구 실패로 엔진이 내려갔다"를 뜻하는 접두사(Rust `core::event_log::ENGINE_DOWN_PREFIX`).
/// 값 계약: Running · Failover · Stopped · StoppedByUser · HotReloading · WatchdogActive · AutoGuardActive
/// · EngineDown:<원인> · WaitingForDevice:<시도 수>:<다음 시도까지 초>(Rust `core::engine_keeper::status_string`)
/// · LoadHigh:<최대ms>/<예산ms>/<초과회>/<늦은회>/<처리채널> · LoadNormal(load_warning_banner.dart).
const String engineDownPrefix = 'EngineDown:';

/// 엔진 유지자가 오디오 장치를 기다리는 동안의 상태 접두사(안정성 ③).
const String waitingForDevicePrefix = 'WaitingForDevice:';

/// `EngineDown:<원인>`이면 원인을, 아니면 null을 돌려준다.
String? parseEngineDown(String status) =>
    status.startsWith(engineDownPrefix) ? status.substring(engineDownPrefix.length) : null;

/// `WaitingForDevice:<시도 수>:<다음 시도까지 초>`이면 (시도 수, 초)를, 아니면 null을 돌려준다.
(int, int)? parseWaitingForDevice(String status) {
  if (!status.startsWith(waitingForDevicePrefix)) return null;
  final parts = status.substring(waitingForDevicePrefix.length).split(':');
  if (parts.length != 2) return null;
  final attempts = int.tryParse(parts[0]);
  final next = int.tryParse(parts[1]);
  if (attempts == null || next == null) return null;
  return (attempts, next);
}

/// 실패 원인별 안내(유지 스레드 설계 8절, 사용자 결정 U4). 모르는 원인은 null(일반 문구를 쓴다).
String? engineDownAdvice(String cause) {
  switch (cause) {
    case 'open_failed':
      return '오디오 장치는 보이지만 열지 못했습니다 — 30초마다 다시 시도합니다. 다른 프로그램이 장치를 쓰는지 확인하세요.';
    case 'start_hung':
      return '오디오 드라이버가 응답하지 않습니다. 앱을 다시 시작해야 합니다.';
    case 'unavailable':
      return '설정의 오디오 장치를 이 컴퓨터에서 쓸 수 없습니다 — 환경설정에서 장치를 다시 고르세요.';
  }
  return null;
}

/// 엔진이 멈춘 동안 사라지지 않는 배너. 상태 스트림은 주입받는다(시험용으로 가짜 스트림을 쓴다).
/// `WaitingForDevice:…`이면 장치를 기다리는 주황 배너를, `EngineDown:<원인>`이면 빨간 배너를 보인다.
/// `Running`·`Failover`(다시 열림)와 `StoppedByUser`(사용자가 엔진을 정지)를 받으면 사라진다.
class EngineDownBanner extends StatefulWidget {
  const EngineDownBanner({super.key, required this.statusStream, this.now = DateTime.now});

  final Stream<String> statusStream;
  final DateTime Function() now;

  @override
  State<EngineDownBanner> createState() => _EngineDownBannerState();
}

class _EngineDownBannerState extends State<EngineDownBanner> {
  StreamSubscription<String>? _sub;
  String? _cause;
  (int, int)? _waiting;
  DateTime? _since;
  int _count = 0;

  @override
  void initState() {
    super.initState();
    _sub = widget.statusStream.listen(_onStatus);
  }

  void _onStatus(String status) {
    if (!mounted) return;
    final cause = parseEngineDown(status);
    final waiting = parseWaitingForDevice(status);
    if (cause != null) {
      setState(() {
        // "…부터"는 처음 내려간 시각이다. 이어지는 실패는 원인과 횟수만 바꾼다.
        if (_cause == null && _waiting == null) _since = widget.now();
        _cause = cause;
        _waiting = null;
        _count += 1;
      });
    } else if (waiting != null) {
      setState(() {
        if (_cause == null && _waiting == null) _since = widget.now();
        _cause = null;
        _waiting = waiting;
      });
    } else if ((status == 'Running' || status == 'Failover' || status == 'StoppedByUser') &&
        (_cause != null || _waiting != null)) {
      setState(() {
        _cause = null;
        _waiting = null;
        _since = null;
        _count = 0;
      });
    }
  }

  @override
  void dispose() {
    _sub?.cancel();
    super.dispose();
  }

  static String _two(int v) => v.toString().padLeft(2, '0');

  @override
  Widget build(BuildContext context) {
    final since = _since;
    if (since == null) return const SizedBox.shrink();
    final t = '${_two(since.hour)}:${_two(since.minute)}:${_two(since.second)}';
    final waiting = _waiting;
    final cause = _cause;
    final String text;
    final Color color;
    if (waiting != null) {
      final (attempts, next) = waiting;
      text = '오디오 장치를 기다리는 중입니다 — $attempts번 시도, 다음 시도 $next초 뒤 · $t부터. 인터페이스 전원·USB를 확인하세요.';
      color = Colors.orange.shade900;
    } else if (cause != null) {
      final advice = engineDownAdvice(cause);
      text = advice != null
          ? '$advice ($cause · $t부터)'
          : '오디오 엔진이 멈췄고 자동 복구에 실패했습니다 — 마지막 원인: $cause · $t부터 · 연속 $_count회. '
              '오디오 장치 연결을 확인하고 앱을 다시 시작해 주세요.';
      color = Colors.red.shade900;
    } else {
      return const SizedBox.shrink();
    }
    return Material(
      color: color,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Row(
          children: [
            const Icon(Icons.error_outline, color: Colors.white),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                text,
                style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
