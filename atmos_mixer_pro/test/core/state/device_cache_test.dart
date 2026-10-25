import 'package:flutter_test/flutter_test.dart';

import 'package:atmos_mixer_pro/core/state/global_state.dart';

/// 장치 목록의 "연결 확인 전" 표시(2026-10-22 Windows). 엔진이 ASIO로 도는 동안 읽은 목록에는 다른 ASIO 장치가
/// 드라이버를 열지 않은 채 들어 있어, 연결이 빠진 Focusrite가 보이는데 골랐다가 30초 뒤 "장치를 찾지 못함"이 났다.
void main() {
  const rme = '[ASIO] ASIO MADIface USB';
  const focusrite = '[ASIO] Focusrite USB ASIO';

  tearDown(() => GlobalDeviceCache.deepScanned = false);

  test('엔진이 ASIO로 도는 동안 읽은 목록의 다른 ASIO 장치만 연결 확인 전이다', () {
    GlobalDeviceCache.deepScanned = false;
    expect(GlobalDeviceCache.isUnverifiedAsio(focusrite, rme), isTrue);
    expect(GlobalDeviceCache.isUnverifiedAsio(rme, rme), isFalse, reason: '엔진이 연 장치는 확인된 것');
    expect(GlobalDeviceCache.isUnverifiedAsio('[WASAPI] Speakers (Realtek)', rme), isFalse, reason: 'WASAPI 장치');
    expect(GlobalDeviceCache.isUnverifiedAsio(focusrite, '[WASAPI] Speakers (Realtek)'), isFalse,
        reason: 'ASIO 엔진이 아니면 목록을 만들 때 ASIO 드라이버를 열어 봤다');
    expect(GlobalDeviceCache.isUnverifiedAsio(focusrite, null), isFalse, reason: '기본 장치 엔진');
    expect(GlobalDeviceCache.isUnverifiedAsio(null, rme), isFalse, reason: '기본 장치 선택');
  });

  test('재스캔한 목록은 드라이버를 열어 본 것이라 표시하지 않는다', () {
    GlobalDeviceCache.deepScanned = true;
    expect(GlobalDeviceCache.isUnverifiedAsio(focusrite, rme), isFalse);
  });
}
