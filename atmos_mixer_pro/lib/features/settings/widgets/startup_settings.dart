import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:flutter/material.dart';

/// 환경설정의 시작 설정. "Windows 시작 시 자동 실행"(Windows) · "macOS 시작 시 자동 실행"(macOS, [autostartLabel])을 끄면
/// 로그인해도 프로그램이 켜지지 않으므로 "로그인할 때 공연 자동 시작"은 바꿔도 소용이 없어 잠근다(값은 그대로 둔다).
/// [autostartHint]가 있으면 자동 실행 스위치를 잠그고 그 안내를 보인다(macOS에서 /Applications에 설치되지 않은 앱).
class StartupSettings extends StatelessWidget {
  const StartupSettings({
    super.key,
    required this.showWindowsAutostart,
    required this.windowsAutostart,
    required this.autoResumeOnLogon,
    required this.onWindowsAutostartChanged,
    required this.onAutoResumeOnLogonChanged,
    this.autostartLabel = 'Windows 시작 시\n자동 실행',
    this.autostartHint,
  });

  final bool showWindowsAutostart;
  final bool windowsAutostart;
  final bool autoResumeOnLogon;
  final ValueChanged<bool> onWindowsAutostartChanged;
  final ValueChanged<bool> onAutoResumeOnLogonChanged;
  final String autostartLabel;
  final String? autostartHint;

  @override
  Widget build(BuildContext context) {
    final resumeLocked = showWindowsAutostart && !windowsAutostart;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (showWindowsAutostart) ...[
          _row(autostartLabel, windowsAutostart, autostartHint == null ? onWindowsAutostartChanged : null),
          if (autostartHint != null)
            Padding(
              padding: const EdgeInsets.only(top: 4),
              child: Text(autostartHint!, style: const TextStyle(color: Colors.white38, fontSize: 11)),
            ),
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
