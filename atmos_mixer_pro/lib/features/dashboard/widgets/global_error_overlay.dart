import 'package:atmos_mixer_pro/core/state/engine_resync.dart';
import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// 화면 가운데 오류 창(globalErrorProvider). 대시보드 Stack 안에 둔다.
/// 엔진·장치 오류는 케이블 확인·엔진 리셋 안내와 함께 띄우고, 저장·불러오기 같은 작업 실패는 확인만 받는다.
/// 예전에는 모든 오류가 "치명적 시스템 오류" 창에 떠서, 로그 저장 실패를 닫으려 해도 엔진을 다시 시작해야 했다
/// (HANDOFF 남은 일 14).
class GlobalErrorOverlay extends ConsumerStatefulWidget {
  const GlobalErrorOverlay({super.key});

  @override
  ConsumerState<GlobalErrorOverlay> createState() => _GlobalErrorOverlayState();
}

class _GlobalErrorOverlayState extends ConsumerState<GlobalErrorOverlay> {
  bool _isRecovering = false;

  @override
  Widget build(BuildContext context) {
    final error = ref.watch(globalErrorProvider);
    if (error == null) return const SizedBox.shrink();

    return Positioned.fill(
      child: Container(
        color: Colors.black87,
        child: Center(
          child: error.engine ? _engineError(context, error.message) : _operationError(error.message),
        ),
      ),
    );
  }

  Widget _engineError(BuildContext context, String message) {
    return _card(
      color: AppColors.danger,
      icon: Icons.warning_rounded,
      title: '치명적 시스템 오류 발생',
      message: message,
      guidance: '오디오 장치 케이블 연결을 확인하고\n아래의 복구 버튼을 눌러 엔진을 재시작하세요.',
      button: ElevatedButton.icon(
        onPressed: _isRecovering
            ? null
            : () async {
                final config = ref.read(configProvider);
                if (config != null) {
                  setState(() {
                    _isRecovering = true;
                  });
                  try {
                    await rust_api.apiForceRestartEngine(
                      deviceName: config.deviceName,
                    );
                    resyncEngineStateFromWidgetRef(ref);
                    if (context.mounted) {
                      ref.read(globalErrorProvider.notifier).clearError();
                    }
                  } finally {
                    if (context.mounted) {
                      setState(() {
                        _isRecovering = false;
                      });
                    }
                  }
                }
              },
        icon: _isRecovering
            ? const SizedBox(
                width: 20,
                height: 20,
                child: CircularProgressIndicator(
                  strokeWidth: 2,
                  color: Colors.white,
                ),
              )
            : const Icon(Icons.refresh, size: 24),
        label: Text(
          _isRecovering ? '복구 중...' : '엔진 리셋 (원클릭 복구)',
          style: const TextStyle(
            fontSize: 18,
            fontWeight: FontWeight.bold,
          ),
        ),
        style: ElevatedButton.styleFrom(
          backgroundColor: AppColors.danger,
          foregroundColor: Colors.white,
          disabledBackgroundColor: Colors.white24,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(4),
          ),
        ),
      ),
    );
  }

  Widget _operationError(String message) {
    return _card(
      color: AppColors.accentOrange,
      icon: Icons.error_outline,
      title: '작업을 마치지 못했습니다',
      message: message,
      button: ElevatedButton(
        onPressed: () => ref.read(globalErrorProvider.notifier).clearError(),
        style: ElevatedButton.styleFrom(
          backgroundColor: AppColors.accentOrange,
          foregroundColor: Colors.white,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(4),
          ),
        ),
        child: const Text(
          '확인',
          style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
        ),
      ),
    );
  }

  Widget _card({
    required Color color,
    required IconData icon,
    required String title,
    required String message,
    String? guidance,
    required Widget button,
  }) {
    return Container(
      constraints: const BoxConstraints(maxWidth: 500),
      padding: const EdgeInsets.all(32),
      decoration: BoxDecoration(
        color: AppColors.background,
        borderRadius: BorderRadius.circular(4),
        border: Border.all(
          color: color.withValues(alpha: 0.5),
          width: 1.0,
        ),
        boxShadow: [
          BoxShadow(
            color: color.withValues(alpha: 0.15),
            blurRadius: 12,
            spreadRadius: 2,
          ),
        ],
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, color: color, size: 64),
          const SizedBox(height: 24),
          Text(
            title,
            style: TextStyle(
              color: color,
              fontSize: 24,
              fontWeight: FontWeight.bold,
            ),
          ),
          const SizedBox(height: 16),
          Text(
            message,
            textAlign: TextAlign.center,
            style: const TextStyle(color: Colors.white, fontSize: 16),
          ),
          if (guidance != null) ...[
            const SizedBox(height: 24),
            Text(
              guidance,
              textAlign: TextAlign.center,
              style: const TextStyle(color: Colors.white70, fontSize: 14),
            ),
          ],
          const SizedBox(height: 32),
          SizedBox(width: double.infinity, height: 56, child: button),
        ],
      ),
    );
  }
}
