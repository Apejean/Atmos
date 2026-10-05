import 'package:atmos_mixer_pro/core/utils/log_export_dir.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('로그 내보내기 바탕화면 경로', () {
    test('Windows: 셸이 아는 실제 바탕화면(OneDrive로 옮겨진 경우 포함)을 먼저 쓴다', () async {
      final dir = await resolveDesktopDir(
        isWindows: true,
        env: {'USERPROFILE': r'C:\Users\op'},
        windowsShellDesktop: () async => r'C:\Users\op\OneDrive\Desktop' '\r\n',
        dirExists: (p) => p == r'C:\Users\op\OneDrive\Desktop',
      );
      expect(dir, r'C:\Users\op\OneDrive\Desktop');
    });

    test('Windows: 셸에 물을 수 없으면 USERPROFILE\\Desktop', () async {
      final dir = await resolveDesktopDir(
        isWindows: true,
        env: {'USERPROFILE': r'C:\Users\op'},
        windowsShellDesktop: () async => null,
        dirExists: (p) => p == r'C:\Users\op\Desktop',
      );
      expect(dir, r'C:\Users\op\Desktop');
    });

    test('Windows: 어느 바탕화면도 없으면 null(호출 쪽이 폴더를 묻는다)', () async {
      final dir = await resolveDesktopDir(
        isWindows: true,
        env: {'USERPROFILE': r'C:\Users\op'},
        windowsShellDesktop: () async => r'C:\Users\op\OneDrive\Desktop',
        dirExists: (_) => false,
      );
      expect(dir, isNull);
    });

    test('macOS: HOME/Desktop, 없으면 null', () async {
      Future<String?> resolve(bool exists) => resolveDesktopDir(
            isWindows: false,
            env: {'HOME': '/Users/op'},
            windowsShellDesktop: () async => fail('macOS에서 Windows 셸을 부르면 안 된다'),
            dirExists: (p) => exists && p == '/Users/op/Desktop',
          );
      expect(await resolve(true), '/Users/op/Desktop');
      expect(await resolve(false), isNull);
    });
  });
}
