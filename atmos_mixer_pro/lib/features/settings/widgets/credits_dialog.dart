import 'dart:io';

import 'package:atmos_mixer_pro/core/legal/legal_texts.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:package_info_plus/package_info_plus.dart';

/// Help → About Awesome...(2026-10-25 사용자 결정: 버전 창 "The Awesome"과 Credits를 하나로, Ableton Live처럼).
/// 맨 위에 로고처럼 큰 "The Awesome"과 그 아래 작은 설치 버전, 그 밑에 Credits 글자 고지를 둔다. 공개 소프트웨어
/// 라이선스 전문, EULA와 ASIO 로고는 설치 폴더 Legal 폴더의 문서에 두고, Windows에서는 "Legal information"을
/// 누르면 탐색기로 그 폴더를 연다. 로고 그림은 여기 두지 않는다(ASIO 지침 1절 f의 로고는 ASIO를 고르는 환경설정에 있다).
/// About 창이 이것 하나라 ASIO 계약 3조 1항 c(제3자를 언급하는 모든 About 창에 고지)를 이 창이 채운다.
Future<void> showCredits(BuildContext context) async {
  final info = await PackageInfo.fromPlatform();
  if (!context.mounted) return;
  await showDialog<void>(
    context: context,
    builder: (context) => AlertDialog(
      backgroundColor: AppColors.background,
      content: SizedBox(
        width: 560,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisSize: MainAxisSize.min,
            children: [
              const Text(
                'The Awesome',
                textAlign: TextAlign.center,
                style: TextStyle(color: Colors.white, fontSize: 34, fontWeight: FontWeight.w800, letterSpacing: 1.2),
              ),
              const SizedBox(height: 4),
              Text(
                aboutVersionText(info),
                textAlign: TextAlign.center,
                style: const TextStyle(color: Colors.white60, fontSize: 12),
              ),
              const SizedBox(height: 16),
              const Divider(color: Colors.white24),
              const SizedBox(height: 12),
              const Text('Credits', style: TextStyle(color: Colors.white, fontSize: 16, fontWeight: FontWeight.bold)),
              const SizedBox(height: 12),
              CreditsText(legalFolder: legalFolderPath()),
            ],
          ),
        ),
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

/// 창에 보일 버전 문구(설치된 앱의 버전, 2026-10-22 사용자 요청: 어느 버전이 깔렸는지 몰랐다). 버전은 pubspec.yaml의
/// version이 빌드 때 실행 파일(Windows 버전 정보, macOS Info.plist)에 들어간 값이다. 빌드 번호가 없거나 0이면
/// (pubspec에 +빌드를 적지 않음) 빼고 보인다.
String aboutVersionText(PackageInfo info) {
  final build = info.buildNumber.trim();
  final hasBuild = build.isNotEmpty && build != '0';
  return '버전 ${info.version}${hasBuild ? ' (빌드 $build)' : ''}';
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
        // ASIO 상표 두 줄은 Windows에서만(2026-10-25 사용자 결정): macOS 판은 ASIO를 쓰지 않고, ASIO 계약 3조 1항 g는
        // ASIO와 호환되지 않는 제품에 ASIO 상표를 쓰지 않게 한다(환경설정 로고와 같은 기준).
        if (defaultTargetPlatform == TargetPlatform.windows) ...[
          const Text('Trademarks', style: heading),
          const SizedBox(height: 4),
          const Text('$asioTrademarkAttribution\n$asioCopyrightNotice', style: body),
          const SizedBox(height: 16),
        ],
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
