import 'package:atmos_mixer_pro/core/legal/legal_texts.dart';
import 'package:flutter/material.dart';

/// 환경설정에 쓰는 어두운 배경용 흰 로고. Steinberg 파일 `ASIO-compatible-logo-Steinberg-®-white-transparent-RGB.png`
/// (452×288)를 바꾸지 않고 이름의 ®만 R로 바꿔 넣었다. 그림에 상표 문구가 없어 지침 4쪽의 허용 형태대로
/// 오른쪽에 일반 글꼴로 문구를 둔다.
const String asioLogoWhiteAsset = 'assets/legal/ASIO-compatible-logo-Steinberg-R-white-transparent-RGB.png';

/// 환경설정에 보일 흰 로고 높이(논리 픽셀).
const double asioSettingLogoHeight = 40;

/// 환경설정 Driver type 아래(Windows): 흰 로고와 사용 지침 15절 문구. 지침 1절 f는 ASIO를 사람이 직접 켜는
/// 앱이면 ASIO를 켜거나 설정하는 모든 창에 로고를 두게 한다. 이 앱은 새 설치 기본이 WASAPI이고 환경설정에서
/// ASIO를 고르며, ASIO를 고르는 창은 환경설정뿐이다.
class AsioSettingNotice extends StatelessWidget {
  const AsioSettingNotice({super.key});

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        SizedBox(
          height: asioSettingLogoHeight,
          child: AspectRatio(
            aspectRatio: 452 / 288,
            child: Image.asset(
              asioLogoWhiteAsset,
              fit: BoxFit.contain,
              filterQuality: FilterQuality.medium,
              semanticLabel: 'ASIO Compatible logo',
            ),
          ),
        ),
        const SizedBox(width: 12),
        const Expanded(
          child: Text(asioTrademarkAttribution, style: TextStyle(color: Colors.white60, fontSize: 12)),
        ),
      ],
    );
  }
}
