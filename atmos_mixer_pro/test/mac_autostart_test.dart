// macOS 로그인 자동 실행(2026-11-28 사용자 결정 "감시 프로그램 경유"): 환경설정 "macOS 시작 시 자동 실행"을 켜면
// 사용자 LaunchAgent가 로그인 때 앱 번들 안의 감시 프로그램을 --logon으로 띄운다(Windows 작업 스케줄러 → 감시 → 앱과 같음).
import 'dart:io';

import 'package:atmos_mixer_pro/core/state/launch_mode.dart';
import 'package:flutter_test/flutter_test.dart';

const _installed = '/Applications/The Awesome.app/Contents/MacOS/atmos_mixer_pro';

void main() {
  test('/Applications에 설치된 앱만 자동 실행을 등록할 수 있다', () {
    expect(macSupervisorFor(_installed), '/Applications/The Awesome.app/Contents/MacOS/atmos_supervisor');
    // 개발 빌드·시험 복사본은 로그인 항목을 남기지 않는다.
    expect(macSupervisorFor('/Users/me/atmos/build/macos/Build/Products/Release/atmos_mixer_pro.app/Contents/MacOS/atmos_mixer_pro'),
        isNull);
    expect(macSupervisorFor('/Applications/Other/x'), isNull);
  });

  test('LaunchAgent는 감시 프로그램을 --logon으로 로그인 때 한 번 띄운다', () {
    final plist = macAutostartPlist('/Applications/The Awesome.app/Contents/MacOS/atmos_supervisor');
    expect(plist, contains('<string>$macAutostartLabel</string>'));
    expect(plist, contains('<string>/Applications/The Awesome.app/Contents/MacOS/atmos_supervisor</string>'));
    expect(plist, contains('<string>--logon</string>'));
    expect(plist, contains('<key>RunAtLoad</key>\n  <true/>'));
    // 감시가 끝나면(운영자가 앱을 닫음) launchd가 다시 띄우지 않는다.
    expect(plist, isNot(contains('KeepAlive')));
  });

  test('켜면 LaunchAgent 파일을 만들고 끄면 지운다(기본은 꺼짐)', () async {
    final home = await Directory.systemTemp.createTemp('mac_autostart_');
    addTearDown(() => home.delete(recursive: true));
    expect(await loadMacAutostart(home.path), isFalse);

    expect(await saveMacAutostart(home.path, _installed, true), isTrue);
    final file = File(macAutostartPlistPath(home.path));
    expect(file.existsSync(), isTrue);
    expect(file.readAsStringSync(), contains('atmos_supervisor'));
    expect(await loadMacAutostart(home.path), isTrue);

    expect(await saveMacAutostart(home.path, _installed, false), isTrue);
    expect(file.existsSync(), isFalse);
    expect(await loadMacAutostart(home.path), isFalse);
  });

  test('설치되지 않은 앱에서는 켜지 않는다', () async {
    final home = await Directory.systemTemp.createTemp('mac_autostart_');
    addTearDown(() => home.delete(recursive: true));
    expect(await saveMacAutostart(home.path, '/tmp/x.app/Contents/MacOS/atmos_mixer_pro', true), isFalse);
    expect(File(macAutostartPlistPath(home.path)).existsSync(), isFalse);
  });
}
