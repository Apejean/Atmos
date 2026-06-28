import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'PointerScrollEvent on nested vertical ListView inside horizontal ListView',
    (WidgetTester tester) async {
      final ScrollController horizontalController = ScrollController();
      final ScrollController verticalController = ScrollController();
      bool outerListenerCalled = false;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Listener(
              onPointerSignal: (pointerSignal) {
                if (pointerSignal is PointerScrollEvent) {
                  GestureBinding.instance.pointerSignalResolver.register(
                    pointerSignal,
                    (PointerSignalEvent event) {
                      outerListenerCalled = true;
                      if (event is PointerScrollEvent) {
                        horizontalController.jumpTo(
                          horizontalController.offset + event.scrollDelta.dy,
                        );
                      }
                    },
                  );
                }
              },
              child: ListView.builder(
                controller: horizontalController,
                scrollDirection: Axis.horizontal,
                itemCount: 1,
                itemBuilder: (context, index) => SizedBox(
                  width: 300,
                  child: ListView.builder(
                    controller: verticalController,
                    scrollDirection: Axis.vertical,
                    itemCount: 20,
                    itemBuilder: (context, vIndex) =>
                        Container(height: 50, color: Colors.red),
                  ),
                ),
              ),
            ),
          ),
        ),
      );

      await tester.pumpAndSettle();

      // Simulate vertical mouse wheel scroll over the vertical ListView
      final Offset location = tester.getCenter(find.byType(ListView).last);
      final TestPointer pointer = TestPointer(1, PointerDeviceKind.mouse);
      pointer.hover(location);
      await tester.sendEventToBinding(pointer.scroll(const Offset(0.0, 50.0)));
      await tester.pumpAndSettle();

      expect(outerListenerCalled, false); // Inner list should consume it
      expect(verticalController.offset, 50.0); // Inner list scrolled
      expect(horizontalController.offset, 0.0); // Outer list didn't scroll
    },
  );
}
