import 'dart:ui';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/trajectory_state.dart';

class TrajectoryEditorToolbar extends ConsumerWidget {
  const TrajectoryEditorToolbar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final activeId = ref.watch(activeTrajectoryIdProvider);
    if (activeId == null) return const SizedBox.shrink();

    return Align(
      alignment: Alignment.bottomCenter,
      child: Container(
        margin: const EdgeInsets.only(bottom: 24),
        child: ClipRRect(
          borderRadius: BorderRadius.circular(24),
          child: BackdropFilter(
            filter: ImageFilter.blur(sigmaX: 16, sigmaY: 16),
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              decoration: BoxDecoration(
                color: Colors.black.withValues(alpha: 0.6),
                border: Border.all(color: Colors.white24),
                borderRadius: BorderRadius.circular(24),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  _ToolbarButton(
                    icon: Icons.edit,
                    label: 'Draw',
                    iconColor: ref.watch(isDrawingModeProvider) ? AppColors.primaryNeon : Colors.white70,
                    onTap: () {
                      ref.read(isDrawingModeProvider.notifier).toggle();
                    },
                  ),
                  const SizedBox(width: 8),
                  _ToolbarButton(
                    icon: Icons.category,
                    label: 'Preset',
                    onTap: () {
                      // TODO: Implement preset shapes
                    },
                  ),
                  const SizedBox(width: 8),
                  _ToolbarButton(
                    icon: Icons.fiber_manual_record,
                    iconColor: Colors.redAccent,
                    label: 'Live Rec',
                    onTap: () {
                      // TODO: Implement live recording
                    },
                  ),
                  const SizedBox(width: 8),
                  Container(width: 1, height: 24, color: Colors.white24),
                  const SizedBox(width: 8),
                  _ToolbarButton(
                    icon: Icons.height,
                    label: 'Z-Height',
                    onTap: () {
                      // TODO: Implement Z-Height control
                    },
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _ToolbarButton extends StatelessWidget {
  final IconData icon;
  final String label;
  final VoidCallback onTap;
  final Color? iconColor;

  const _ToolbarButton({
    required this.icon,
    required this.label,
    required this.onTap,
    this.iconColor,
  });

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(8),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              icon,
              color: iconColor ?? Colors.white70,
              size: 20,
            ),
            const SizedBox(height: 2),
            Text(
              label,
              style: const TextStyle(
                color: Colors.white70,
                fontSize: 10,
                fontWeight: FontWeight.w500,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
