import 'package:atmos_mixer_pro/features/settings/widgets/third_party_notices.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Third-party Notices 화면에서 KEMAR 항목을 열면 저작권·인용이 보인다', (tester) async {
    // 테스트 바인딩이 넣은 NOTICES 수집기를 빼고 앱의 고지만 남긴다.
    LicenseRegistry.reset();
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => TextButton(
            onPressed: () => showThirdPartyNotices(context),
            child: const Text('open'),
          ),
        ),
      ),
    );

    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    expect(find.text(kemarHrtfPackageName), findsOneWidget);

    await tester.tap(find.text(kemarHrtfPackageName));
    await tester.pumpAndSettle();
    expect(find.textContaining('Copyright 1994 by the MIT Media Laboratory'), findsOneWidget);
    expect(find.textContaining('Gardner, W. G., and Martin, K. D. (1995)'), findsOneWidget);
  });
}
