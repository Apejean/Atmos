import 'package:atmos_mixer_pro/features/settings/widgets/asio_setting_notice.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('환경설정 ASIO 고지는 흰 로고(원본 비율)와 사용 지침 15절 문구를 한 줄에 보인다', (tester) async {
    await tester.pumpWidget(const MaterialApp(home: Scaffold(body: AsioSettingNotice())));

    expect(find.image(const AssetImage(asioLogoWhiteAsset)), findsOneWidget);
    expect(find.text('ASIO is a registered trademark of Steinberg Media Technologies GmbH.'), findsOneWidget);

    // 로고는 늘이거나 자르지 않는다: 원본 452×288 비율 그대로.
    final logo = tester.getSize(find.byType(Image));
    expect(logo.height, asioSettingLogoHeight);
    expect(logo.width / logo.height, closeTo(452 / 288, 0.001));

    // 문구는 로고 오른쪽에 있다.
    expect(tester.getTopLeft(find.byType(Text)).dx, greaterThan(tester.getTopRight(find.byType(Image)).dx));

    // 앱 묶음에 원본 파일(14,702바이트)이 그대로 들어간다.
    final bytes = await tester.runAsync(() => rootBundle.load(asioLogoWhiteAsset));
    expect(bytes!.lengthInBytes, 14702);
  });
}
