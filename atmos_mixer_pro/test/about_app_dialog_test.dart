import 'package:atmos_mixer_pro/features/dashboard/widgets/about_app_dialog.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:package_info_plus/package_info_plus.dart';

PackageInfo _info(String version, String build) =>
    PackageInfo(appName: 'atmos_mixer_pro', packageName: 'atmos_mixer_pro', version: version, buildNumber: build);

/// Help → The Awesome: 설치된 앱의 버전을 보여 준다(2026-10-22 사용자 요청).
void main() {
  test('버전 문구는 빌드 번호가 없거나 0이면 뺀다', () {
    expect(aboutVersionText(_info('1.3.4', '')), '버전 1.3.4');
    expect(aboutVersionText(_info('1.3.4', '0')), '버전 1.3.4');
    expect(aboutVersionText(_info('1.3.4', '7')), '버전 1.3.4 (빌드 7)');
  });

  testWidgets('창에 회사 이름과 설치된 버전이 보이고 확인을 누르면 닫힌다', (tester) async {
    PackageInfo.setMockInitialValues(
      appName: 'atmos_mixer_pro',
      packageName: 'atmos_mixer_pro',
      version: '1.3.4',
      buildNumber: '',
      buildSignature: '',
    );
    await tester.pumpWidget(MaterialApp(
      home: Builder(
        builder: (context) => TextButton(onPressed: () => showAboutApp(context), child: const Text('open')),
      ),
    ));
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    expect(find.text('The Awesome'), findsOneWidget);
    expect(find.text('버전 1.3.4'), findsOneWidget);

    await tester.tap(find.text('확인'));
    await tester.pumpAndSettle();
    expect(find.text('버전 1.3.4'), findsNothing);
  });
}
