import 'package:atmos_mixer_pro/core/state/config_notice.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/config_notice_banner.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('알림이 있어도 시작 절차는 사람의 입력 없이 이어진다', () async {
    final shown = <String?>[];
    var startupContinued = false;
    // 시작 절차의 한 단계처럼 부르고, 확인을 기다리지 않고 다음 단계로 넘어가는지 본다.
    await loadConfigNotice(fetch: () async => '설정을 되살렸습니다', show: shown.add)
        .timeout(const Duration(seconds: 1));
    startupContinued = true;
    expect(shown, ['설정을 되살렸습니다']);
    expect(startupContinued, isTrue);
  });

  test('안내를 못 가져와도 시작 절차를 멈추지 않는다', () async {
    await loadConfigNotice(fetch: () async => throw StateError('ffi'), show: (_) => fail('보이면 안 된다'))
        .timeout(const Duration(seconds: 1));
  });

  testWidgets('배너는 안내가 있을 때만 보이고 확인을 누르면 닫힌다', (tester) async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    await tester.pumpWidget(UncontrolledProviderScope(
      container: container,
      child: const MaterialApp(home: Scaffold(body: ConfigNoticeBanner())),
    ));
    expect(find.text('확인'), findsNothing);

    container.read(configNoticeProvider.notifier).show(null);
    await tester.pump();
    expect(find.text('확인'), findsNothing);

    container.read(configNoticeProvider.notifier).show('설정 파일이 손상되어 직전 정상본으로 되살렸습니다.');
    await tester.pump();
    expect(find.textContaining('직전 정상본'), findsOneWidget);

    await tester.tap(find.text('확인'));
    await tester.pump();
    expect(find.textContaining('직전 정상본'), findsNothing);
  });
}
