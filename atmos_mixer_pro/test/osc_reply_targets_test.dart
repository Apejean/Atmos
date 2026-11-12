import 'package:atmos_mixer_pro/core/utils/osc_reply_targets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('parse splits on commas, trims, drops blanks and keeps text as typed', () {
    expect(parseReplyTargetsText(' 192.168.0.20:9000 ,, bad ,'), ['192.168.0.20:9000', 'bad']);
    expect(parseReplyTargetsText(''), isEmpty);
    expect(replyTargetsText(['a:1', 'b:2']), 'a:1, b:2');
  });

  test('invalid targets match the engine rules', () {
    expect(
      invalidReplyTargets([
        '192.168.0.20:9000',
        '[::1]:9000',
        'controller.local:9000',
        '192.168.0.20',
        '192.168.0.20:0',
        '0.0.0.0:9000',
        '[::]:9000',
        '239.1.2.3:9000',
        '255.255.255.255:9000',
        '::1:9000',
      ]),
      [
        'controller.local:9000',
        '192.168.0.20',
        '192.168.0.20:0',
        '0.0.0.0:9000',
        '[::]:9000',
        '239.1.2.3:9000',
        '255.255.255.255:9000',
        '::1:9000',
      ],
    );
  });
}
