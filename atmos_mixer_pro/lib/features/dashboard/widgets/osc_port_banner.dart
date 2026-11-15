import 'dart:async';
import 'package:flutter/material.dart';

/// OSC 수신 포트 상태 문자열(Rust `osc::listener`). **엔진 상태가 아니라** 포트 표시다.
/// `OscPortBlocked:<port>` · `OscPortOk:<port>`.
const String oscPortBlockedPrefix = 'OscPortBlocked:';
const String oscPortOkPrefix = 'OscPortOk:';

/// `OscPortBlocked:<port>`이면 포트 번호, 아니면 null.
int? parseOscPortBlocked(String status) =>
    status.startsWith(oscPortBlockedPrefix) ? int.tryParse(status.substring(oscPortBlockedPrefix.length)) : null;

/// 설정한 OSC 수신 포트를 열지 못해 다시 시도하는 동안 보이는 배너. 상태 스트림은 주입받는다.
/// `OscPortBlocked:<port>`에서 보이고, 같은 포트의 `OscPortOk:<port>`에서 사라진다.
class OscPortBanner extends StatefulWidget {
  const OscPortBanner({super.key, required this.statusStream});

  final Stream<String> statusStream;

  @override
  State<OscPortBanner> createState() => _OscPortBannerState();
}

class _OscPortBannerState extends State<OscPortBanner> {
  StreamSubscription<String>? _sub;
  int? _port;

  @override
  void initState() {
    super.initState();
    _sub = widget.statusStream.listen(_onStatus);
  }

  void _onStatus(String status) {
    if (!mounted) return;
    final blocked = parseOscPortBlocked(status);
    if (blocked != null) {
      setState(() => _port = blocked);
    } else if (_port != null && status == '$oscPortOkPrefix$_port') {
      setState(() => _port = null);
    }
  }

  @override
  void dispose() {
    _sub?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final port = _port;
    if (port == null) return const SizedBox.shrink();
    return Material(
      color: Colors.deepOrange.shade800,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Row(
          children: [
            const Icon(Icons.portable_wifi_off, color: Colors.white),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                'OSC 수신 포트 $port를 열지 못해 외부 신호를 받지 못하고 있습니다. 다른 프로그램이 이 포트를 쓰고 있을 수 '
                '있습니다. 자동으로 계속 다시 시도합니다(환경설정에서 포트를 바꿀 수도 있습니다).',
                style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
