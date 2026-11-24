import 'package:atmos_mixer_pro/features/dashboard/widgets/track_routing_dialog.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

/// 트랙 연결 표 창(매트릭스 라우팅 설계 7.2·8.3, 개정 3).
void main() {
  TrackRoute r(int f, int o, [double g = 1.0]) => TrackRoute(fileChannel: f, outputChannel: o, gain: g);

  final names = [for (var i = 0; i < 6; i++) 'Analog ${i + 1}'];

  Future<List<(List<TrackRoute>, bool)>> open(
    WidgetTester tester, {
    required List<TrackRoute> routes,
    List<bool>? enabled,
    bool limiterOn = true,
    int? fileChannels = 1,
  }) async {
    await tester.binding.setSurfaceSize(const Size(1024, 768));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final calls = <(List<TrackRoute>, bool)>[];
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: TrackRoutingDialog(
          title: '방 1 · 1번 트랙',
          fileChannels: fileChannels,
          initialRoutes: routes,
          channelNames: names,
          enabled: enabled ?? List.filled(6, true),
          limiterOn: limiterOn,
          seedRoutes: () => [r(0, 0)],
          onChanged: (rs, {required commit}) => calls.add((rs, commit)),
        ),
      ),
    ));
    await tester.pump();
    return calls;
  }

  testWidgets('빈 칸을 누르면 100% 칸이 더해진 표가 확정으로 온다', (tester) async {
    final calls = await open(tester, routes: [r(0, 0)]);
    await tester.tap(find.byKey(const Key('route_cell_0_2')));
    await tester.pump();
    expect(calls.last.$2, isTrue);
    expect(calls.last.$1, [r(0, 0), r(0, 2)]);
    for (final key in ['route_cell_0_0', 'route_cell_0_2']) {
      expect(find.descendant(of: find.byKey(Key(key)), matching: find.text('100%')), findsOneWidget, reason: key);
    }
  });

  testWidgets('마지막 칸은 끊을 수 없다(끊기 단추가 꺼지고 이유를 보인다)', (tester) async {
    await open(tester, routes: [r(0, 1)]);
    await tester.tap(find.byKey(const Key('route_cell_0_1')));
    await tester.pump();
    final button = tester.widget<TextButton>(find.byKey(const Key('route_disconnect_button')));
    expect(button.onPressed, isNull);
    expect(find.textContaining('연결이 하나는 있어야 합니다'), findsWidgets);
  });

  testWidgets('닫힌 출력을 가리키는 칸은 주황 열로 따로 보인다', (tester) async {
    await open(tester, routes: [r(0, 0), r(0, 3)], enabled: [true, true, true, false, true, true]);
    expect(find.text('Ch-4 (닫힘)'), findsOneWidget);
  });

  testWidgets('한 출력의 합이 100%를 넘으면 합 경고, 리미터가 꺼져 있으면 리미터 경고', (tester) async {
    await open(tester, routes: [r(0, 1), r(1, 1, 0.5)], fileChannels: 2, limiterOn: false);
    expect(find.text('합 150%'), findsOneWidget);
    expect(find.textContaining('피크 리미터가 꺼져 있습니다'), findsOneWidget);
  });

  testWidgets('? 단추로 도움말이 펼쳐지고 접히며, 펼친 채로도 칸을 누를 수 있다', (tester) async {
    final calls = await open(tester, routes: [r(0, 0)]);
    expect(find.textContaining('여러 출력으로 보내기'), findsNothing);
    await tester.tap(find.byKey(const Key('routing_help_button')));
    await tester.pump();
    expect(find.textContaining('여러 출력으로 보내기'), findsOneWidget);
    await tester.tap(find.byKey(const Key('route_cell_0_4')));
    await tester.pump();
    expect(calls.last.$1, [r(0, 0), r(0, 4)]);
    await tester.tap(find.byKey(const Key('routing_help_button')));
    await tester.pump();
    expect(find.textContaining('여러 출력으로 보내기'), findsNothing);
  });

  testWidgets('1024×768에서 도움말을 펼쳐도 음량 줄이 화면 안에 있다', (tester) async {
    await open(tester, routes: [r(0, 0)]);
    await tester.tap(find.byKey(const Key('routing_help_button')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('route_cell_0_0')));
    await tester.pump();
    final slider = tester.getRect(find.byKey(const Key('route_gain_slider')));
    expect(slider.bottom, lessThanOrEqualTo(768));
    expect(slider.top, greaterThanOrEqualTo(0));
  });

  testWidgets('음량을 끄는 동안은 확정 아님, 놓으면 확정', (tester) async {
    final calls = await open(tester, routes: [r(0, 0), r(0, 1)]);
    await tester.tap(find.byKey(const Key('route_cell_0_1')));
    await tester.pump();
    final slider = find.byKey(const Key('route_gain_slider'));
    await tester.drag(slider, const Offset(-200, 0));
    await tester.pump();
    expect(calls.any((c) => !c.$2), isTrue, reason: '끄는 동안의 값이 엔진에 가야 한다');
    expect(calls.last.$2, isTrue, reason: '놓으면 확정');
    expect(calls.last.$1.firstWhere((x) => x.outputChannel == 1).gain, lessThan(1.0));
  });
}
