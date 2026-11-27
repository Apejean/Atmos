import 'package:atmos_mixer_pro/core/input/duplicate_click_filter.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';

/// 원격 제어의 이중 클릭 거름망(core/input/duplicate_click_filter.dart).
void main() {
  Duration ms(int v) => Duration(milliseconds: v);
  const a = Offset(100, 200);

  PointerDownEvent down(int pointer, int t, Offset at,
          {PointerDeviceKind kind = PointerDeviceKind.mouse, int buttons = kPrimaryButton, int device = 0}) =>
      PointerDownEvent(pointer: pointer, timeStamp: ms(t), position: at, kind: kind, buttons: buttons, device: device);
  PointerMoveEvent move(int pointer, int t, Offset at) =>
      PointerMoveEvent(pointer: pointer, timeStamp: ms(t), position: at, buttons: kPrimaryButton);
  PointerUpEvent up(int pointer, int t, Offset at, {PointerDeviceKind kind = PointerDeviceKind.mouse}) =>
      PointerUpEvent(pointer: pointer, timeStamp: ms(t), position: at, kind: kind);

  List<bool> run(DuplicateClickFilter f, List<PointerEvent> events) => events.map(f.shouldDrop).toList();

  test('TeamViewer 마우스 모드: 15ms 뒤 같은 자리의 두 번째 클릭(움직임·뗌 포함)을 통째로 버린다', () {
    final f = DuplicateClickFilter();
    final dropped = run(f, [
      down(1, 0, a),
      up(1, 5, a),
      down(2, 15, a),
      move(2, 17, a + const Offset(1, 0)),
      up(2, 20, a),
    ]);
    expect(dropped, [false, false, true, true, true]);
    expect(f.dropped, 1);
  });

  test('휴대폰 TeamViewer: 눌림 사이 32·47·62·78ms의 두 번째 클릭도 버린다(2026-11-24·27 OS 기록)', () {
    for (final gap in [32, 47, 62, 78]) {
      final f = DuplicateClickFilter();
      expect(run(f, [down(1, 0, a), up(1, 16, a), down(2, gap, a), up(2, gap + 16, a)]), [false, false, true, true],
          reason: '간격 ${gap}ms');
    }
  });

  test('사람의 더블클릭(눌림 사이 120ms 이상)은 그대로 둔다', () {
    final f = DuplicateClickFilter();
    expect(run(f, [down(1, 0, a), up(1, 60, a), down(2, 120, a), up(2, 180, a)]), everyElement(isFalse));
  });

  test('다른 자리·다른 단추·다른 장치·펜은 같은 클릭으로 보지 않는다', () {
    final f = DuplicateClickFilter();
    expect(f.shouldDrop(down(1, 0, a)), isFalse);
    expect(f.shouldDrop(down(2, 10, a + const Offset(30, 0))), isFalse, reason: '다른 자리');
    expect(f.shouldDrop(down(3, 20, a + const Offset(30, 0), buttons: kSecondaryButton)), isFalse, reason: '오른쪽 단추');
    expect(f.shouldDrop(down(4, 25, a + const Offset(30, 0), device: 7)), isFalse, reason: '다른 장치');
    expect(f.shouldDrop(down(5, 27, a + const Offset(30, 0), kind: PointerDeviceKind.stylus)), isFalse, reason: '펜');
  });

  test('터치도 같은 자리에 겹쳐 온 누름은 버리고, 두 손가락(다른 자리)은 그대로 둔다', () {
    final f = DuplicateClickFilter();
    const touch = PointerDeviceKind.touch;
    expect(f.shouldDrop(down(1, 0, a, kind: touch)), isFalse);
    expect(f.shouldDrop(down(2, 10, a, kind: touch)), isTrue);
    expect(f.shouldDrop(up(2, 12, a, kind: touch)), isTrue);
    expect(f.shouldDrop(down(3, 14, const Offset(400, 50), kind: touch)), isFalse);
  });

  test('바인딩이 이미 있으면(통합 시험 등) 새로 만들지 않고 그것을 쓴다', () {
    final existing = TestWidgetsFlutterBinding.ensureInitialized();
    expect(AtmosBinding.ensureInitialized(), same(existing));
  });

  test('버린 뒤 다음 정상 클릭은 지나가고, 세 번 겹쳐 와도 기준은 처음 누름이다', () {
    final f = DuplicateClickFilter();
    expect(run(f, [down(1, 0, a), down(2, 15, a), up(2, 16, a), down(3, 30, a), up(3, 31, a), up(1, 32, a)]),
        [false, true, true, true, true, false]);
    expect(f.shouldDrop(down(4, 300, a)), isFalse, reason: '거름망이 다음 클릭을 막으면 안 된다');
    expect(f.dropped, 2);
  });
}
