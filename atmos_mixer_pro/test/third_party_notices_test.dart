import 'package:atmos_mixer_pro/features/settings/widgets/third_party_notices.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('KEMAR HRTF 고지가 라이선스 목록에 한 번만 들어가고 출처·인용·이력을 담는다', () async {
    registerThirdPartyNotices();
    registerThirdPartyNotices();

    final entries = await LicenseRegistry.licenses
        .where((e) => e.packages.contains(kemarHrtfPackageName))
        .toList();
    expect(entries, hasLength(1));

    final text = entries.single.paragraphs.map((p) => p.text).join('\n');
    expect(text, contains('Copyright 1994 by the MIT Media Laboratory'));
    expect(text, contains('Gardner, W. G., and Martin, K. D. (1995)'));
    expect(text, contains('Journal of the Acoustical Society of America, 97(6), 3907–3908'));
    expect(text, contains('This HRTF data is provided without any usage restrictions.'));
    expect(text, contains('Piotr Majdak and Michael Mihocic'));
    expect(text, contains('own binary format'));
  });
}
