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
                    _buildRow('Width:', blueprintState.canvasWidthMeters, (v) {
                      ref.read(blueprintProvider.notifier).updateDimensions(canvasWidthMeters: v);
                    }),
                    const SizedBox(height: 12),
                    _buildRow('Depth:', blueprintState.canvasHeightMeters, (v) {
                      ref.read(blueprintProvider.notifier).updateDimensions(canvasHeightMeters: v);
                    }),
                    const SizedBox(height: 12),
                    _buildRow('Ceiling Height:', blueprintState.roomHeightMeters, (v) {
                      ref.read(blueprintProvider.notifier).updateDimensions(roomHeightMeters: v);
                    }),
                    const SizedBox(height: 12),
                    _buildRow('Ear Level:', blueprintState.listeningHeightMeters, (v) {
                      ref.read(blueprintProvider.notifier).updateDimensions(listeningHeightMeters: v);
                    }),
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

  Widget _buildRow(String label, double value, Function(double) onChanged) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(label, style: const TextStyle(color: Colors.white70, fontSize: 13)),
        SizedBox(
          width: 80,
          height: 24,
          child: TextFormField(
            initialValue: value.toStringAsFixed(1),
            style: const TextStyle(color: Colors.white, fontSize: 13, fontWeight: FontWeight.bold),
            keyboardType: const TextInputType.numberWithOptions(decimal: true),
            decoration: const InputDecoration(
              contentPadding: EdgeInsets.symmetric(horizontal: 8, vertical: 0),
              isDense: true,
              border: OutlineInputBorder(),
              suffixText: 'm',
              suffixStyle: TextStyle(color: Colors.white54, fontSize: 12),
            ),
            onChanged: (val) {
              final d = double.tryParse(val);
              if (d != null) onChanged(d);
            },
          ),
        ),
      ],
    );
  }
}
