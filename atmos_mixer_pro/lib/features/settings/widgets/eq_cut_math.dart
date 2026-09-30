import 'dart:math' as math;

/// 로우컷/하이컷의 크기 응답(dB)을 DSP와 **같은 구성**으로 계산한다.
///
/// DSP(rust/src/audio/dsp.rs의 EqFilterState)는 Butterworth 연결을 쓴다.
/// - 12dB/oct: 2차 1개(사용자 Q)
/// - 18dB/oct: 1차 + 2차(Q 1.0 x r)
/// - 24dB/oct: 2차(Q 0.541 x r) + 2차(Q 1.307 x r), r = 사용자 Q / 0.707
///
/// 예전 EQ 화면은 2차 응답에 slope/12를 곱하고 밴드 게인까지 더해 그렸다.
/// DSP는 컷 필터에 게인을 쓰지 않고 슬로프도 받지 않았으므로 화면과 소리가
/// 달랐다. 이 함수는 화면과 DSP가 같은 곡선을 갖게 한다.
double cutFilterMagnitudeDb({
  required bool lowCut,
  required double cutoffHz,
  required double q,
  required int slopeDbPerOct,
  required double frequencyHz,
  required double sampleRate,
}) {
  final double omega = 2.0 * math.pi * frequencyHz / sampleRate;
  final double r = q / math.sqrt1_2;
  if (slopeDbPerOct >= 24) {
    return _secondOrderDb(lowCut, cutoffHz, 0.5411961 * r, omega, sampleRate) +
        _secondOrderDb(lowCut, cutoffHz, 1.3065630 * r, omega, sampleRate);
  }
  if (slopeDbPerOct >= 18) {
    return _firstOrderDb(lowCut, cutoffHz, omega, sampleRate) +
        _secondOrderDb(lowCut, cutoffHz, 1.0 * r, omega, sampleRate);
  }
  return _secondOrderDb(lowCut, cutoffHz, q, omega, sampleRate);
}

/// 2차 고역/저역 통과(RBJ 쿡북, 쌍선형 + 차단 주파수 사전 왜곡).
/// ZDF SVF와 같은 아날로그 원형을 같은 방식으로 이산화하므로 응답이 같다.
double _secondOrderDb(bool highPass, double fc, double q, double omega, double fs) {
  final double w0 = 2.0 * math.pi * fc / fs;
  final double alpha = math.sin(w0) / (2.0 * math.max(q, 1e-4));
  final double c = math.cos(w0);
  final double b0 = highPass ? (1.0 + c) / 2.0 : (1.0 - c) / 2.0;
  final double b1 = highPass ? -(1.0 + c) : (1.0 - c);
  final double b2 = b0;
  final double a0 = 1.0 + alpha;
  final double a1 = -2.0 * c;
  final double a2 = 1.0 - alpha;
  return _biquadDb(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0, omega);
}

/// 1차 고역/저역 통과(쌍선형 + 사전 왜곡). DSP의 OnePole(TPT)과 같은 응답.
double _firstOrderDb(bool highPass, double fc, double omega, double fs) {
  final double k = math.tan(math.pi * fc / fs);
  final double a0 = 1.0 + k;
  final double a1 = (k - 1.0) / a0;
  final double b0 = highPass ? 1.0 / a0 : k / a0;
  final double b1 = highPass ? -1.0 / a0 : k / a0;
  return _biquadDb(b0, b1, 0.0, a1, 0.0, omega);
}

double _biquadDb(double b0, double b1, double b2, double a1, double a2, double omega) {
  final double cw = math.cos(omega), c2w = math.cos(2.0 * omega);
  final double sw = math.sin(omega), s2w = math.sin(2.0 * omega);
  final double nr = b0 + b1 * cw + b2 * c2w;
  final double ni = -(b1 * sw + b2 * s2w);
  final double dr = 1.0 + a1 * cw + a2 * c2w;
  final double di = -(a1 * sw + a2 * s2w);
  final double num = nr * nr + ni * ni;
  final double den = dr * dr + di * di;
  if (num <= 0.0 || den <= 0.0) return -120.0;
  return 10.0 * math.log(num / den) / math.ln10;
}
