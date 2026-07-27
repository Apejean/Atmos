import 'dart:math' as math;

void main() {
  double bandF = 1000.0;
  double bandQ = 0.707;
  double w0 = 2.0 * math.pi * bandF / 48000.0;
  double alpha = math.sin(w0) / (2.0 * bandQ);
  
  double b0 = (1.0 - math.cos(w0)) / 2.0;
  double b1 = 1.0 - math.cos(w0);
  double b2 = (1.0 - math.cos(w0)) / 2.0;
  double a0 = 1.0 + alpha;
  double a1 = -2.0 * math.cos(w0);
  double a2 = 1.0 - alpha;

  double nb0 = b0 / a0;
  double nb1 = b1 / a0;
  double nb2 = b2 / a0;
  double na1 = a1 / a0;
  double na2 = a2 / a0;

  for (double f in [20.0, 100.0, 500.0, 1000.0, 2000.0, 10000.0]) {
    double omega = 2.0 * math.pi * f / 48000.0;
    double cosW = math.cos(omega);
    double cos2W = math.cos(2.0 * omega);
    double sinW = math.sin(omega);
    double sin2W = math.sin(2.0 * omega);

    double numReal = nb0 + nb1 * cosW + nb2 * cos2W;
    double numImag = nb1 * sinW + nb2 * sin2W;
    double denReal = 1.0 + na1 * cosW + na2 * cos2W;
    double denImag = na1 * sinW + na2 * sin2W;

    double numSq = numReal * numReal + numImag * numImag;
    double denSq = denReal * denReal + denImag * denImag;
    double magSq = numSq / denSq;
    double db = 10.0 * math.log(magSq) / math.ln10;
    print("Freq: $f Hz -> $db dB");
  }
}
