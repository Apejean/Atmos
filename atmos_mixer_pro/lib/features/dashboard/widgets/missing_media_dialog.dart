import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';

/// 찾지 못한 파일을 보여 주고, 파일이 있는 폴더를 고르게 한다(core/state/project_media.dart). [lead]는 목록 위
/// 설명, [skipLabel]은 고르지 않는 단추, [initialDirectory]는 폴더 선택 창을 처음 열 곳이다.
/// 고르지 않는 단추를 누르거나 폴더를 고르지 않으면 null.
Future<String?> askMediaFolder(
  BuildContext context,
  List<String> missing, {
  String lead = '프로젝트 폴더에 없는 오디오·도면 파일입니다. 파일이 있는 폴더를 고르면 '
      '그 폴더와 하위 폴더에서 파일 이름으로 다시 연결합니다.',
  String skipLabel = '그대로 열기',
  String? initialDirectory,
}) async {
  final choose = await showDialog<bool>(
    context: context,
    barrierDismissible: false,
    builder: (context) => AlertDialog(
      backgroundColor: AppColors.background,
      title: Text('파일 ${missing.length}개를 찾지 못했습니다', style: const TextStyle(color: Colors.white)),
      content: _MissingMediaList(missing: missing, lead: lead),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(skipLabel, style: const TextStyle(color: Colors.white70)),
        ),
        ElevatedButton(
          style: ElevatedButton.styleFrom(backgroundColor: AppColors.primaryBlue),
          onPressed: () => Navigator.of(context).pop(true),
          child: const Text('폴더 고르기', style: TextStyle(color: Colors.white)),
        ),
      ],
    ),
  );
  if (choose != true) return null;
  const title = '오디오·도면 파일이 있는 폴더';
  try {
    return await FilePicker.getDirectoryPath(dialogTitle: title, initialDirectory: initialDirectory);
  } catch (_) {
    // 시작 폴더를 열지 못하면(그사이 지워졌거나 형식이 다른 경로) 창이 정한 곳에서 연다.
    if (initialDirectory == null) rethrow;
    return FilePicker.getDirectoryPath(dialogTitle: title);
  }
}

/// 끝까지 찾지 못한 파일을 알린다. 프로젝트는 열린 상태다.
Future<void> showMissingMedia(BuildContext context, List<String> missing) => showDialog<void>(
      context: context,
      builder: (context) => AlertDialog(
        backgroundColor: AppColors.background,
        title: Text('파일 ${missing.length}개를 찾지 못한 채 열었습니다', style: const TextStyle(color: Colors.white)),
        content: _MissingMediaList(
          missing: missing,
          lead: '이 트랙은 재생되지 않고 도면은 보이지 않습니다. 파일을 프로젝트 폴더에 넣고 '
              '프로젝트를 다시 열면 연결됩니다.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('확인', style: TextStyle(color: Colors.white70)),
          ),
        ],
      ),
    );

class _MissingMediaList extends StatelessWidget {
  const _MissingMediaList({required this.missing, required this.lead});

  final List<String> missing;
  final String lead;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 520,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(lead, style: const TextStyle(color: Colors.white70)),
          const SizedBox(height: 12),
          ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 240),
            child: ListView(
              shrinkWrap: true,
              children: [
                for (final path in missing)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 2),
                    child: SelectableText(path, style: const TextStyle(color: Colors.white, fontSize: 12)),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
