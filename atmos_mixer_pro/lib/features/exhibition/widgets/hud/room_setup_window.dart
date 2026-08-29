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
  late TextEditingController _nameController;
  double width = 6.0;
  double depth = 4.5;
  double ceilingHeight = 3.0;
  double earLevel = 1.2;
  String materialName = 'Drywall / Glass (Standard Indoor)';
  double absorptionCoeff = 0.10;
  List<double> alphaOctaves = const [0.15, 0.12, 0.10, 0.08, 0.07, 0.06];

  @override
  void initState() {
    super.initState();
    _initValues(widget.room);
  }

  void _initValues(RoomZone? r) {
    _nameController = TextEditingController(text: r?.label ?? 'Room 1');
    if (r != null) {
      width = r.physicalWidth;
      depth = r.physicalHeight;
      ceilingHeight = r.ceilingHeight;
      earLevel = r.earLevel;
      materialName = r.materialName;
      absorptionCoeff = r.absorptionCoeff;
      alphaOctaves = r.alphaOctaves;
    }
  }

  @override
  void didUpdateWidget(RoomSetupWindow oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.room != oldWidget.room && widget.room != null) {
      setState(() {
        _nameController.text = widget.room!.label;
        width = widget.room!.physicalWidth;
        depth = widget.room!.physicalHeight;
        ceilingHeight = widget.room!.ceilingHeight;
        earLevel = widget.room!.earLevel;
        materialName = widget.room!.materialName;
        absorptionCoeff = widget.room!.absorptionCoeff;
        alphaOctaves = widget.room!.alphaOctaves;
      });
    }
  }

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  double get _currentEstimatedRt60 {
    final tempRoom = RoomZone(
      id: widget.room?.id ?? 'temp',
      label: _nameController.text,
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      color: 0xFF2196F3,
      physicalWidth: width,
      physicalHeight: depth,
      ceilingHeight: ceilingHeight,
      earLevel: earLevel,
      materialName: materialName,
      absorptionCoeff: absorptionCoeff,
      alphaOctaves: alphaOctaves,
    );
    return tempRoom.estimatedRt60;
  }

  @override
  Widget build(BuildContext context) {
    const bgColor = Color(0xFF1E2836);
    const borderColor = Color(0xFF384A62);
    const textLight = Color(0xFFE2E8F0);

    return Container(
      width: 280,
      decoration: BoxDecoration(
        color: bgColor.withValues(alpha: 0.96),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: borderColor, width: 1.2),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.6),
            blurRadius: 16,
            offset: const Offset(0, 6),
          ),
        ],
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Header
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
            decoration: const BoxDecoration(
              border: Border(bottom: BorderSide(color: borderColor, width: 1)),
            ),
            child: Row(
              children: [
                const Icon(Icons.tune_rounded, color: Colors.lightBlueAccent, size: 18),
                const SizedBox(width: 8),
                const Expanded(
                  child: Text(
                    'ROOM SETUP',
                    style: TextStyle(
                      color: textLight,
                      fontSize: 13,
                      fontWeight: FontWeight.bold,
                      letterSpacing: 0.5,
                    ),
                  ),
                ),
                InkWell(
                  onTap: widget.onClose,
                  borderRadius: BorderRadius.circular(12),
                  child: const Padding(
                    padding: EdgeInsets.all(2.0),
                    child: Icon(Icons.close, color: Colors.white60, size: 18),
                  ),
                ),
              ],
            ),
          ),

          // Body
          Padding(
            padding: const EdgeInsets.all(16.0),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // Room Name Input
                const Text(
                  'Room Name',
                  style: TextStyle(color: Colors.white70, fontSize: 11, fontWeight: FontWeight.w600),
                ),
                const SizedBox(height: 6),
                Container(
                  height: 34,
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                  decoration: BoxDecoration(
                    color: const Color(0xFF131A24),
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(color: Colors.white.withValues(alpha: 0.15)),
                  ),
                  child: TextField(
                    controller: _nameController,
                    style: const TextStyle(color: Colors.white, fontSize: 13),
                    decoration: const InputDecoration(
                      isDense: true,
                      contentPadding: EdgeInsets.symmetric(vertical: 8),
                      border: InputBorder.none,
                      hintText: 'e.g. Main Hall',
                      hintStyle: TextStyle(color: Colors.white38, fontSize: 12),
                    ),
                  ),
                ),

                const SizedBox(height: 14),
                const Divider(color: Colors.white12, height: 1),
                const SizedBox(height: 14),

                // Dimension Steppers
                _StepperInputRow(
                  icon: Icons.swap_horiz,
                  label: 'Width:',
                  value: width,
                  onChanged: (val) => setState(() => width = val.clamp(1.0, 1000.0)),
                  step: 0.5,
                ),
                const SizedBox(height: 10),
                _StepperInputRow(
                  icon: Icons.view_in_ar,
                  label: 'Depth:',
                  value: depth,
                  onChanged: (val) => setState(() => depth = val.clamp(1.0, 1000.0)),
                  step: 0.5,
                ),
                const SizedBox(height: 10),
                _StepperInputRow(
                  icon: Icons.height,
                  label: 'Ceiling Height:',
                  value: ceilingHeight,
                  onChanged: (val) => setState(() => ceilingHeight = val.clamp(1.5, 20.0)),
                  step: 0.1,
                ),
                const SizedBox(height: 10),
                _StepperInputRow(
                  icon: Icons.headphones_outlined,
                  label: 'Ear Level:',
                  value: earLevel,
                  onChanged: (val) => setState(() => earLevel = val.clamp(0.5, 3.0)),
                  step: 0.1,
                ),

                const SizedBox(height: 14),
                const Divider(color: Colors.white12, height: 1),
                const SizedBox(height: 12),

                // Material Preset
                const Text(
                  'Acoustic Material Preset',
                  style: TextStyle(color: Colors.white70, fontSize: 11, fontWeight: FontWeight.w600),
                ),
                const SizedBox(height: 6),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 2),
                  decoration: BoxDecoration(
                    color: const Color(0xFF131A24),
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(color: Colors.white.withValues(alpha: 0.15)),
                  ),
                  child: DropdownButtonHideUnderline(
                    child: DropdownButton<String>(
                      value: RoomMaterialPreset.presets.any((p) => p.name == materialName)
                          ? materialName
                          : RoomMaterialPreset.presets.first.name,
                      isExpanded: true,
                      dropdownColor: const Color(0xFF1B232E),
                      style: const TextStyle(color: Colors.white, fontSize: 11),
                      icon: const Icon(Icons.keyboard_arrow_down, color: Colors.white54, size: 18),
                      items: RoomMaterialPreset.presets.map((p) {
                        return DropdownMenuItem<String>(
                          value: p.name,
                          child: Text(
                            p.name,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(fontSize: 11),
                          ),
                        );
                      }).toList(),
                      onChanged: (val) {
                        if (val != null) {
                          final preset = RoomMaterialPreset.presets.firstWhere((p) => p.name == val);
                          setState(() {
                            materialName = preset.name;
                            absorptionCoeff = preset.averageAlpha;
                            alphaOctaves = preset.alphaOctaves;
                          });
                        }
                      },
                    ),
                  ),
                ),

                const SizedBox(height: 12),
                // Real-time RT60 Acoustic Badge
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                  decoration: BoxDecoration(
                    color: Colors.lightBlue.withValues(alpha: 0.1),
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(color: Colors.lightBlueAccent.withValues(alpha: 0.3)),
                  ),
                  child: Row(
                    children: [
                      const Icon(Icons.graphic_eq_rounded, size: 14, color: Colors.lightBlueAccent),
                      const SizedBox(width: 6),
                      Text(
                        'Estimated RT60: ${_currentEstimatedRt60.toStringAsFixed(2)}s',
                        style: const TextStyle(
                          color: Colors.lightBlueAccent,
                          fontSize: 11,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),

          // Footer (Buttons)
          Padding(
            padding: const EdgeInsets.only(right: 16.0, bottom: 16.0, left: 16.0),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                _buildButton(
                  label: 'Cancel',
                  isPrimary: false,
                  onPressed: widget.onClose ?? () {},
                ),
                const SizedBox(width: 8),
                _buildButton(
                  label: 'Apply',
                  isPrimary: true,
                  onPressed: () {
                    if (widget.room != null && widget.onApply != null) {
                      final updated = widget.room!.copyWith(
                        label: _nameController.text.trim().isEmpty ? 'Room' : _nameController.text.trim(),
                        physicalWidth: width,
                        physicalHeight: depth,
                        ceilingHeight: ceilingHeight,
                        earLevel: earLevel,
                        materialName: materialName,
                        absorptionCoeff: absorptionCoeff,
                        alphaOctaves: alphaOctaves,
                      );
                      widget.onApply!(updated);
                    }
                  },
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildButton({required String label, required bool isPrimary, required VoidCallback onPressed}) {
    final primaryColor = Colors.lightBlue.shade700;

    return InkWell(
      onTap: onPressed,
      borderRadius: BorderRadius.circular(6),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        decoration: BoxDecoration(
          color: isPrimary ? primaryColor : Colors.transparent,
          borderRadius: BorderRadius.circular(6),
          border: isPrimary ? null : Border.all(color: Colors.white24, width: 1),
        ),
        child: Text(
          label,
          style: TextStyle(
            color: isPrimary ? Colors.white : Colors.white70,
            fontSize: 12,
            fontWeight: isPrimary ? FontWeight.w600 : FontWeight.normal,
          ),
        ),
      ),
    );
  }
}

class _StepperInputRow extends StatefulWidget {
  final IconData icon;
  final String label;
  final double value;
  final Function(double) onChanged;
  final double step;

  const _StepperInputRow({
    required this.icon,
    required this.label,
    required this.value,
    required this.onChanged,
    required this.step,
  });

  @override
  State<_StepperInputRow> createState() => _StepperInputRowState();
}

class _StepperInputRowState extends State<_StepperInputRow> {
  bool _isEditing = false;
  late TextEditingController _controller;
  late FocusNode _focusNode;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.value.toStringAsFixed(1));
    _focusNode = FocusNode();
    _focusNode.addListener(() {
      if (!_focusNode.hasFocus && _isEditing) {
        _submit();
      }
    });
  }

  @override
  void didUpdateWidget(_StepperInputRow oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.value != widget.value && !_isEditing) {
      _controller.text = widget.value.toStringAsFixed(1);
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  void _submit() {
    final val = double.tryParse(_controller.text);
    if (val != null) {
      widget.onChanged(val);
    } else {
      _controller.text = widget.value.toStringAsFixed(1);
    }
    setState(() {
      _isEditing = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    const iconColor = Colors.lightBlueAccent;
    const labelColor = Color(0xFFE2E8F0);
    const boxBorderColor = Colors.lightBlueAccent;

    return Row(
      children: [
        Icon(widget.icon, color: iconColor, size: 18),
        const SizedBox(width: 8),
        Expanded(
          child: Text(
            widget.label,
            style: const TextStyle(color: labelColor, fontSize: 12),
          ),
        ),
        Container(
          width: 86,
          height: 30,
          decoration: BoxDecoration(
            color: const Color(0xFF131A24),
            borderRadius: BorderRadius.circular(4),
            border: Border.all(color: boxBorderColor.withValues(alpha: 0.6), width: 1.0),
          ),
          child: Row(
            children: [
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.only(left: 6.0, right: 2.0),
                  child: GestureDetector(
                    onDoubleTap: () {
                      setState(() {
                        _isEditing = true;
                      });
                      _focusNode.requestFocus();
                    },
                    child: _isEditing
                        ? Material(
                            type: MaterialType.transparency,
                            child: TextField(
                              controller: _controller,
                              focusNode: _focusNode,
                              textAlign: TextAlign.right,
                              style: const TextStyle(
                                color: Colors.white,
                                fontSize: 12,
                                fontWeight: FontWeight.w500,
                              ),
                              decoration: const InputDecoration(
                                isDense: true,
                                contentPadding: EdgeInsets.zero,
                                border: InputBorder.none,
                              ),
                              keyboardType: const TextInputType.numberWithOptions(decimal: true),
                              onSubmitted: (_) => _submit(),
                            ),
                          )
                        : Text(
                            '${widget.value.toStringAsFixed(1)}m',
                            textAlign: TextAlign.right,
                            style: const TextStyle(
                              color: Colors.lightBlueAccent,
                              fontSize: 12,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                  ),
                ),
              ),
              Container(
                width: 20,
                decoration: BoxDecoration(
                  border: Border(left: BorderSide(color: boxBorderColor.withValues(alpha: 0.3))),
                ),
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    InkWell(
                      onTap: () => widget.onChanged(widget.value + widget.step),
                      child: const Icon(Icons.arrow_drop_up, color: Colors.lightBlueAccent, size: 12),
                    ),
                    InkWell(
                      onTap: () => widget.onChanged(widget.value - widget.step),
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
}
