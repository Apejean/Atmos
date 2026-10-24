import 'dart:io';

import 'package:atmos_mixer_pro/core/legal/legal_texts.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

/// Help → Credits(2026-10-25 사용자 결정: Ableton Live처럼). 앱 안에는 매뉴얼 Credits 같은 글자 고지만 두고,
/// 공개 소프트웨어 라이선스 전문과 ASIO 로고는 설치 폴더 Legal 폴더의 문서에 둔다. 로고는 여기 두지 않는다
/// (ASIO 지침 1절 f의 로고는 ASIO를 고르는 환경설정에 있다).
Future<void> showCredits(BuildContext context) {
  return showDialog<void>(
    context: context,
    builder: (context) => AlertDialog(
      backgroundColor: AppColors.background,
      title: const Text('Credits', style: TextStyle(color: Colors.white)),
      content: SizedBox(
        width: 560,
        child: SingleChildScrollView(child: CreditsText(legalFolder: legalFolderPath())),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('확인', style: TextStyle(color: Colors.white70)),
        ),
      ],
    ),
  );
}

/// 설치 폴더의 Legal 폴더 경로. Windows만 실행 파일 옆이라고 알 수 있고, 다른 OS는 null(문구만 보인다).
String? legalFolderPath() {
  if (defaultTargetPlatform != TargetPlatform.windows) return null;
  final exe = Platform.resolvedExecutable;
  final cut = exe.lastIndexOf(RegExp(r'[\\/]'));
  return '${cut < 0 ? '.' : exe.substring(0, cut)}\\$legalFolderName';
}

/// Credits 창 본문: 상표, 데이터 출처, 공개 소프트웨어 안내.
class CreditsText extends StatelessWidget {
  const CreditsText({super.key, this.legalFolder});

  /// 화면에 보일 Legal 폴더 경로(없으면 안 보인다).
  final String? legalFolder;

  @override
  Widget build(BuildContext context) {
    const heading = TextStyle(color: Colors.white, fontWeight: FontWeight.bold);
    const body = TextStyle(color: Colors.white70, height: 1.4);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        const Text('Trademarks', style: heading),
        const SizedBox(height: 4),
        const Text('$asioTrademarkAttribution\n$asioCopyrightNotice', style: body),
        const SizedBox(height: 16),
        const Text(kemarHrtfPackageName, style: heading),
        const SizedBox(height: 4),
        const Text(kemarHrtfNotice, style: body),
        const SizedBox(height: 16),
        const Text('Open-source software', style: heading),
        const SizedBox(height: 4),
        const Text(
          'This product includes open-source software. The full license texts are in the '
          '$legalFolderName folder of the installation folder.\n'
          '이 제품에 쓰인 공개 소프트웨어의 라이선스 전문은 설치 폴더의 $legalFolderName 폴더에 있습니다.',
          style: body,
        ),
        if (legalFolder != null) ...[
          const SizedBox(height: 4),
          SelectableText(legalFolder!, style: body),
        ],
      ],
    );
  }
}
