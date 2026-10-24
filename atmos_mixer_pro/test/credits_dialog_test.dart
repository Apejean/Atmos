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

  testWidgets('Credits는 글자 고지만 보인다: ASIO 상표 두 줄, KEMAR 출처, Legal 폴더 안내, 로고 없음', (tester) async {
    await openCredits(tester);

    expect(find.text('Credits'), findsOneWidget);
    expect(find.textContaining('ASIO is a registered trademark of Steinberg Media Technologies GmbH.'), findsOneWidget);
    expect(
      find.textContaining('ASIO is a trademark of Steinberg Media Technologies GmbH, registered in Europe and other countries.'),
      findsOneWidget,
    );
    expect(find.textContaining('Gardner, W. G., and Martin, K. D. (1995)'), findsOneWidget);
    expect(find.textContaining('Legal folder of the installation folder'), findsOneWidget);
    expect(find.byType(Image), findsNothing);
  });

  testWidgets('Windows에서는 실행 파일 옆 Legal 폴더 경로를 보인다', (tester) async {
    await openCredits(tester);

    final path = legalFolderPath();
    expect(path, endsWith(r'\Legal'));
    expect(find.text(path!), findsOneWidget);
  }, variant: TargetPlatformVariant.only(TargetPlatform.windows));

  testWidgets('Windows가 아니면 경로 줄이 없다', (tester) async {
    await openCredits(tester);

    expect(legalFolderPath(), isNull);
    expect(find.byType(SelectableText), findsNothing);
  }, variant: TargetPlatformVariant.only(TargetPlatform.macOS));
}
