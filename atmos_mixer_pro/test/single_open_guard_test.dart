import 'dart:async';

import 'package:atmos_mixer_pro/core/utils/single_open_guard.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

/// Settings → Preferences, Help → About Awesome...를 한 번 눌러도 창이 두 장 겹쳐 열렸다(2026-11-24 Windows 실기).
/// 원격 제어(TeamViewer 마우스 모드)가 클릭 한 번에 눌림·뗌을 15ms 간격으로 두 쌍 보냈고, 두 쌍이 같은 프레임에 들어오면
/// 메뉴 항목의 `onPressed`가 두 번 불린다(MenuItemButton._handleSelect는 항목이 아직 화면에 있는 동안 들어온 탭마다 호출).
/// 입력은 앱이 고칠 수 없어서, 창이 열려 있는 동안 같은 창을 다시 열지 않게 막는다.
Widget _menuApp(void Function(BuildContext context) onPreferences) => MaterialApp(
      home: Scaffold(
        body: Builder(
          builder: (context) => MenuBar(
            children: [
              SubmenuButton(
                menuChildren: [
                  MenuItemButton(
                    onPressed: () => onPreferences(context),
                    child: const Text('Preferences'),
                  ),
                ],
                child: const Text('Settings'),
              ),
            ],
          ),
        ),
      ),
    );

Future<void> _openPreferencesMenu(WidgetTester tester) async {
  await tester.tap(find.text('Settings'));
  await tester.pumpAndSettle();
}

/// 같은 프레임 안에 눌림·뗌 한 쌍을 두 번 보낸다(원격 제어가 보낸 것과 같다).
Future<void> _doubleClick(WidgetTester tester, Finder target) async {
  final center = tester.getCenter(target);
  for (var i = 0; i < 2; i++) {
    final gesture = await tester.startGesture(center, kind: PointerDeviceKind.mouse);
    await gesture.up();
  }
  await tester.pumpAndSettle();
}

Future<void> _showPreferences(BuildContext context) => showDialog<void>(
      context: context,
      builder: (context) => const AlertDialog(title: Text('Preferences dialog')),
    );

void main() {
  group('SingleOpenGuard', () {
    test('앞의 열기가 끝나기 전에 다시 부르면 무시한다', () async {
      final guard = SingleOpenGuard();
      final closing = Completer<void>();
      var opened = 0;

      final first = guard.run(() async {
        opened++;
        await closing.future;
      });
      await guard.run(() async => opened++);

      expect(opened, 1);
      expect(guard.isOpen, isTrue);

      closing.complete();
      await first;
      expect(guard.isOpen, isFalse);
    });

    test('닫힌 뒤에는 다시 열 수 있다', () async {
      final guard = SingleOpenGuard();
      var opened = 0;
      await guard.run(() async => opened++);
      await guard.run(() async => opened++);
      expect(opened, 2);
    });

    test('열기가 예외를 던져도 표시가 풀려 다시 열 수 있다', () async {
      final guard = SingleOpenGuard();
      await expectLater(guard.run(() async => throw StateError('열기 실패')), throwsStateError);
      expect(guard.isOpen, isFalse);

      var opened = 0;
      await guard.run(() async => opened++);
      expect(opened, 1);
    });
  });

  group('메뉴 항목을 같은 프레임에 두 번 눌렀을 때', () {
    testWidgets('대조: 막지 않으면 창이 두 장 겹쳐 열린다(원인 재현)', (tester) async {
      await tester.pumpWidget(_menuApp((context) => _showPreferences(context)));
      await _openPreferencesMenu(tester);

      await _doubleClick(tester, find.text('Preferences'));

      expect(find.text('Preferences dialog'), findsNWidgets(2));
    });

    testWidgets('문지기를 거치면 한 장만 열린다', (tester) async {
      final guard = SingleOpenGuard();
      await tester.pumpWidget(
        _menuApp((context) => guard.run(() => _showPreferences(context))),
      );
      await _openPreferencesMenu(tester);

      await _doubleClick(tester, find.text('Preferences'));

      expect(find.text('Preferences dialog'), findsOneWidget);
    });

    testWidgets('창을 닫은 뒤에는 다시 한 번 열 수 있다', (tester) async {
      final guard = SingleOpenGuard();
      await tester.pumpWidget(
        _menuApp((context) => guard.run(() => _showPreferences(context))),
      );
      await _openPreferencesMenu(tester);
      await _doubleClick(tester, find.text('Preferences'));
      expect(find.text('Preferences dialog'), findsOneWidget);

      Navigator.of(tester.element(find.text('Preferences dialog'))).pop();
      await tester.pumpAndSettle();
      expect(find.text('Preferences dialog'), findsNothing);

      await _openPreferencesMenu(tester);
      await tester.tap(find.text('Preferences'));
      await tester.pumpAndSettle();
      expect(find.text('Preferences dialog'), findsOneWidget);
    });
  });
}
