import 'package:flutter/material.dart';

import 'package:atmos_mixer_pro/features/exhibition/models/room_zone.dart';

class RoomSetupWindow extends StatefulWidget {
  final RoomZone? room;
  final Function(RoomZone)? onApply;
  final VoidCallback? onClose;

  const RoomSetupWindow({super.key, this.room, this.onApply, this.onClose});

  @override
  State<RoomSetupWindow> createState() => _RoomSetupWindowState();
}

class _RoomSetupWindowState extends State<RoomSetupWindow> {
  double width = 6.0;
  double depth = 4.5;
  double ceilingHeight = 3.0;
  double earLevel = 1.2;

  @override
  void initState() {
    super.initState();
    if (widget.room != null) {
      width = widget.room!.physicalWidth;
      depth = widget.room!.physicalHeight;
      ceilingHeight = widget.room!.ceilingHeight;
      earLevel = widget.room!.earLevel;
    }
  }

  @override
  void didUpdateWidget(RoomSetupWindow oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.room != oldWidget.room && widget.room != null) {
      setState(() {
        width = widget.room!.physicalWidth;
        depth = widget.room!.physicalHeight;
        ceilingHeight = widget.room!.ceilingHeight;
        earLevel = widget.room!.earLevel;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    const bgColor = Color(0xFF2C394B); // Deep blue-grey from mockup
    const borderColor = Color(0xFF3F556D);
    const textLight = Color(0xFFE2E8F0);

    return Container(
      width: 260,
      decoration: BoxDecoration(
        color: bgColor.withValues(alpha: 0.95),
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: borderColor, width: 1),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.5),
            blurRadius: 10,
            offset: const Offset(0, 4),
          ),
        ],
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          // Header
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            decoration: BoxDecoration(
              border: Border(bottom: BorderSide(color: borderColor, width: 1)),
            ),
            child: Row(
              children: [
                const Icon(Icons.picture_in_picture_alt_outlined, color: Colors.lightBlueAccent, size: 18),
                const SizedBox(width: 8),
                const Expanded(
                  child: Text(
                    'ROOM SETUP',
                    style: TextStyle(
                      color: textLight,
                      fontSize: 13,
                      fontWeight: FontWeight.w600,
                      letterSpacing: 0.5,
                    ),
                  ),
                ),
                InkWell(
                  onTap: widget.onClose,
                  child: const Icon(Icons.close, color: Colors.white54, size: 18),
                ),
              ],
            ),
          ),

          // Body
          Padding(
            padding: const EdgeInsets.all(16.0),
            child: Column(
              children: [
                _buildInputRow(
                  icon: Icons.swap_horiz,
                  label: 'Width:',
                  value: width,
                  onUp: () => setState(() => width += 0.5),
                  onDown: () => setState(() => width -= 0.5),
                ),
                const SizedBox(height: 12),
                _buildInputRow(
                  icon: Icons.view_in_ar, // Approximation for the perspective icon
                  label: 'Depth:',
                  value: depth,
                  onUp: () => setState(() => depth += 0.5),
                  onDown: () => setState(() => depth -= 0.5),
                ),
                const SizedBox(height: 12),
                _buildInputRow(
                  icon: Icons.height,
                  label: 'Ceiling Height:',
                  value: ceilingHeight,
                  onUp: () => setState(() => ceilingHeight += 0.1),
                  onDown: () => setState(() => ceilingHeight -= 0.1),
                ),
                const SizedBox(height: 12),
                _buildInputRow(
                  icon: Icons.headphones_outlined,
                  label: 'Ear Level:',
                  value: earLevel,
                  onUp: () => setState(() => earLevel += 0.1),
                  onDown: () => setState(() => earLevel -= 0.1),
                ),
              ],
            ),
          ),

          // Footer (Buttons)
          Padding(
            padding: const EdgeInsets.only(right: 16.0, bottom: 16.0),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                _buildButton(
                  label: 'Apply',
                  isPrimary: true,
                  onPressed: () {
                    if (widget.room != null && widget.onApply != null) {
                      final updated = widget.room!.copyWith(
                        physicalWidth: width,
                        physicalHeight: depth,
                        ceilingHeight: ceilingHeight,
                        earLevel: earLevel,
                      );
                      widget.onApply!(updated);
                    }
                  },
                ),
                const SizedBox(width: 8),
                _buildButton(
                  label: 'Cancel',
                  isPrimary: false,
                  onPressed: widget.onClose ?? () {},
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildInputRow({
    required IconData icon,
    required String label,
    required double value,
    required VoidCallback onUp,
    required VoidCallback onDown,
  }) {
    const iconColor = Colors.lightBlueAccent;
    const labelColor = Color(0xFFE2E8F0);
    const boxBorderColor = Colors.lightBlueAccent;

    return Row(
      children: [
        Icon(icon, color: iconColor, size: 20),
        const SizedBox(width: 12),
        Expanded(
          child: Text(
            label,
            style: const TextStyle(color: labelColor, fontSize: 13),
          ),
        ),
        // Custom Stepper Input Box
        Container(
          width: 90,
          height: 32,
          decoration: BoxDecoration(
            color: Colors.transparent,
            borderRadius: BorderRadius.circular(4),
            border: Border.all(color: boxBorderColor.withValues(alpha: 0.7), width: 1.2),
          ),
          child: Row(
            children: [
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.only(left: 8.0, right: 4.0),
                  child: Text(
                    '${value.toStringAsFixed(1)} m',
                    textAlign: TextAlign.right,
                    style: const TextStyle(
                      color: Colors.lightBlueAccent,
                      fontSize: 13,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ),
              ),
              // Stepper Arrows
              Container(
                width: 24,
                decoration: BoxDecoration(
                  border: Border(left: BorderSide(color: boxBorderColor.withValues(alpha: 0.3))),
                ),
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    InkWell(
                      onTap: onUp,
                      child: const Icon(Icons.arrow_drop_up, color: Colors.lightBlueAccent, size: 12),
                    ),
                    InkWell(
                      onTap: onDown,
                      child: const Icon(Icons.arrow_drop_down, color: Colors.lightBlueAccent, size: 12),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildButton({required String label, required bool isPrimary, required VoidCallback onPressed}) {
    final primaryColor = Colors.lightBlue.shade700;
    
    return InkWell(
      onTap: onPressed,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
        decoration: BoxDecoration(
          color: isPrimary ? primaryColor : Colors.transparent,
          borderRadius: BorderRadius.circular(4),
          border: isPrimary ? null : Border.all(color: Colors.white24, width: 1),
        ),
        child: Text(
          label,
          style: TextStyle(
            color: isPrimary ? Colors.white : Colors.white70,
            fontSize: 12,
            fontWeight: isPrimary ? FontWeight.w500 : FontWeight.normal,
          ),
        ),
      ),
    );
  }
}
