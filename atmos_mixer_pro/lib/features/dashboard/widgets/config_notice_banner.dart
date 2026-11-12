import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/state/config_notice.dart';

/// 설정 파일을 되살렸다는 배너. 확인을 누르면 닫힌다. 다른 동작을 막지 않는다.
class ConfigNoticeBanner extends ConsumerWidget {
  const ConfigNoticeBanner({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final message = ref.watch(configNoticeProvider);
    if (message == null) return const SizedBox.shrink();
    return Material(
      color: Colors.blueGrey.shade800,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Row(
          children: [
            const Icon(Icons.restore, color: Colors.white),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                message,
                style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold),
              ),
            ),
            TextButton(
              onPressed: () => ref.read(configNoticeProvider.notifier).dismiss(),
              child: const Text('확인', style: TextStyle(color: Colors.white)),
            ),
          ],
        ),
      ),
    );
  }
}
