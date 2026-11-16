import 'dart:io';
import 'package:flutter_test/flutter_test.dart';

/// 설계 데이터 파일 검사(apiCheckPrefsFile, 검토 A3)는 앱이 SharedPreferences를 처음 읽기 전에 돌아야 한다.
/// 플러그인은 처음 읽은 맵을 메모리에 두므로, 순서가 틀리면 되살린 파일을 보지 못하고 다음 저장이 빈 상태를 굳힌다.
/// 소스 감사로 이 순서를 고정한다.
void main() {
  // 주석은 빼고 코드만 본다(주석에 SharedPreferences라는 말이 있어도 읽기가 아니다).
  final mainSrc = File('lib/main.dart')
      .readAsLinesSync()
      .where((l) => !l.trimLeft().startsWith('//'))
      .join('\n');

  test('main.dart: 검사는 runApp·로고 미리 읽기보다 앞이고, 그 앞에는 SharedPreferences가 없다', () {
    final check = mainSrc.indexOf('apiCheckPrefsFile(');
    expect(check, greaterThan(0));
    expect(check, lessThan(mainSrc.indexOf('runApp(')));
    expect(check, lessThan(mainSrc.indexOf('await precacheSplashLogo()')));
    expect(mainSrc.substring(0, check).contains('SharedPreferences'), isFalse);
  });

  test('검사 앞에서 부르는 함수들은 SharedPreferences를 쓰지 않는다', () {
    // main.dart에서 검사 앞의 await 대상: RustLib.init, apiStartupGate, apiStartShowState, showStateDir.
    final launch = File('lib/core/state/launch_mode.dart').readAsStringSync();
    final showStateDir = RegExp(r'Future<String> showStateDir\(\)[^\n]*').firstMatch(launch)!.group(0)!;
    expect(showStateDir.contains('SharedPreferences'), isFalse);
  });

  test('lib 어디에도 SharedPreferences를 최상위 변수 초기화로 읽는 곳이 없다', () {
    final offenders = <String>[];
    for (final f in Directory('lib').listSync(recursive: true).whereType<File>()) {
      if (!f.path.endsWith('.dart') || f.path.contains('${Platform.pathSeparator}src${Platform.pathSeparator}rust')) {
        continue;
      }
      for (final line in f.readAsLinesSync()) {
        // 들여쓰기 없는 최상위 선언에서 바로 읽으면(앱이 시작되며 평가될 수 있음) 검사보다 먼저일 수 있다.
        if (RegExp(r'^(final|var|late|const)\b.*SharedPreferences').hasMatch(line)) offenders.add('${f.path}: $line');
      }
    }
    expect(offenders, isEmpty);
  });
}
