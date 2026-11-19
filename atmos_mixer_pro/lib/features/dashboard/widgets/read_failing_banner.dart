import 'dart:async';
import 'package:flutter/material.dart';

/// 음원 파일을 다시 열지 못해 무음으로 기다리는 트랙 상태(Rust `core::read_failures`, 검토 A6). **엔진 상태가 아니다.**
/// 값 계약: `ReadFailing:<트랙 수>:<첫 파일 이름>`(수 1 이상) · `ReadFailing:0`(모두 회복·정지).
/// 파일 이름은 경로가 아니라 마지막 이름이고 ':'가 섞일 수 있어 맨 뒤에 온다(앞 두 칸만 나눠 읽는다).
/// 배너는 이 값만 따른다 — 엔진을 다시 열면 옛 트랙이 버려져 Rust가 `ReadFailing:0`을 보내므로 `Running`으로 내리지 않는다.
const String readFailingPrefix = 'ReadFailing:';

/// `ReadFailing:` 값 하나.
class ReadFailing {
  const ReadFailing(this.count, this.firstFile);

  final int count;

  /// 가장 먼저 실패한 트랙의 파일 이름. `count`가 0이면 null.
  final String? firstFile;

  /// `ReadFailing:`로 시작하고 형식이 맞으면 값을, 아니면 null.
  static ReadFailing? parse(String status) {
    if (!status.startsWith(readFailingPrefix)) return null;
    final rest = status.substring(readFailingPrefix.length);
    final colon = rest.indexOf(':');
    final count = int.tryParse(colon < 0 ? rest : rest.substring(0, colon));
    if (count == null || count < 0) return null;
    if (count == 0) return const ReadFailing(0, null);
    if (colon < 0) return null;
    return ReadFailing(count, rest.substring(colon + 1));
  }
}

/// 음원 파일을 읽지 못해 무음인 트랙이 있는 동안 보이는 지속 배너(무인 현장에서 사라지지 않는다). 상태 스트림은 주입받는다.
/// 색은 장치 대기 배너와 같은 "기다리는 중" 주황이다(engine_down_banner.dart).
class ReadFailingBanner extends StatefulWidget {
  const ReadFailingBanner({super.key, required this.statusStream});

  final Stream<String> statusStream;

  @override
  State<ReadFailingBanner> createState() => _ReadFailingBannerState();
}

class _ReadFailingBannerState extends State<ReadFailingBanner> {
  StreamSubscription<String>? _sub;
  ReadFailing? _failing;

  @override
  void initState() {
    super.initState();
    _sub = widget.statusStream.listen(_onStatus);
  }

  void _onStatus(String status) {
    if (!mounted) return;
    final value = ReadFailing.parse(status);
    if (value == null) return;
    setState(() => _failing = value.count > 0 ? value : null);
  }

  @override
  void dispose() {
    _sub?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final failing = _failing;
    if (failing == null) return const SizedBox.shrink();
    return Material(
      color: Colors.orange.shade900,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Row(
          children: [
            const Icon(Icons.music_off, color: Colors.white),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                '음원 파일을 읽지 못해 트랙 ${failing.count}개가 무음입니다: ${failing.firstFile}. '
                '자동으로 계속 다시 시도합니다(10초마다). 파일·디스크 연결을 확인하세요.',
                style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
