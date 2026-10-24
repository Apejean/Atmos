/// 앱의 Credits 창과 설치 폴더 Legal 문서(`tool/legal/generate_third_party_licenses.dart`)가 함께 쓰는 고지 문구.
/// 생성 스크립트가 Flutter 없이 읽도록 순수 Dart로 둔다.
library;

/// MIT KEMAR HRTF: 헤드폰 바이노럴 미리듣기가 쓰는 측정 데이터(rust/assets/hrtf). 원본에 적힌
/// 사용 조건을 그대로 옮기고 그보다 넓은 권리를 주장하지 않는다. 기록: rust/assets/hrtf/README.md.
const String kemarHrtfPackageName = 'MIT KEMAR HRTF data';

const String kemarHrtfNotice = '''Copyright 1994 by the MIT Media Laboratory.
Measurements by Bill Gardner and Keith Martin, MIT Media Lab.

Gardner, W. G., and Martin, K. D. (1995). "HRTF measurements of a KEMAR." Journal of the Acoustical Society of America, 97(6), 3907–3908.

Terms of use as stated in the SOFA file: "This HRTF data is provided without any usage restrictions. It is requested that you cite Gardner and Martin (1995) when using this data for research or commercial applications."

Source: http://sound.media.mit.edu/resources/KEMAR.html

History: The original MIT KEMAR data was converted to the SOFA format by Piotr Majdak and Michael Mihocic (Acoustics Research Institute, Austrian Academy of Sciences). The Awesome further converted that SOFA file into its own binary format without changing the values. The Awesome equalizes the responses at run time for its headphone binaural preview.''';

/// ASIO 상표 고지: Steinberg ASIO SDK 2.3.4 사용 지침 15절 문구와 계약서(2.0.5) 3조 1항 k의 Steinberg 고지 문구.
/// 근거가 달라(지침 15절 / 계약 3조 1항 k) 두 줄을 늘 함께 싣는다(2026-10-25 재확인: 로고 그림으로 대신하지 않는다).
const String asioTrademarkAttribution = 'ASIO is a registered trademark of Steinberg Media Technologies GmbH.';
const String asioCopyrightNotice =
    'ASIO is a trademark of Steinberg Media Technologies GmbH, registered in Europe and other countries.';

/// 설치 폴더 안 법률 문서 폴더 이름(Ableton Live처럼 Legal).
const String legalFolderName = 'Legal';
