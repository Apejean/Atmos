import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

/// 앱에 넣은 제3자 데이터의 출처·저작권 고지. Flutter 라이선스 화면(LicensePage)에
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

/// ASIO 상표 고지: Steinberg가 ASIO SDK 2.3.4와 함께 준 로고(474×443)를 바꾸지 않고 그대로 넣었다
/// (색·자르기·늘이기 금지, 가로세로비 유지). 사용 지침 15절의 상표 문구와 계약서(2.0.5) 3조 1항 k의
/// Steinberg 고지 문구를 로고 아래에 글자로 함께 둔다. ASIO는 Windows 판에서만 쓰므로 Windows에서만 보인다.
const String asioLogoAsset = 'assets/legal/ASIO-compatible-logo-Steinberg-TM-BW.png';
const String asioTrademarkAttribution = 'ASIO is a registered trademark of Steinberg Media Technologies GmbH.';
const String asioCopyrightNotice =
    'ASIO is a trademark of Steinberg Media Technologies GmbH, registered in Europe and other countries.';

/// 화면에 보일 로고 너비(논리 픽셀). 그림 속 상표 문구가 읽히고, 배율 250%까지 원본보다 키우지 않는다.
const double asioLogoWidth = 180;

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
  Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => const ThirdPartyNoticesPage()));
}

/// 라이선스 화면 맨 아래에 ASIO 고지 칸을 붙인 화면. showLicensePage는 맨 아래 칸을 받지 않아
/// 같은 LicensePage를 직접 감싼다.
class ThirdPartyNoticesPage extends StatelessWidget {
  const ThirdPartyNoticesPage({super.key});

  @override
  Widget build(BuildContext context) {
    return Material(
      child: Column(
        children: [
          const Expanded(child: LicensePage(applicationName: 'The Awesome')),
          if (defaultTargetPlatform == TargetPlatform.windows) const AsioTrademarkNotice(),
        ],
      ),
    );
  }
}

/// ASIO 로고와 상표·고지 문구.
class AsioTrademarkNotice extends StatelessWidget {
  const AsioTrademarkNotice({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final style = theme.textTheme.bodySmall;
    return SafeArea(
      top: false,
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(border: Border(top: BorderSide(color: theme.dividerColor))),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            SizedBox(
              width: asioLogoWidth,
              child: AspectRatio(
                aspectRatio: 474 / 443,
                child: Image.asset(
                  asioLogoAsset,
                  fit: BoxFit.contain,
                  filterQuality: FilterQuality.medium,
                  semanticLabel: 'ASIO Compatible logo',
                ),
              ),
            ),
            const SizedBox(height: 8),
            Text(asioTrademarkAttribution, style: style, textAlign: TextAlign.center),
            Text(asioCopyrightNotice, style: style, textAlign: TextAlign.center),
          ],
        ),
      ),
    );
  }
}
