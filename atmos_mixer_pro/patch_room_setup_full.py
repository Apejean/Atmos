import re

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'r') as f:
    content = f.read()

target_build_row = """  Widget _buildRow(String label, String value) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(
          label,
          style: const TextStyle(color: Colors.white70, fontSize: 13),
        ),
        Text(
          value,
          style: const TextStyle(
            color: Colors.white,
            fontSize: 13,
            fontWeight: FontWeight.bold,
          ),
        ),
      ],
    );
  }"""
  
replacement_build_row = """  Widget _buildRow(String label, double value, Function(double) onChanged) {
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
  }"""
  
# Remove multiple spaces to match safely
import re
content = re.sub(r'  Widget _buildRow\(String label, String value\) \{[\s\S]*?    \);\n  \}', replacement_build_row, content)

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'w') as f:
    f.write(content)
