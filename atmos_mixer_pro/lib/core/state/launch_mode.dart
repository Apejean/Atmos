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
