import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';

/// Watchdog & EBU R128 AutoGuard Safety Alert Border Widget
/// Highlights screen edges with a blinking Red / Yellow warning border when
/// Watchdog error recovery or EBU R128 loudness auto-attenuation is active.
class SafetyAlertBorderWidget extends StatefulWidget {
  final Widget child;
  final bool isWatchdogActive;
  final bool isAutoGuardActive;
  final String? alertMessage;

  const SafetyAlertBorderWidget({
    super.key,
    required this.child,
    this.isWatchdogActive = false,
    this.isAutoGuardActive = false,
    this.alertMessage,
  });

  @override
  State<SafetyAlertBorderWidget> createState() =>
      _SafetyAlertBorderWidgetState();
}

class _SafetyAlertBorderWidgetState extends State<SafetyAlertBorderWidget>
    with SingleTickerProviderStateMixin {
  late AnimationController _blinkController;
  late Animation<double> _opacityAnim;

  @override
  void initState() {
    super.initState();
    _blinkController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 600),
    )..repeat(reverse: true);

    _opacityAnim = Tween<double>(begin: 0.3, end: 1.0).animate(
      CurvedAnimation(parent: _blinkController, curve: Curves.easeInOut),
    );
  }

  @override
  void dispose() {
    _blinkController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final bool hasAlert = widget.isWatchdogActive || widget.isAutoGuardActive;

    if (!hasAlert) {
      return widget.child;
    }

    final Color alertColor = widget.isWatchdogActive
        ? AppColors.danger
        : Colors.amberAccent;

    final String title = widget.isWatchdogActive
        ? '⚠️ WATCHDOG ACTIVE (Error Recovery & Auto-Heal)'
        : '🛡️ AUTOGUARD ACTIVE (EBU R128 Loudness Attenuation)';

    return Stack(
      children: [
        widget.child,
        // Blinking Alert Border
        Positioned.fill(
          child: IgnorePointer(
            child: AnimatedBuilder(
              animation: _opacityAnim,
              builder: (context, _) {
                return Container(
                  decoration: BoxDecoration(
                    border: Border.all(
                      color: alertColor.withValues(alpha: _opacityAnim.value),
                      width: 4.0,
                    ),
                  ),
                );
              },
            ),
          ),
        ),
        // Top Banner Notification
        Positioned(
          top: 0,
          left: 0,
          right: 0,
          child: Material(
            color: Colors.transparent,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              color: alertColor.withValues(alpha: 0.95),
              child: Row(
                children: [
                  Icon(
                    widget.isWatchdogActive ? Icons.shield_outlined : Icons.volume_down,
                    color: Colors.black,
                    size: 20,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      '$title ${widget.alertMessage ?? ''}',
                      style: const TextStyle(
                        color: Colors.black,
                        fontSize: 12,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }
}
