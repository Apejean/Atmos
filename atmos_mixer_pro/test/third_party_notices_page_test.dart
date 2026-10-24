import 'package:atmos_mixer_pro/features/settings/widgets/third_party_notices.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
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

  testWidgets('Windows에서는 화면 맨 아래에 ASIO 로고(원본 비율)와 상표·고지 문구가 보인다', (tester) async {
    LicenseRegistry.reset();
    await _openNotices(tester);

    final notice = find.byType(AsioTrademarkNotice);
    expect(notice, findsOneWidget);
    expect(find.image(const AssetImage(asioLogoAsset)), findsOneWidget);
    expect(find.text(asioTrademarkAttribution), findsOneWidget);
    expect(find.text(asioCopyrightNotice), findsOneWidget);

    // 맨 아래: 라이선스 목록 아래에 붙어 화면 바닥까지 닿는다.
    final page = tester.getRect(find.byType(ThirdPartyNoticesPage));
    final noticeRect = tester.getRect(notice);
    expect(noticeRect.bottom, page.bottom);
    expect(tester.getRect(find.byType(LicensePage)).bottom, lessThanOrEqualTo(noticeRect.top));

    // 로고는 늘이거나 자르지 않는다: 원본 474×443 비율 그대로.
    final logo = tester.getSize(find.descendant(of: notice, matching: find.byType(Image)));
    expect(logo.width, asioLogoWidth);
    expect(logo.width / logo.height, closeTo(474 / 443, 0.001));

    // 앱 묶음에 원본 파일(25,830바이트)이 그대로 들어간다.
    final bytes = await tester.runAsync(() => rootBundle.load(asioLogoAsset));
    expect(bytes!.lengthInBytes, 25830);
  }, variant: TargetPlatformVariant.only(TargetPlatform.windows));

  testWidgets('Windows가 아니면(ASIO 없음) ASIO 고지 칸이 없다', (tester) async {
    LicenseRegistry.reset();
    await _openNotices(tester);

    expect(find.byType(LicensePage), findsOneWidget);
    expect(find.byType(AsioTrademarkNotice), findsNothing);
  }, variant: TargetPlatformVariant.only(TargetPlatform.macOS));
}

Future<void> _openNotices(WidgetTester tester) async {
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
}
