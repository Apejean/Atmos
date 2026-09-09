import 'dart:math' as math;
import 'package:flutter_test/flutter_test.dart';
import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';

/// RT60 추정식(Sabine/Eyring 하이브리드) 검증.
///
/// 핵심 회귀 방지 대상: 공기 흡음(4mV) 항이 Eyring 분기에만 있고 Sabine 분기에는
/// 빠져 있던 버그. 공기 흡음은 크고 반사가 심한 공간(= Sabine 분기)일수록 비중이
/// 커지므로, 그 분기에서 빠지면 대형 콘크리트 공간의 RT60이 20% 넘게 과대평가된다.
void main() {
  RoomZone room({
    required double w,
    required double d,
    required double h,
    required double alpha,
  }) =>
      RoomZone(
        id: 'r',
        x: 0,
        y: 0,
        width: 1,
        height: 1,
        color: 0xFFFFFFFF,
        physicalWidth: w,
        physicalHeight: d,
        ceilingHeight: h,
        absorptionCoeff: alpha,
      );

  group('estimatedRt60', () {
    test('Sabine 분기(반사성 대공간)에 공기 흡음이 반영된다', () {
      // 15 x 20 x 15m 콘크리트 홀 (실제 앱에서 관측된 설정)
      const w = 15.0, d = 20.0, h = 15.0, alpha = 0.02;
      final v = w * d * h; // 4500 m^3
      final s = 2 * (w * d + w * h + d * h); // 1650 m^2

      final expected = (0.161 * v) / (s * alpha + 0.002 * v);
      final actual = room(w: w, d: d, h: h, alpha: alpha).estimatedRt60;

      expect(actual, closeTo(expected, 0.001));

      // 공기 흡음을 뺀 순수 Sabine 값보다 반드시 짧아야 한다(과대평가 방지).
      final withoutAir = (0.161 * v) / (s * alpha);
      expect(actual, lessThan(withoutAir),
          reason: '공기 흡음이 빠지면 RT60이 과대평가된다');
      expect(withoutAir - actual, greaterThan(4.0),
          reason: '이 크기의 공간에서는 공기 흡음 효과가 수 초 단위로 유의미해야 한다');
    });

    test('Eyring 분기(흡음성 데드룸)는 기존 동작을 유지한다', () {
      const w = 10.0, d = 8.0, h = 3.0, alpha = 0.35;
      final v = w * d * h;
      final s = 2 * (w * d + w * h + d * h);

      final expected =
          (0.161 * v) / (-s * math.log(1.0 - alpha) + 0.002 * v);
      final actual = room(w: w, d: d, h: h, alpha: alpha).estimatedRt60;

      expect(actual, closeTo(expected, 0.001));
    });

    test('흡음이 커질수록 RT60은 단조 감소한다', () {
      double rtFor(double alpha) =>
          room(w: 12, d: 10, h: 4, alpha: alpha).estimatedRt60;

      final values = [0.02, 0.1, 0.3, 0.6, 0.9].map(rtFor).toList();
      for (var i = 1; i < values.length; i++) {
        expect(values[i], lessThan(values[i - 1]),
            reason: '흡음계수가 커지면 잔향은 줄어야 한다 (index $i)');
      }
    });
  });
}
