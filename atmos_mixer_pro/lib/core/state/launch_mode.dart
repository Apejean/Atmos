import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// 앱 실행 인자. main이 받은 값으로 덮어쓴다(감시 프로그램이 --supervised 등을 넘긴다).
final launchArgsProvider = Provider<List<String>>((ref) => const []);

/// 환경설정 "로그인할 때 공연 자동 시작" 키. 이 컴퓨터의 운영 설정이라 프로젝트 파일에 넣지 않는다.
const kAutoResumeOnLogonKey = 'auto_resume_on_logon';

/// 앱 시작 절차 뒤 공연을 이어 갈지 정한다.
/// - 감시 프로그램이 충돌·멈춤 뒤 다시 띄웠다(--auto-relaunched): 항상 이어 간다.
/// - 로그인 자동 실행(--logon): 환경설정이 켜져 있을 때만 이어 간다.
/// - 사람이 바로가기로 켰다: 지금처럼 대기한다.
bool shouldResumeShow({required List<String> args, required bool resumeOnLogon}) {
  if (args.contains('--auto-relaunched')) return true;
  if (args.contains('--logon')) return resumeOnLogon;
  return false;
}

/// OSC 시작 신호(`/awesome/status/boot`)의 시작 종류(규격 §3.2): 감시가 다시 띄움 relaunch, 로그인 자동 실행 logon,
/// 그 밖(사람이 켬) manual.
String bootStartKind(List<String> args) {
  if (args.contains('--auto-relaunched')) return 'relaunch';
  if (args.contains('--logon')) return 'logon';
  return 'manual';
}

/// "로그인할 때 공연 자동 시작"(기본 켜짐). 점검하러 재부팅할 때 끈다.
Future<bool> loadAutoResumeOnLogon() async {
  final prefs = await SharedPreferences.getInstance();
  return prefs.getBool(kAutoResumeOnLogonKey) ?? true;
}

Future<void> saveAutoResumeOnLogon(bool value) async {
  final prefs = await SharedPreferences.getInstance();
  await prefs.setBool(kAutoResumeOnLogonKey, value);
}

/// 공연 상태 파일(show_state.json)을 두는 앱 지원 폴더. 설정 파일과 같은 곳이다.
Future<String> showStateDir() async => (await getApplicationSupportDirectory()).path;

/// 환경설정 "Windows 시작 시 자동 실행"을 끈 표시 파일. [dir]는 앱 지원 폴더([showStateDir])다. 파일이 없으면
/// 켜짐이라 업데이트해도 지금 동작 그대로다. 로그인 작업으로 시작한 감시 프로그램이 이 파일을 보면 앱을 띄우지
/// 않고 끝난다 — 이름은 감시 프로그램의 `windows_autostart_off`(supervisor/src/run.rs)와 같아야 한다.
File _windowsAutostartOffFile(String dir) => File('$dir${Platform.pathSeparator}windows_autostart_off');

Future<bool> loadWindowsAutostart(String dir) async => !await _windowsAutostartOffFile(dir).exists();

Future<void> saveWindowsAutostart(String dir, bool value) async {
  final marker = _windowsAutostartOffFile(dir);
  if (!value) {
    await marker.create(recursive: true);
  } else if (await marker.exists()) {
    await marker.delete();
  }
}

/// macOS 로그인 자동 실행 LaunchAgent 이름(2026-11-28 사용자 결정 "감시 프로그램 경유"). 로그인 때 앱 번들 안의 감시
/// 프로그램(`Contents/MacOS/atmos_supervisor`, 릴리스 빌드 단계 `macos/Runner/add_supervisor.sh`)을 `--logon`으로 띄우고,
/// 감시가 같은 폴더의 앱을 띄운다. Windows의 작업 스케줄러 → 감시 → 앱과 같다. 켜고 끄는 것은 이 파일이 있는지다.
const macAutostartLabel = 'com.theawesome.atmos.autostart';

/// 사용자 LaunchAgent 파일 경로. [home]은 사용자 홈 폴더다.
String macAutostartPlistPath(String home) => '$home/Library/LaunchAgents/$macAutostartLabel.plist';

/// 앱 실행 파일 [executable]이 /Applications에 설치된 앱 번들 안에 있으면 같은 폴더의 감시 프로그램 경로, 아니면 null.
/// 개발 빌드·시험 복사본이 로그인 항목을 남기지 않게 설치된 앱만 등록한다.
String? macSupervisorFor(String executable) {
  final match = RegExp(r'^/Applications/[^/]+\.app/Contents/MacOS/[^/]+$').hasMatch(executable);
  if (!match) return null;
  return '${executable.substring(0, executable.lastIndexOf('/'))}/atmos_supervisor';
}

String _xmlEscape(String s) =>
    s.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');

/// LaunchAgent 내용. 로그인(GUI 세션) 때 한 번 띄우고, 감시가 끝나면(운영자가 앱을 닫음) 다시 띄우지 않는다.
String macAutostartPlist(String supervisor) => '''<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>$macAutostartLabel</string>
  <key>ProgramArguments</key>
  <array>
    <string>${_xmlEscape(supervisor)}</string>
    <string>--logon</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>LimitLoadToSessionType</key>
  <string>Aqua</string>
  <key>ProcessType</key>
  <string>Interactive</string>
</dict>
</plist>
''';

/// "macOS 시작 시 자동 실행"이 켜져 있는가(LaunchAgent 파일이 있음). 기본은 꺼짐이다 — Mac은 설계용이라 사람이 켠다.
Future<bool> loadMacAutostart(String home) => File(macAutostartPlistPath(home)).exists();

/// 켜면 LaunchAgent 파일을 쓰고 끄면 지운다. 앱이 /Applications에 설치되지 않았으면 켜지 않고 false.
Future<bool> saveMacAutostart(String home, String executable, bool value) async {
  final file = File(macAutostartPlistPath(home));
  if (!value) {
    if (await file.exists()) await file.delete();
    return true;
  }
  final supervisor = macSupervisorFor(executable);
  if (supervisor == null) return false;
  await file.parent.create(recursive: true);
  await file.writeAsString(macAutostartPlist(supervisor));
  return true;
}
