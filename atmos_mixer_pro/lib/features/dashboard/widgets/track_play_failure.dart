import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/src/rust/api/error.dart';

/// 트랙을 재생하지 못했을 때 이유를 화면 아래 알림으로 보여 준다(앱과 엔진은 그대로 돈다).
///
/// 예전에는 엔진이 멈췄을 때 쓰는 "치명적 시스템 오류" 창(케이블 확인·엔진 재시작 안내)에 이유 대신
/// `Instance of 'AtmosError'`가 떴다(2026-10-10 Windows P6-E: 파일이 없는 트랙 재생). 파일이 없는 것은
/// 엔진 고장이 아니므로 알림으로 보여 주고, 자세한 내용은 앱 로그에 남는다(rust `api_play_track`).
void showTrackPlayFailure(BuildContext context, Object error, {String prefix = '트랙 재생 실패'}) {
  if (!context.mounted) return;
  final reason = error is AtmosError ? error.message : '$error';
  ScaffoldMessenger.of(context).showSnackBar(
    SnackBar(
      content: Text('$prefix: $reason'),
      backgroundColor: AppColors.danger,
      duration: const Duration(seconds: 8),
    ),
  );
}
