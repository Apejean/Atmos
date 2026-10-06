import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';

/// 프로젝트 폴더에서 찾지 못한 파일을 보여 주고, 파일이 있는 폴더를 고르게 한다(core/state/project_media.dart).
/// "그대로 열기"를 누르거나 폴더를 고르지 않으면 null.
Future<String?> askMediaFolder(BuildContext context, List<String> missing) async {
  final choose = await showDialog<bool>(
    context: context,
    barrierDismissible: false,
    builder: (context) => AlertDialog(
      backgroundColor: AppColors.background,
      title: Text('파일 ${missing.length}개를 찾지 못했습니다', style: const TextStyle(color: Colors.white)),
      content: _MissingMediaList(
        missing: missing,
        lead: '프로젝트 폴더에 없는 오디오·도면 파일입니다. 파일이 있는 폴더를 고르면 '
            '그 폴더와 하위 폴더에서 파일 이름으로 다시 연결합니다.',
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text('그대로 열기', style: TextStyle(color: Colors.white70)),
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
  return FilePicker.getDirectoryPath(dialogTitle: '오디오·도면 파일이 있는 폴더');
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
