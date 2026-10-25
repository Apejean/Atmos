import 'package:atmos_mixer_pro/features/settings/widgets/credits_dialog.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:package_info_plus/package_info_plus.dart';

PackageInfo _info(String version, String build) =>
    PackageInfo(appName: 'atmos_mixer_pro', packageName: 'atmos_mixer_pro', version: version, buildNumber: build);

/// Help → About Awesome...: 큰 "The Awesome"과 설치 버전, 그 밑에 Credits(2026-10-25 사용자 결정).
void main() {
  setUp(() {
    PackageInfo.setMockInitialValues(
      appName: 'atmos_mixer_pro',
      packageName: 'atmos_mixer_pro',
      version: '1.3.4',
      buildNumber: '',
      buildSignature: '',
    );
  });

  test('버전 문구는 빌드 번호가 없거나 0이면 뺀다', () {
    expect(aboutVersionText(_info('1.3.4', '')), '버전 1.3.4');
    expect(aboutVersionText(_info('1.3.4', '0')), '버전 1.3.4');
    expect(aboutVersionText(_info('1.3.4', '7')), '버전 1.3.4 (빌드 7)');
  });

  Future<void> openCredits(WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => TextButton(onPressed: () => showCredits(context), child: const Text('open')),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
  }

  testWidgets('맨 위에 로고처럼 큰 The Awesome, 그 아래 작은 설치 버전, 그 밑에 Credits가 있고 확인을 누르면 닫힌다',
      (tester) async {
    await openCredits(tester);

    final title = find.text('The Awesome');
    final version = find.text('버전 1.3.4');
    expect(title, findsOneWidget);
    expect(version, findsOneWidget);
    expect(tester.getTopLeft(title).dy, lessThan(tester.getTopLeft(version).dy));
    expect(tester.getTopLeft(version).dy, lessThan(tester.getTopLeft(find.text('Credits')).dy));
    expect(tester.widget<Text>(title).style!.fontSize, greaterThan(tester.widget<Text>(version).style!.fontSize!));

    await tester.tap(find.text('확인'));
    await tester.pumpAndSettle();
    expect(version, findsNothing);
  });

  testWidgets('Credits는 글자 고지만 보인다: ASIO 두 줄(지침 15절·계약 3조 1항 k), KEMAR 출처, 안내 문장·경로·로고 없음', (tester) async {
    await openCredits(tester);

    expect(find.text('Credits'), findsOneWidget);
    expect(find.textContaining('ASIO is a registered trademark of Steinberg Media Technologies GmbH.'), findsOneWidget);
    expect(
      find.textContaining('ASIO is a trademark of Steinberg Media Technologies GmbH, registered in Europe and other countries.'),
      findsOneWidget,
    );
    expect(find.textContaining('Gardner, W. G., and Martin, K. D. (1995)'), findsOneWidget);
    expect(find.textContaining('installation folder'), findsNothing);
    expect(find.textContaining('설치 폴더'), findsNothing);
    expect(find.byType(SelectableText), findsNothing);
    expect(find.byType(Image), findsNothing);
  });

  testWidgets('Windows에서는 실행 파일 옆 Legal 폴더를 여는 Legal information이 보인다', (tester) async {
    await openCredits(tester);

    expect(legalFolderPath(), endsWith(r'\Legal'));
    expect(find.text('Legal information'), findsOneWidget);
  }, variant: TargetPlatformVariant.only(TargetPlatform.windows));

  testWidgets('Legal information은 받은 함수로 그 폴더를 열고, 폴더가 없으면 알린다', (tester) async {
    final opened = <String>[];
    var exists = true;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: CreditsText(
              legalFolder: r'C:\App\Legal',
              openFolder: (path) async {
                opened.add(path);
                return exists;
              },
            ),
          ),
        ),
      ),
    );

    await tester.ensureVisible(find.text('Legal information'));
    await tester.tap(find.text('Legal information'));
    await tester.pump();
    expect(opened, [r'C:\App\Legal']);
    expect(find.textContaining('Legal 폴더를 찾지 못했습니다'), findsNothing);

    exists = false;
    await tester.tap(find.text('Legal information'));
    await tester.pump();
    expect(find.text(r'Legal 폴더를 찾지 못했습니다: C:\App\Legal'), findsOneWidget);
  });

  testWidgets('Windows가 아니면 Legal information이 없다', (tester) async {
    await openCredits(tester);

    expect(legalFolderPath(), isNull);
    expect(find.text('Legal information'), findsNothing);
  }, variant: TargetPlatformVariant.only(TargetPlatform.macOS));
}
