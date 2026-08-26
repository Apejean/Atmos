import re

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'r') as f:
    content = f.read()

# I need to change `_buildInputRow` to use a custom Stateful widget so it can handle text input.
# Or simpler: change the `Text('${value.toStringAsFixed(1)} m')` to a `TextField` but it should only appear on double click.
# So we need `_buildInputRow` to manage state, which means it MUST be a widget, not a method.

# Replace `_buildInputRow` method call in `build` method.
content = content.replace(
    '''                _buildInputRow(
                  icon: Icons.swap_horiz,
                  label: 'Width:',
                  value: width,
                  onUp: () => setState(() => width += 0.5),
                  onDown: () => setState(() => width -= 0.5),
                ),''',
    '''                _StepperInputRow(
                  icon: Icons.swap_horiz,
                  label: 'Width:',
                  value: width,
                  onChanged: (val) => setState(() => width = val),
                  step: 0.5,
                ),'''
)

content = content.replace(
    '''                _buildInputRow(
                  icon: Icons.view_in_ar, // Approximation for the perspective icon
                  label: 'Depth:',
                  value: depth,
                  onUp: () => setState(() => depth += 0.5),
                  onDown: () => setState(() => depth -= 0.5),
                ),''',
    '''                _StepperInputRow(
                  icon: Icons.view_in_ar,
                  label: 'Depth:',
                  value: depth,
                  onChanged: (val) => setState(() => depth = val),
                  step: 0.5,
                ),'''
)

content = content.replace(
    '''                _buildInputRow(
                  icon: Icons.height,
                  label: 'Ceiling Height:',
                  value: ceilingHeight,
                  onUp: () => setState(() => ceilingHeight += 0.1),
                  onDown: () => setState(() => ceilingHeight -= 0.1),
                ),''',
    '''                _StepperInputRow(
                  icon: Icons.height,
                  label: 'Ceiling Height:',
                  value: ceilingHeight,
                  onChanged: (val) => setState(() => ceilingHeight = val),
                  step: 0.1,
                ),'''
)

content = content.replace(
    '''                _buildInputRow(
                  icon: Icons.headphones_outlined,
                  label: 'Ear Level:',
                  value: earLevel,
                  onUp: () => setState(() => earLevel += 0.1),
                  onDown: () => setState(() => earLevel -= 0.1),
                ),''',
    '''                _StepperInputRow(
                  icon: Icons.headphones_outlined,
                  label: 'Ear Level:',
                  value: earLevel,
                  onChanged: (val) => setState(() => earLevel = val),
                  step: 0.1,
                ),'''
)

# Now delete the _buildInputRow method and add the _StepperInputRow class
method_start = content.find("Widget _buildInputRow({")
method_end = content.find("Widget _buildButton({", method_start)

# Delete method
content = content[:method_start] + content[method_end:]

# Add _StepperInputRow class at the end of the file
stepper_class = '''
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
        Icon(widget.icon, color: iconColor, size: 20),
        const SizedBox(width: 12),
        Expanded(
          child: Text(
            widget.label,
            style: const TextStyle(color: labelColor, fontSize: 13),
          ),
        ),
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
                  child: GestureDetector(
                    onDoubleTap: () {
                      setState(() {
                        _isEditing = true;
                      });
                      _focusNode.requestFocus();
                    },
                    child: _isEditing
                        ? TextField(
                            controller: _controller,
                            focusNode: _focusNode,
                            style: const TextStyle(
                              color: Colors.white,
                              fontSize: 13,
                              fontWeight: FontWeight.w500,
                            ),
                            decoration: const InputDecoration(
                              isDense: true,
                              contentPadding: EdgeInsets.zero,
                              border: InputBorder.none,
                            ),
                            keyboardType: const TextInputType.numberWithOptions(decimal: true),
                            onSubmitted: (_) => _submit(),
                          )
                        : Text(
                            '${widget.value.toStringAsFixed(1)} m',
                            textAlign: TextAlign.right,
                            style: const TextStyle(
                              color: Colors.lightBlueAccent,
                              fontSize: 13,
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                  ),
                ),
              ),
              Container(
                width: 24,
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
'''
content += stepper_class

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'w') as f:
    f.write(content)
