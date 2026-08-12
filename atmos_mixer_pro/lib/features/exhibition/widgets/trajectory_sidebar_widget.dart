import 'dart:ui';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/trajectory_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';

class TrajectorySidebarWidget extends ConsumerStatefulWidget {
  final VoidCallback onClose;

  const TrajectorySidebarWidget({
    super.key,
    required this.onClose,
  });

  @override
  ConsumerState<TrajectorySidebarWidget> createState() => _TrajectorySidebarWidgetState();
}

class _TrajectorySidebarWidgetState extends ConsumerState<TrajectorySidebarWidget> {
  bool _isObjectMode = true; // Object Mode vs Channel Mode

  @override
  Widget build(BuildContext context) {
    final trajectories = ref.watch(trajectoryProvider);
    final rooms = ref.watch(roomZoneProvider);

    return Align(
      alignment: Alignment.centerRight,
      child: Container(
        width: 340,
        margin: const EdgeInsets.only(top: 16, bottom: 16, right: 16),
        child: ClipRRect(
          borderRadius: BorderRadius.circular(16),
          child: BackdropFilter(
            filter: ImageFilter.blur(sigmaX: 16, sigmaY: 16),
            child: Container(
              decoration: BoxDecoration(
                color: Colors.black.withValues(alpha: 0.6),
                border: Border.all(color: Colors.white24),
                borderRadius: BorderRadius.circular(16),
              ),
              child: Column(
                children: [
                  _buildHeader(),
                  const Divider(color: Colors.white24, height: 1),
                  _buildModeToggle(),
                  const Divider(color: Colors.white24, height: 1),
                  Expanded(
                    child: ListView(
                      padding: const EdgeInsets.all(16),
                      children: [
                        _buildRoomList(rooms),
                        const SizedBox(height: 24),
                        _buildTrajectoriesList(trajectories),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildHeader() {
    return Padding(
      padding: const EdgeInsets.all(16.0),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          const Text(
            'Routing & Trajectories',
            style: TextStyle(
              color: Colors.white,
              fontSize: 16,
              fontWeight: FontWeight.bold,
            ),
          ),
          IconButton(
            icon: const Icon(Icons.close, color: Colors.white54, size: 20),
            onPressed: widget.onClose,
            padding: EdgeInsets.zero,
            constraints: const BoxConstraints(),
          ),
        ],
      ),
    );
  }

  Widget _buildModeToggle() {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16.0, vertical: 12.0),
      child: Row(
        children: [
          Expanded(
            child: _ModeButton(
              title: 'Channel Mode',
              isSelected: !_isObjectMode,
              onTap: () => setState(() => _isObjectMode = false),
            ),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: _ModeButton(
              title: 'Object Mode',
              isSelected: _isObjectMode,
              onTap: () => setState(() => _isObjectMode = true),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildRoomList(final List rooms) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text(
          'Room Zones',
          style: TextStyle(
            color: AppColors.primaryNeon,
            fontWeight: FontWeight.bold,
            fontSize: 14,
          ),
        ),
        const SizedBox(height: 12),
        if (rooms.isEmpty)
          const Text('No rooms configured.', style: TextStyle(color: Colors.white54, fontSize: 12))
        else
          ...rooms.map((room) => Container(
                margin: const EdgeInsets.only(bottom: 8),
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: Colors.white.withValues(alpha: 0.05),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  children: [
                    Container(
                      width: 12,
                      height: 12,
                      decoration: BoxDecoration(
                        color: Color(room.color),
                        shape: BoxShape.circle,
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        room.name,
                        style: const TextStyle(color: Colors.white, fontSize: 14),
                      ),
                    ),
                  ],
                ),
              )),
      ],
    );
  }

  Widget _buildTrajectoriesList(final List trajectories) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text(
          'Trajectories (Object Mode)',
          style: TextStyle(
            color: AppColors.primaryNeon,
            fontWeight: FontWeight.bold,
            fontSize: 14,
          ),
        ),
        const SizedBox(height: 12),
        if (trajectories.isEmpty)
          const Text('No trajectories available.', style: TextStyle(color: Colors.white54, fontSize: 12))
        else
          ...trajectories.map((traj) {
            return Container(
              margin: const EdgeInsets.only(bottom: 12),
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: Colors.white.withValues(alpha: 0.05),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: traj.color.withValues(alpha: 0.5)),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Text(
                        traj.name,
                        style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold),
                      ),
                      Switch(
                        value: traj.isVisible,
                        onChanged: (val) {
                          ref.read(trajectoryProvider.notifier).updateTrajectory(
                            traj.copyWith(isVisible: val),
                          );
                        },
                        activeThumbColor: AppColors.primaryNeon,
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Row(
                    children: [
                      const Text('Speed', style: TextStyle(color: Colors.white70, fontSize: 12)),
                      Expanded(
                        child: Slider(
                          value: traj.speed,
                          min: 0.1,
                          max: 10.0,
                          activeColor: traj.color,
                          onChanged: (val) {
                            ref.read(trajectoryProvider.notifier).updateTrajectory(
                              traj.copyWith(speed: val),
                            );
                          },
                        ),
                      ),
                      Text(traj.speed.toStringAsFixed(1), style: const TextStyle(color: Colors.white54, fontSize: 12)),
                    ],
                  ),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      const Text('Ping-Pong (Loop)', style: TextStyle(color: Colors.white70, fontSize: 12)),
                      Checkbox(
                        value: traj.isPingPong,
                        activeColor: traj.color,
                        onChanged: (val) {
                          if (val != null) {
                            ref.read(trajectoryProvider.notifier).updateTrajectory(
                              traj.copyWith(isPingPong: val),
                            );
                          }
                        },
                      ),
                    ],
                  ),
                  const SizedBox(height: 4),
                  Text(
                    'Waypoints: ${traj.waypoints.length}',
                    style: const TextStyle(color: Colors.white54, fontSize: 11),
                  ),
                ],
              ),
            );
          }),
      ],
    );
  }
}

class _ModeButton extends StatelessWidget {
  final String title;
  final bool isSelected;
  final VoidCallback onTap;

  const _ModeButton({
    required this.title,
    required this.isSelected,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(vertical: 8),
        decoration: BoxDecoration(
          color: isSelected ? AppColors.primaryNeon.withValues(alpha: 0.2) : Colors.transparent,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isSelected ? AppColors.primaryNeon : Colors.white24,
          ),
        ),
        alignment: Alignment.center,
        child: Text(
          title,
          style: TextStyle(
            color: isSelected ? AppColors.primaryNeon : Colors.white54,
            fontWeight: FontWeight.bold,
            fontSize: 12,
          ),
        ),
      ),
    );
  }
}
