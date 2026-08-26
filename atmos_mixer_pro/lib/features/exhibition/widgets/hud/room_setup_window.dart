import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/room_zone_state.dart';
import 'package:atmos_mixer_pro/features/exhibition/state/blueprint_state.dart';

class RoomSetupFloatingWindow extends ConsumerStatefulWidget {
  const RoomSetupFloatingWindow({super.key});

  @override
  ConsumerState<RoomSetupFloatingWindow> createState() =>
      _RoomSetupFloatingWindowState();
}

class _RoomSetupFloatingWindowState extends ConsumerState<RoomSetupFloatingWindow> {
  double _x = 20.0;
  double _y = 20.0;
  bool _isVisible = true;

  @override
  Widget build(BuildContext context) {
    if (!_isVisible) return const SizedBox.shrink();
    final blueprintState = ref.watch(blueprintProvider);
    
    return Positioned(
      left: _x,
      top: _y,
      child: GestureDetector(
        onPanUpdate: (details) {
          setState(() {
            _x += details.delta.dx;
            _y += details.delta.dy;
          });
        },
        child: Container(
          width: 250,
          decoration: BoxDecoration(
            color: AppColors.cardSurfaceSolid.withValues(alpha: 0.9),
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: AppColors.lightGrey, width: 1.5),
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              // Header
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                decoration: const BoxDecoration(
                  border: Border(
                    bottom: BorderSide(color: AppColors.lightGrey, width: 1),
                  ),
                ),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    const Row(
                      children: [
                        Icon(Icons.dashboard_customize, size: 16, color: Colors.white70),
                        SizedBox(width: 8),
                        Text(
                          'ROOM SETUP',
                          style: TextStyle(
                            color: Colors.white,
                            fontSize: 14,
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                      ],
                    ),
                    InkWell(
                      onTap: () => setState(() => _isVisible = false),
                      child: const Icon(Icons.close, size: 16, color: Colors.white54),
                    ),
                  ],
                ),
              ),
              // Body
              Padding(
                padding: const EdgeInsets.all(16.0),
                child: Column(
                  children: [
                    _buildRow('Width:', '${blueprintState.canvasWidthMeters.toStringAsFixed(1)} m'),
                    const SizedBox(height: 12),
                    _buildRow('Depth:', '${blueprintState.canvasHeightMeters.toStringAsFixed(1)} m'),
                    const SizedBox(height: 12),
                    _buildRow('Ceiling Height:', '3.0 m'),
                    const SizedBox(height: 12),
                    _buildRow('Ear Level:', '1.2 m'),
                  ],
                ),
              ),
              // Footer
              Padding(
                padding: const EdgeInsets.only(bottom: 12.0, right: 12.0),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: [
                    TextButton(
                      onPressed: () {},
                      child: const Text('Cancel', style: TextStyle(color: Colors.white54)),
                    ),
                    const SizedBox(width: 8),
                    ElevatedButton(
                      style: ElevatedButton.styleFrom(
                        backgroundColor: AppColors.primaryBlue,
                        foregroundColor: Colors.white,
                        minimumSize: const Size(60, 32),
                        padding: const EdgeInsets.symmetric(horizontal: 16),
                      ),
                      onPressed: () {},
                      child: const Text('Apply'),
                    ),
                  ],
                ),
              )
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildRow(String label, String value) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(
          label,
          style: const TextStyle(color: Colors.white70, fontSize: 13),
        ),
        Container(
          width: 80,
          height: 28,
          decoration: BoxDecoration(
            border: Border.all(color: AppColors.primaryBlue),
            borderRadius: BorderRadius.circular(4),
          ),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Padding(
                padding: const EdgeInsets.only(left: 8.0),
                child: Text(
                  value,
                  style: const TextStyle(color: Colors.white, fontSize: 13),
                ),
              ),
              const Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(Icons.arrow_drop_up, size: 12, color: AppColors.primaryBlue),
                  Icon(Icons.arrow_drop_down, size: 12, color: AppColors.primaryBlue),
                ],
              )
            ],
          ),
        ),
      ],
    );
  }
}
