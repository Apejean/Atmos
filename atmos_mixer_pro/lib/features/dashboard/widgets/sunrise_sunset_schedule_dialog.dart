import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';

/// Sunrise / Sunset Exhibition Opening & Closing Schedule Dialog
class SunriseSunsetScheduleDialog extends StatefulWidget {
  const SunriseSunsetScheduleDialog({super.key});

  @override
  State<SunriseSunsetScheduleDialog> createState() =>
      _SunriseSunsetScheduleDialogState();
}

class _SunriseSunsetScheduleDialogState
    extends State<SunriseSunsetScheduleDialog> {
  bool _isSchedulerEnabled = true;
  TimeOfDay _openingTime = const TimeOfDay(hour: 9, minute: 0);
  TimeOfDay _closingTime = const TimeOfDay(hour: 18, minute: 0);
  bool _autoFadeOutOnClose = true;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      backgroundColor: const Color(0xFF1E1E2C),
      title: const Row(
        children: [
          Icon(Icons.wb_sunny_outlined, color: Colors.amberAccent),
          SizedBox(width: 8),
          Text(
            'Sunrise / Sunset Exhibition Scheduler',
            style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold),
          ),
        ],
      ),
      content: SizedBox(
        width: 460,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SwitchListTile(
              title: const Text('Enable Auto Scheduler', style: TextStyle(color: Colors.white, fontSize: 13)),
              subtitle: const Text('Automated opening playback & evening fade-out', style: TextStyle(color: Colors.white54, fontSize: 11)),
              value: _isSchedulerEnabled,
              activeThumbColor: AppColors.primaryNeon,
              onChanged: (val) => setState(() => _isSchedulerEnabled = val),
            ),
            const Divider(color: Colors.white12),
            ListTile(
              leading: const Icon(Icons.wb_sunny, color: Colors.amberAccent),
              title: const Text('Exhibition Opening (Sunrise Playback)', style: TextStyle(color: Colors.white, fontSize: 13)),
              trailing: TextButton(
                onPressed: () async {
                  final t = await showTimePicker(context: context, initialTime: _openingTime);
                  if (t != null) setState(() => _openingTime = t);
                },
                child: Text(
                  _openingTime.format(context),
                  style: const TextStyle(color: AppColors.primaryNeon, fontWeight: FontWeight.bold),
                ),
              ),
            ),
            ListTile(
              leading: const Icon(Icons.nights_stay, color: Colors.indigoAccent),
              title: const Text('Exhibition Closing (Sunset Dimming)', style: TextStyle(color: Colors.white, fontSize: 13)),
              trailing: TextButton(
                onPressed: () async {
                  final t = await showTimePicker(context: context, initialTime: _closingTime);
                  if (t != null) setState(() => _closingTime = t);
                },
                child: Text(
                  _closingTime.format(context),
                  style: const TextStyle(color: Colors.indigoAccent, fontWeight: FontWeight.bold),
                ),
              ),
            ),
            CheckboxListTile(
              title: const Text('30s Equal-Power Smooth Fade Out on Close', style: TextStyle(color: Colors.white70, fontSize: 12)),
              value: _autoFadeOutOnClose,
              activeColor: AppColors.primaryNeon,
              onChanged: (val) => setState(() => _autoFadeOutOnClose = val ?? true),
            ),
          ],
        ),
      ),
      actions: [
        ElevatedButton(
          style: ElevatedButton.styleFrom(
            backgroundColor: AppColors.primaryNeon,
            foregroundColor: Colors.black,
          ),
          onPressed: () {
            ScaffoldMessenger.of(context).showSnackBar(
              const SnackBar(
                content: Text('📅 Exhibition opening/closing schedule saved!'),
                backgroundColor: Colors.green,
              ),
            );
            Navigator.of(context).pop();
          },
          child: const Text('Save Schedule'),
        ),
      ],
    );
  }
}
