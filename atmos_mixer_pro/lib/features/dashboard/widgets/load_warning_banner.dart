import 'dart:async';
import 'package:flutter/material.dart';

/// 부하 경고 상태 문자열(Rust `core::load_warning`).
/// `LoadHigh:<최대ms>/<예산ms>/<초과회>/<늦은회>/<처리채널>` · `LoadNormal`.
/// 엔진 상태가 아니라 부하 표시다. 같은 상태 스트림을 쓰지만 "마지막 문자열 = 엔진 상태"로 보지 않는다.
const String loadHighPrefix = 'LoadHigh:';
const String loadNormalStatus = 'LoadNormal';

/// `LoadHigh:` 값 하나.
class LoadHigh {
  const LoadHigh({
    required this.maxMs,
    required this.budgetMs,
    required this.overruns,
    required this.late,
    required this.processingChannels,
  });

  final double maxMs;
  final double budgetMs;
  final int overruns;
  final int late;
  final int processingChannels;

  /// `LoadHigh:`로 시작하고 다섯 값이 모두 숫자면 값을, 아니면 null.
  static LoadHigh? parse(String status) {
    if (!status.startsWith(loadHighPrefix)) return null;
    final parts = status.substring(loadHighPrefix.length).split('/');
    if (parts.length != 5) return null;
    final maxMs = double.tryParse(parts[0]);
    final budgetMs = double.tryParse(parts[1]);
    final overruns = int.tryParse(parts[2]);
    final late = int.tryParse(parts[3]);
    final channels = int.tryParse(parts[4]);
    if (maxMs == null || budgetMs == null || overruns == null || late == null || channels == null) {
      return null;
    }
    return LoadHigh(
      maxMs: maxMs,
      budgetMs: budgetMs,
      overruns: overruns,
      late: late,
      processingChannels: channels,
    );
  }
}

/// 오디오 처리 부하가 예산을 넘거나 콜백이 늦은 동안 보이는 배너. 상태 스트림은 주입받는다.
/// `LoadHigh`를 받으면 보이고(값은 최근 구간으로 갱신), `LoadNormal`(1분 정상)이나 `Running`·`Failover`(엔진 다시 시작)에서
/// 사라진다. 새 엔진은 경고 없음으로 시작해 `LoadNormal`을 보내지 않으므로 엔진 시작 신호에서 지워야 한다.
class LoadWarningBanner extends StatefulWidget {
  const LoadWarningBanner({super.key, required this.statusStream});

  final Stream<String> statusStream;

  @override
  State<LoadWarningBanner> createState() => _LoadWarningBannerState();
}

class _LoadWarningBannerState extends State<LoadWarningBanner> {
  StreamSubscription<String>? _sub;
  LoadHigh? _load;

  @override
  void initState() {
    super.initState();
    _sub = widget.statusStream.listen(_onStatus);
  }

  void _onStatus(String status) {
    if (!mounted) return;
    final load = LoadHigh.parse(status);
    if (load != null) {
      setState(() => _load = load);
    } else if ((status == loadNormalStatus || status == 'Running' || status == 'Failover') && _load != null) {
      setState(() => _load = null);
    }
  }

  @override
  void dispose() {
    _sub?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final load = _load;
    if (load == null) return const SizedBox.shrink();
    return Material(
      color: Colors.amber.shade800,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Row(
          children: [
            const Icon(Icons.speed, color: Colors.black),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                '오디오 처리 부하가 높아 소리가 끊길 수 있습니다 — 최근 10초 최대 ${load.maxMs.toStringAsFixed(1)}ms'
                ' / 예산 ${load.budgetMs.toStringAsFixed(1)}ms, 예산 초과 ${load.overruns}회, 늦은 콜백 ${load.late}회,'
                ' 처리 채널 ${load.processingChannels}개. 쓰지 않는 출력 채널을 끄거나 자동 보정·리버브를 줄여 주세요.',
                style: const TextStyle(color: Colors.black, fontWeight: FontWeight.bold),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
