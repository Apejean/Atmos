import 'dart:io';

import 'package:atmos_mixer_pro/core/state/launch_mode.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  group('공연 이어 가기 결정', () {
    test('감시가 충돌·멈춤 뒤 다시 띄웠으면 환경설정과 상관없이 이어 간다', () {
      expect(shouldResumeShow(args: ['--supervised', '--auto-relaunched'], resumeOnLogon: false), isTrue);
    });

    test('로그인 자동 실행은 환경설정을 따른다', () {
      expect(shouldResumeShow(args: ['--supervised', '--logon'], resumeOnLogon: true), isTrue);
      expect(shouldResumeShow(args: ['--supervised', '--logon'], resumeOnLogon: false), isFalse);
    });

    test('사람이 켰으면 대기한다', () {
      expect(shouldResumeShow(args: const [], resumeOnLogon: true), isFalse);
      expect(shouldResumeShow(args: ['--supervised'], resumeOnLogon: true), isFalse);
    });
  });

  group('환경설정 "로그인할 때 공연 자동 시작"', () {
    test('기본은 켜짐이고 끈 값은 남는다', () async {
      SharedPreferences.setMockInitialValues({});
      expect(await loadAutoResumeOnLogon(), isTrue);
      await saveAutoResumeOnLogon(false);
      expect(await loadAutoResumeOnLogon(), isFalse);
      await saveAutoResumeOnLogon(true);
      expect(await loadAutoResumeOnLogon(), isTrue);
    });
  });

  group('환경설정 "Windows 시작 시 자동 실행"', () {
    late Directory dir;

    setUp(() => dir = Directory.systemTemp.createTempSync('atmos_autostart'));
    tearDown(() => dir.deleteSync(recursive: true));

    // 이름은 감시 프로그램(supervisor/src/run.rs)이 보는 표시 파일과 같아야 한다.
    File marker() => File('${dir.path}${Platform.pathSeparator}windows_autostart_off');

    test('표시 파일이 없으면 켜짐이다(업데이트해도 지금 동작 그대로)', () async {
      expect(await loadWindowsAutostart(dir.path), isTrue);
    });

    test('끄면 표시 파일이 생기고 다시 켜면 없어진다', () async {
      await saveWindowsAutostart(dir.path, false);
      expect(marker().existsSync(), isTrue);
      expect(await loadWindowsAutostart(dir.path), isFalse);

      await saveWindowsAutostart(dir.path, true);
      expect(marker().existsSync(), isFalse);
      expect(await loadWindowsAutostart(dir.path), isTrue);
    });

    test('같은 값을 두 번 저장해도 된다', () async {
      await saveWindowsAutostart(dir.path, true);
      await saveWindowsAutostart(dir.path, false);
      await saveWindowsAutostart(dir.path, false);
      expect(await loadWindowsAutostart(dir.path), isFalse);
    });
  });

  test('OSC 시작 신호의 시작 종류는 실행 인자로 정한다', () {
    expect(bootStartKind(['--supervised', '--auto-relaunched']), 'relaunch');
    expect(bootStartKind(['--logon']), 'logon');
    expect(bootStartKind(const []), 'manual');
  });
}
