import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

/// 앱에 넣은 제3자 데이터의 출처·저작권 고지. Flutter 라이선스 화면(showLicensePage)에
/// 패키지 라이선스들과 함께 보인다.
///
/// MIT KEMAR HRTF: 헤드폰 바이노럴 미리듣기가 쓰는 측정 데이터(rust/assets/hrtf). 원본에 적힌
/// 사용 조건을 그대로 옮기고 그보다 넓은 권리를 주장하지 않는다. 기록: rust/assets/hrtf/README.md.
const String kemarHrtfPackageName = 'MIT KEMAR HRTF data';

const String kemarHrtfNotice = '''Copyright 1994 by the MIT Media Laboratory.
Measurements by Bill Gardner and Keith Martin, MIT Media Lab.

Gardner, W. G., and Martin, K. D. (1995). "HRTF measurements of a KEMAR." Journal of the Acoustical Society of America, 97(6), 3907–3908.

Terms of use as stated in the SOFA file: "This HRTF data is provided without any usage restrictions. It is requested that you cite Gardner and Martin (1995) when using this data for research or commercial applications."

Source: http://sound.media.mit.edu/resources/KEMAR.html

History: The original MIT KEMAR data was converted to the SOFA format by Piotr Majdak and Michael Mihocic (Acoustics Research Institute, Austrian Academy of Sciences). The Awesome further converted that SOFA file into its own binary format without changing the values. The Awesome equalizes the responses at run time for its headphone binaural preview.''';

bool _registered = false;

/// 고지를 라이선스 목록에 한 번만 더한다.
void registerThirdPartyNotices() {
  if (_registered) return;
  _registered = true;
  LicenseRegistry.addLicense(
    () => Stream<LicenseEntry>.value(
      const LicenseEntryWithLineBreaks(<String>[kemarHrtfPackageName], kemarHrtfNotice),
    ),
  );
}

/// Help → Third-party Notices: 라이선스 화면을 연다.
void showThirdPartyNotices(BuildContext context) {
  registerThirdPartyNotices();
  showLicensePage(context: context, applicationName: 'The Awesome');
}
