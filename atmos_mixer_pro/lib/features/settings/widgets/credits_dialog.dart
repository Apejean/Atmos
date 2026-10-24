import 'dart:io';

import 'package:atmos_mixer_pro/core/legal/legal_texts.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

/// Help → Credits(2026-10-25 사용자 결정: Ableton Live처럼). 앱 안에는 매뉴얼 Credits 같은 글자 고지만 두고,
/// 공개 소프트웨어 라이선스 전문, EULA와 ASIO 로고는 설치 폴더 Legal 폴더의 문서에 둔다. Windows에서는
/// "Legal information"을 누르면 탐색기로 그 폴더를 연다. 로고는 여기 두지 않는다(ASIO 지침 1절 f의 로고는
/// ASIO를 고르는 환경설정에 있다).
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

/// Windows 탐색기로 폴더를 연다. 폴더가 없으면 false(탐색기는 없는 경로를 받으면 다른 폴더를 연다).
Future<bool> openFolderInExplorer(String path) async {
  if (!Directory(path).existsSync()) return false;
  await Process.start('explorer.exe', [path]);
  return true;
}

/// Credits 창 본문: 상표, 데이터 출처, 공개 소프트웨어 안내와 Legal 폴더 열기.
class CreditsText extends StatefulWidget {
  const CreditsText({super.key, this.legalFolder, this.openFolder = openFolderInExplorer});

  /// 열 Legal 폴더 경로(없으면 "Legal information"이 안 보인다).
  final String? legalFolder;

  /// 폴더를 여는 함수(시험에서 바꿔 끼운다).
  final Future<bool> Function(String path) openFolder;

  @override
  State<CreditsText> createState() => _CreditsTextState();
}

class _CreditsTextState extends State<CreditsText> {
  String? _missing;

  Future<void> _open() async {
    final path = widget.legalFolder!;
    final opened = await widget.openFolder(path);
    if (mounted) setState(() => _missing = opened ? null : path);
  }

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
        // 안내 문장·경로 없이 링크 하나만 둔다(2026-10-25 사용자 요청). 공개 소프트웨어 라이선스 전문과 EULA는 그 폴더에 있다.
        if (widget.legalFolder != null) ...[
          const SizedBox(height: 16),
          TextButton(
            onPressed: _open,
            style: TextButton.styleFrom(padding: EdgeInsets.zero, alignment: Alignment.centerLeft),
            child: const Text(
              'Legal information',
              style: TextStyle(
                color: AppColors.primaryNeon,
                decoration: TextDecoration.underline,
                decorationColor: AppColors.primaryNeon,
              ),
            ),
          ),
          if (_missing != null) Text('Legal 폴더를 찾지 못했습니다: $_missing', style: const TextStyle(color: Colors.orangeAccent)),
        ],
      ],
    );
  }
}
