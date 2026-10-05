import 'dart:io';

/// 로그를 내보낼 바탕화면 폴더를 찾는다. 찾지 못하면 null — 호출 쪽이 폴더를 묻는다.
///
/// Windows는 바탕화면이 OneDrive 아래로 옮겨진 PC가 많아 `USERPROFILE\Desktop`을 그대로 쓰면
/// 없는 폴더에 저장하려다 실패했다. 셸이 아는 실제 바탕화면([windowsShellDesktop])을 먼저 쓴다.
Future<String?> resolveDesktopDir({
  required bool isWindows,
  required Map<String, String> env,
  required Future<String?> Function() windowsShellDesktop,
  required bool Function(String path) dirExists,
}) async {
  final candidates = <String?>[
    if (isWindows) await windowsShellDesktop(),
    if (isWindows)
      env['USERPROFILE'] == null ? null : '${env['USERPROFILE']}\\Desktop'
    else
      env['HOME'] == null ? null : '${env['HOME']}/Desktop',
  ];
  for (final candidate in candidates) {
    final path = candidate?.trim();
    if (path != null && path.isNotEmpty && dirExists(path)) return path;
  }
  return null;
}

/// 앱에서 쓰는 형태. Windows는 PowerShell로 .NET에 바탕화면 경로를 묻는다.
Future<String?> desktopDirForLogExport() => resolveDesktopDir(
      isWindows: Platform.isWindows,
      env: Platform.environment,
      windowsShellDesktop: () async {
        try {
          final r = await Process.run(
            'powershell',
            ['-NoProfile', '-NonInteractive', '-Command', "[Environment]::GetFolderPath('Desktop')"],
          );
          return r.exitCode == 0 ? r.stdout as String : null;
        } catch (_) {
          return null;
        }
      },
      dirExists: (path) => Directory(path).existsSync(),
    );
