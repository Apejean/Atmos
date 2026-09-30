import 'package:flutter_test/flutter_test.dart';
import 'package:atmos_mixer_pro/features/settings/widgets/eq_cut_math.dart';

/// EQ 화면의 컷 필터 곡선이 DSP(rust/tests/test_eq_cut_slope.rs)와 같은
/// Butterworth 기준값을 따르는지 본다. 한 옥타브 떨어진 지점의 감쇠:
/// 2차 -12.30dB / 3차 -18.13dB / 4차 -24.10dB.
void main() {
  const expects = {12: -12.30, 18: -18.13, 24: -24.10};

  test('로우컷 곡선이 슬로프별 Butterworth 감쇠를 그린다', () {
    expects.forEach((slope, expected) {
      final db = cutFilterMagnitudeDb(
        lowCut: true, cutoffHz: 200, q: 0.7071, slopeDbPerOct: slope,
        frequencyHz: 100, sampleRate: 48000,
      );
      expect(db, closeTo(expected, 0.5), reason: '로우컷 $slope dB/oct');
      final pass = cutFilterMagnitudeDb(
        lowCut: true, cutoffHz: 200, q: 0.7071, slopeDbPerOct: slope,
        frequencyHz: 3000, sampleRate: 48000,
      );
      expect(pass.abs(), lessThan(0.3), reason: '로우컷 $slope 통과대역');
    });
  });

  test('하이컷 곡선이 슬로프별 Butterworth 감쇠를 그린다', () {
    expects.forEach((slope, expected) {
      final db = cutFilterMagnitudeDb(
        lowCut: false, cutoffHz: 2000, q: 0.7071, slopeDbPerOct: slope,
        frequencyHz: 4000, sampleRate: 48000,
      );
      expect(db, closeTo(expected, 1.0), reason: '하이컷 $slope dB/oct');
    });
  });

  test('컷 필터 곡선에는 밴드 게인이 섞이지 않는다(DSP도 게인을 쓰지 않는다)', () {
    final at1k = cutFilterMagnitudeDb(
      lowCut: true, cutoffHz: 100, q: 0.7071, slopeDbPerOct: 12,
      frequencyHz: 1000, sampleRate: 48000,
    );
    expect(at1k.abs(), lessThan(0.1));
  });
}
