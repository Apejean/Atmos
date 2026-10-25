import 'package:flutter_test/flutter_test.dart';

import '../../tool/legal/third_party_licenses.dart';

void main() {
  group('parseCargoTreeLine', () {
    test('crates.io 크레이트는 이름·판·라이선스를 읽고 (*) 표시를 뗀다', () {
      final c = parseCargoTreeLine('anyhow v1.0.104|MIT OR Apache-2.0 (*)')!;
      expect([c.name, c.version, c.license], ['anyhow', '1.0.104', 'MIT OR Apache-2.0']);
    });

    test('라이선스가 빈 크레이트도 읽는다(allo-isolate)', () {
      expect(parseCargoTreeLine('allo-isolate v0.1.27|')!.license, '');
    });

    test('우리 크레이트(로컬 경로)와 빈 줄은 뺀다', () {
      expect(parseCargoTreeLine(r'rust_lib_atmos_mixer_pro v1.1.2 (D:\dev\Atmos\atmos_mixer_pro\rust)|'), isNull);
      expect(parseCargoTreeLine('atmos_supervisor v0.1.0 (/Users/me/atmos/supervisor)|'), isNull);
      expect(parseCargoTreeLine('  '), isNull);
    });

    test('crates.io가 아닌 출처(git)는 멈춘다', () {
      expect(() => parseCargoTreeLine('foo v0.1.0 (https://github.com/x/foo#abc)|MIT'), throwsFormatException);
    });
  });

  group('chooseLicenses', () {
    test('MIT OR Apache-2.0: MIT 원문 파일이 있으면 MIT, 없으면 Apache-2.0', () {
      expect(chooseLicenses('MIT OR Apache-2.0', hasMitFile: true), ['MIT']);
      expect(chooseLicenses('Apache-2.0 OR MIT', hasMitFile: false), ['Apache-2.0']);
    });

    test('옛 표기 MIT/Apache-2.0도 OR로 본다', () {
      expect(chooseLicenses('MIT/Apache-2.0', hasMitFile: true), ['MIT']);
    });

    test('AND는 모두 싣는다(encoding_rs, unicode-ident)', () {
      expect(chooseLicenses('(Apache-2.0 OR MIT) AND BSD-3-Clause', hasMitFile: true), ['MIT', 'BSD-3-Clause']);
      expect(chooseLicenses('(MIT OR Apache-2.0) AND Unicode-3.0', hasMitFile: false), ['Apache-2.0', 'Unicode-3.0']);
    });

    test('Zlib·Unlicense가 섞인 선택지도 MIT 원문이 있으면 MIT', () {
      expect(chooseLicenses('Zlib OR Apache-2.0 OR MIT', hasMitFile: true), ['MIT']);
      expect(chooseLicenses('Unlicense OR MIT', hasMitFile: true), ['MIT']);
    });

    test('단일 라이선스(MPL)와 빈 표현', () {
      expect(chooseLicenses('MPL-2.0', hasMitFile: false), ['MPL-2.0']);
      expect(chooseLicenses('', hasMitFile: false), isEmpty);
    });
  });

  group('selectLicenseFiles', () {
    test('MIT를 고르면 MIT 원문만 싣는다', () {
      expect(
        selectLicenseFiles(['MIT'], {'LICENSE-APACHE': LicenseKind.apache, 'LICENSE-MIT': LicenseKind.mit}),
        ['LICENSE-MIT'],
      );
    });

    test('Apache·MPL은 부록을 가리키므로 크레이트 파일을 싣지 않는다', () {
      expect(selectLicenseFiles(['Apache-2.0'], {'LICENSE-APACHE': LicenseKind.apache, 'LICENSE-MIT': LicenseKind.mit}),
          isEmpty);
      expect(selectLicenseFiles(['MPL-2.0'], {'LICENSE': LicenseKind.mpl}), isEmpty);
    });

    test('AND의 BSD-3-Clause(encoding_rs)는 WHATWG 원문과 COPYRIGHT까지 싣는다', () {
      final files = {
        'LICENSE-APACHE': LicenseKind.apache,
        'LICENSE-MIT': LicenseKind.mit,
        'LICENSE-WHATWG': LicenseKind.other,
        'COPYRIGHT': LicenseKind.other,
      };
      expect(selectLicenseFiles(['MIT', 'BSD-3-Clause'], files), ['COPYRIGHT', 'LICENSE-MIT', 'LICENSE-WHATWG']);
    });

    test('표현이 비거나 맞는 파일이 없으면 라이선스 파일을 모두 싣는다', () {
      expect(selectLicenseFiles([], {'LICENSE': LicenseKind.apache}), ['LICENSE']);
      expect(selectLicenseFiles(['MIT'], {'LICENSE': LicenseKind.apache}), ['LICENSE']);
    });
  });

  test('classifyLicenseFile은 이름을 먼저, 본문을 다음에 본다', () {
    expect(classifyLicenseFile('LICENSE-MIT', ''), LicenseKind.mit);
    expect(classifyLicenseFile('LICENSE', 'Permission is hereby granted, free of charge, to any person'), LicenseKind.mit);
    expect(classifyLicenseFile('LICENSE', 'Apache License\nVersion 2.0, January 2004'), LicenseKind.apache);
    expect(classifyLicenseFile('LICENSE', 'Mozilla Public License Version 2.0'), LicenseKind.mpl);
    expect(classifyLicenseFile('COPYRIGHT', 'Permission is hereby granted, free of charge'), LicenseKind.other);
  });
}
