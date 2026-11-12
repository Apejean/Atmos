import 'dart:io';

/// OSC 응답 목적지 입력란(환경설정) ↔ 설정 `oscReplyTargets`(규격 OSC_INTERFACE_SPEC.md §3.1).
/// 쉼표로 나눠 앞뒤 공백만 지우고 빈칸은 버린다. 잘못된 항목도 사용자가 적은 그대로 저장한다
/// (엔진은 보낼 때만 건너뛰고 로그를 남긴다, rust osc::reply). 화면은 [invalidReplyTargets]로 경고만 보인다.
List<String> parseReplyTargetsText(String text) =>
    text.split(',').map((s) => s.trim()).where((s) => s.isNotEmpty).toList();

String replyTargetsText(List<String> targets) => targets.join(', ');

/// 목적지 최대 개수(rust osc::reply::MAX_TARGETS와 같다).
const int maxReplyTargets = 8;

/// 엔진이 받지 않을 항목. IP 숫자와 포트(1~65535)가 아니거나, 0.0.0.0·::·멀티캐스트·255.255.255.255이면 잘못이다.
/// IPv6는 `[::1]:9000` 형식이다.
List<String> invalidReplyTargets(List<String> targets) =>
    targets.where((t) => !_isValidTarget(t)).toList();

bool _isValidTarget(String text) {
  final t = text.trim();
  String host;
  String portText;
  if (t.startsWith('[')) {
    final end = t.indexOf(']:');
    if (end < 0) return false;
    host = t.substring(1, end);
    portText = t.substring(end + 2);
  } else {
    final colon = t.lastIndexOf(':');
    if (colon <= 0 || t.indexOf(':') != colon) return false;
    host = t.substring(0, colon);
    portText = t.substring(colon + 1);
  }
  final port = int.tryParse(portText);
  if (port == null || port < 1 || port > 65535) return false;
  final ip = InternetAddress.tryParse(host);
  if (ip == null) return false;
  if (t.startsWith('[') != (ip.type == InternetAddressType.IPv6)) return false;
  if (ip.address == '0.0.0.0' || ip.address == '::' || ip.address == '255.255.255.255') return false;
  if (ip.isMulticast) return false;
  return true;
}
