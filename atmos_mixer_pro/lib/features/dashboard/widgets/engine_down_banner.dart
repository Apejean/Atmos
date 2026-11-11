import 'dart:async';
import 'package:flutter/material.dart';

/// 엔진 상태 문자열이 "복구 실패로 엔진이 내려갔다"를 뜻하는 접두사(Rust `core::event_log::ENGINE_DOWN_PREFIX`).
/// 값 계약: Running · Failover · Stopped · HotReloading · WatchdogActive · AutoGuardActive · EngineDown:<원인>.
const String engineDownPrefix = 'EngineDown:';

/// `EngineDown:<원인>`이면 원인을, 아니면 null을 돌려준다.
String? parseEngineDown(String status) =>
    status.startsWith(engineDownPrefix) ? status.substring(engineDownPrefix.length) : null;

/// 엔진이 복구에 실패해 소리가 멈춘 동안 사라지지 않는 배너. 상태 스트림은 주입받는다(시험용으로 가짜 스트림을 쓴다).
/// `EngineDown:<원인>`을 받으면 시각·원인·연속 횟수를 보이고, `Running`·`Failover`를 받으면 사라진다.
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
    if (cause != null) {
      setState(() {
        // "…부터"는 처음 내려간 시각이다. 이어지는 실패는 원인과 횟수만 바꾼다.
        if (_cause == null) _since = widget.now();
        _cause = cause;
        _count += 1;
      });
    } else if ((status == 'Running' || status == 'Failover') && _cause != null) {
      setState(() {
        _cause = null;
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
    final cause = _cause;
    final since = _since;
    if (cause == null || since == null) return const SizedBox.shrink();
    final t = '${_two(since.hour)}:${_two(since.minute)}:${_two(since.second)}';
    return Material(
      color: Colors.red.shade900,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Row(
          children: [
            const Icon(Icons.error_outline, color: Colors.white),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                '오디오 엔진이 멈췄고 자동 복구에 실패했습니다 — 마지막 원인: $cause · $t부터 · 연속 $_count회. '
                '오디오 장치 연결을 확인하고 앱을 다시 시작해 주세요.',
                style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
