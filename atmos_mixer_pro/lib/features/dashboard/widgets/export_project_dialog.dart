import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/state/project_export.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';

/// 프로젝트 내보내기 화면들(File > Export Project). 실제 복사와 저장은 core/state/project_export.dart.

/// 내보낼 폴더 이름을 묻는다. 취소하면 null.
Future<String?> askExportFolderName(
  BuildContext context, {
  required String parentDir,
  required String initial,
}) async {
  final controller = TextEditingController(text: initial);
  String? error;
  final name = await showDialog<String>(
    context: context,
    builder: (context) => StatefulBuilder(
      builder: (context, setState) {
        void submit() {
          final problem = validateExportFolderName(controller.text);
          if (problem != null) {
            setState(() => error = problem);
            return;
          }
          Navigator.of(context).pop(controller.text.trim());
        }

        return AlertDialog(
          backgroundColor: AppColors.background,
          title: const Text('프로젝트 내보내기', style: TextStyle(color: Colors.white)),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('위치: $parentDir', style: const TextStyle(color: Colors.white54, fontSize: 12)),
              const SizedBox(height: 12),
              TextField(
                controller: controller,
                autofocus: true,
                style: const TextStyle(color: Colors.white),
                decoration: InputDecoration(labelText: '폴더 이름', errorText: error),
                onSubmitted: (_) => submit(),
              ),
              const SizedBox(height: 8),
              const Text(
                '이 폴더에 project.atmos(설정·스피커·FX 등)와 음원(audio), 도면(drawing)을 모읍니다.\n'
                '다른 PC에서는 폴더를 통째로 옮겨 Load Project로 여세요.',
                style: TextStyle(color: Colors.white54, fontSize: 12),
              ),
            ],
          ),
          actions: [
            TextButton(onPressed: () => Navigator.of(context).pop(), child: const Text('취소')),
            TextButton(onPressed: submit, child: const Text('내보내기')),
          ],
        );
      },
    ),
  );
  controller.dispose();
  return name;
}

/// 같은 이름의 폴더가 이미 있을 때 계속할지 묻는다.
Future<bool> confirmExportIntoExisting(BuildContext context, String folder) async {
  final ok = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      backgroundColor: AppColors.background,
      title: const Text('폴더가 이미 있습니다', style: TextStyle(color: Colors.white)),
      content: Text(
        '$folder\n\n같은 이름의 파일은 덮어씁니다. 계속할까요?',
        style: const TextStyle(color: Colors.white70),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.of(context).pop(false), child: const Text('취소')),
        TextButton(onPressed: () => Navigator.of(context).pop(true), child: const Text('덮어쓰기')),
      ],
    ),
  );
  return ok ?? false;
}

/// 내보내는 동안 진행 상황을 보여 준다. 끝나면 부른 쪽이 Navigator로 닫는다.
void showExportProgress(BuildContext context, ValueNotifier<String> status) {
  showDialog<void>(
    context: context,
    barrierDismissible: false,
    builder: (context) => AlertDialog(
      backgroundColor: AppColors.background,
      content: Row(
        children: [
          const CircularProgressIndicator(color: AppColors.primaryNeon),
          const SizedBox(width: 16),
          Expanded(
            child: ValueListenableBuilder<String>(
              valueListenable: status,
              builder: (context, value, _) => Text(value, style: const TextStyle(color: Colors.white)),
            ),
          ),
        ],
      ),
    ),
  );
}

/// 내보내기 결과를 보여 준다.
Future<void> showExportResult(BuildContext context, ProjectExportResult result) {
  final missing = result.missing.isEmpty
      ? ''
      : '\n\n원본이 없어 복사하지 못한 파일 ${result.missing.length}개(프로젝트에는 원래 경로가 남습니다):\n'
          '${result.missing.join('\n')}';
  return showDialog<void>(
    context: context,
    builder: (context) => AlertDialog(
      backgroundColor: AppColors.background,
      title: const Text('내보내기 완료', style: TextStyle(color: Colors.white)),
      content: SingleChildScrollView(
        child: Text(
          '${result.folder}\n\n파일 ${result.copied}개를 모았습니다. '
          '이 폴더를 통째로 옮겨 Load Project로 project.atmos를 여세요.$missing',
          style: const TextStyle(color: Colors.white70),
        ),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.of(context).pop(), child: const Text('확인')),
      ],
    ),
  );
}
