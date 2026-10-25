import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:flutter/material.dart';
import 'package:package_info_plus/package_info_plus.dart';

/// 설치된 앱의 버전을 보여 준다(Help → The Awesome, 2026-10-22 사용자 요청: 어느 버전이 깔렸는지 몰랐다).
/// 버전은 pubspec.yaml의 version이 빌드 때 실행 파일(Windows 버전 정보, macOS Info.plist)에 들어간 값이다.
Future<void> showAboutApp(BuildContext context) async {
  final info = await PackageInfo.fromPlatform();
  if (!context.mounted) return;
  await showDialog<void>(
    context: context,
    builder: (context) => AlertDialog(
      backgroundColor: AppColors.background,
      title: const Text('The Awesome', style: TextStyle(color: Colors.white)),
      content: Text(aboutVersionText(info), style: const TextStyle(color: Colors.white70)),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('확인', style: TextStyle(color: Colors.white70)),
        ),
      ],
    ),
  );
}

/// 창에 보일 버전 문구. 빌드 번호가 없거나 0이면(pubspec에 +빌드를 적지 않음) 빼고 보인다.
String aboutVersionText(PackageInfo info) {
  final build = info.buildNumber.trim();
  final hasBuild = build.isNotEmpty && build != '0';
  return '버전 ${info.version}${hasBuild ? ' (빌드 $build)' : ''}';
}
