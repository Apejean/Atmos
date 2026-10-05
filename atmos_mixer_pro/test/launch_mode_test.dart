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
}
