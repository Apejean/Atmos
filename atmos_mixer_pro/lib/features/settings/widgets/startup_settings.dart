import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:flutter/material.dart';

/// 환경설정의 시작 설정. "Windows 시작 시 자동 실행"(Windows만)을 끄면 로그인해도 프로그램이 켜지지 않으므로
/// "로그인할 때 공연 자동 시작"은 바꿔도 소용이 없어 잠근다(값은 그대로 둔다).
class StartupSettings extends StatelessWidget {
  const StartupSettings({
    super.key,
    required this.showWindowsAutostart,
    required this.windowsAutostart,
    required this.autoResumeOnLogon,
    required this.onWindowsAutostartChanged,
    required this.onAutoResumeOnLogonChanged,
  });

  final bool showWindowsAutostart;
  final bool windowsAutostart;
  final bool autoResumeOnLogon;
  final ValueChanged<bool> onWindowsAutostartChanged;
  final ValueChanged<bool> onAutoResumeOnLogonChanged;

  @override
  Widget build(BuildContext context) {
    final resumeLocked = showWindowsAutostart && !windowsAutostart;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (showWindowsAutostart) ...[
          _row('Windows 시작 시\n자동 실행', windowsAutostart, onWindowsAutostartChanged),
          const SizedBox(height: 16),
        ],
        _row('로그인할 때\n공연 자동 시작', autoResumeOnLogon, resumeLocked ? null : onAutoResumeOnLogonChanged),
      ],
    );
  }

  Widget _row(String label, bool value, ValueChanged<bool>? onChanged) {
    final enabled = onChanged != null;
    return Row(
      children: [
        SizedBox(
          width: 120,
          child: Text(
            label,
            style: TextStyle(
              color: enabled ? Colors.white70 : Colors.white38,
              fontWeight: FontWeight.bold,
              fontSize: 14,
            ),
          ),
        ),
        Switch(
          value: value,
          activeTrackColor: AppColors.primaryNeon.withValues(alpha: 0.5),
          activeThumbColor: AppColors.primaryNeon,
          onChanged: onChanged,
        ),
        const SizedBox(width: 8),
        Text(
          value ? 'On' : 'Off',
          style: TextStyle(
            color: !enabled
                ? Colors.white38
                : value
                    ? AppColors.primaryNeon
                    : Colors.white54,
            fontWeight: FontWeight.bold,
          ),
        ),
      ],
    );
  }
}
