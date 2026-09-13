import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;

/// `apiSetBinauralEnabled`를 켜고 끄는 스위치(P1-8, 예전에는 백엔드 함수만
/// 있고 UI에 연결되지 않은 고아 상태였다).
///
/// 지금은 설정이 config.json에 저장되지 않는다 — 켠 상태는 앱을 다시 실행하면
/// 꺼진다. 지속시키려면 AppConfig에 필드를 추가하고 저장/로드 경로를 만들어야
/// 하는데, 이번 요청 범위는 "스위치 연결"이라 거기까지는 하지 않았다.
class BinauralToggleBadge extends ConsumerStatefulWidget {
  const BinauralToggleBadge({super.key});

  @override
  ConsumerState<BinauralToggleBadge> createState() =>
      _BinauralToggleBadgeState();
}

class _BinauralToggleBadgeState extends ConsumerState<BinauralToggleBadge> {
  bool _enabled = false;

  void _toggle() {
    final next = !_enabled;
    // 오디오 스레드로 커맨드만 보내는 가벼운 동기 호출이라 대기 없이 즉시
    // 로컬 상태도 함께 바꾼다(다른 badge들과 동일한 낙관적 갱신 패턴).
    rust_api.apiSetBinauralEnabled(enabled: next);
    setState(() {
      _enabled = next;
    });
  }

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: _toggle,
      borderRadius: BorderRadius.circular(6),
      child: Tooltip(
        message: _enabled
            ? '바이노럴 렌더링 켜짐 — 헤드폰으로 채널별 실제 방향을 재현합니다'
            : '바이노럴 렌더링 꺼짐 — 각 채널이 지정된 물리 출력으로 그대로 나갑니다',
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 200),
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
          decoration: BoxDecoration(
            color: _enabled
                ? Colors.deepPurple.shade900.withValues(alpha: 0.8)
                : AppColors.cardSurface,
            borderRadius: BorderRadius.circular(6),
            border: Border.all(
              color: _enabled ? Colors.purpleAccent : Colors.white12,
              width: 1,
            ),
            boxShadow: _enabled
                ? [
                    BoxShadow(
                      color: Colors.purpleAccent.withValues(alpha: 0.2),
                      blurRadius: 4,
                      spreadRadius: 1,
                    ),
                  ]
                : [],
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                _enabled ? Icons.headphones : Icons.headphones_outlined,
                size: 14,
                color: _enabled ? Colors.purpleAccent : Colors.white54,
              ),
              const SizedBox(width: 6),
              Text(
                _enabled ? 'BINAURAL ON' : 'BINAURAL OFF',
                style: TextStyle(
                  color: _enabled ? Colors.purpleAccent : Colors.white70,
                  fontSize: 10,
                  fontWeight: FontWeight.bold,
                  letterSpacing: 0.5,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
