import 'package:flutter_riverpod/flutter_riverpod.dart';

/// 시작할 때 설정 파일(config.json)을 되살렸다는 안내(rust api::config_file). 확인을 누를 때까지 배너로 남는다.
/// 무인 운영에서 시작 절차(엔진·이어 가기·테마 시작)를 막지 않도록 창이 아니라 배너로 보이고, 시작 절차가 끝난
/// 뒤에 채운다(audio_init_splash_screen.dart).
class ConfigNoticeNotifier extends Notifier<String?> {
  @override
  String? build() => null;

  void show(String? message) {
    if (message != null && message.isNotEmpty) state = message;
  }

  void dismiss() => state = null;
}

final configNoticeProvider = NotifierProvider<ConfigNoticeNotifier, String?>(ConfigNoticeNotifier.new);

/// 안내를 가져와 배너에 넘긴다. 사람의 입력을 기다리지 않으며, 가져오지 못해도 시작 절차를 멈추지 않는다.
Future<void> loadConfigNotice({
  required Future<String?> Function() fetch,
  required void Function(String? message) show,
}) async {
  try {
    show(await fetch());
  } catch (_) {
    // 안내를 못 가져와도 앱은 계속 쓴다(되살린 사실은 Rust가 사건 로그에 남겼다).
  }
}
