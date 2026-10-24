import 'package:atmos_mixer_pro/features/settings/widgets/credits_dialog.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
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
