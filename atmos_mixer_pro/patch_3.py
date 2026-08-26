import re

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'r') as f:
    content = f.read()

target = """  Widget _buildRow(String label, String value) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(label, style: const TextStyle(color: Colors.white70, fontSize: 13)),
        Text(value, style: const TextStyle(color: Colors.white, fontSize: 13, fontWeight: FontWeight.bold)),
      ],
    );
  }"""
  
replacement = """  Widget _buildRow(String label, double value, Function(double) onChanged) {
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
content = content.replace(target, replacement)

target2 = """                    _buildRow('Width:', '${blueprintState.canvasWidthMeters.toStringAsFixed(1)} m'),
                    const SizedBox(height: 12),
                    _buildRow('Depth:', '${blueprintState.canvasHeightMeters.toStringAsFixed(1)} m'),
                    const SizedBox(height: 12),
                    _buildRow('Ceiling Height:', '3.0 m'),
                    const SizedBox(height: 12),
                    _buildRow('Ear Level:', '1.2 m'),"""

replacement2 = """                    _buildRow('Width:', blueprintState.canvasWidthMeters, (v) {
                      ref.read(blueprintProvider.notifier).update((s) => s.copyWith(canvasWidthMeters: v));
                    }),
                    const SizedBox(height: 12),
                    _buildRow('Depth:', blueprintState.canvasHeightMeters, (v) {
                      ref.read(blueprintProvider.notifier).update((s) => s.copyWith(canvasHeightMeters: v));
                    }),
                    const SizedBox(height: 12),
                    _buildRow('Ceiling Height:', blueprintState.roomHeightMeters, (v) {
                      ref.read(blueprintProvider.notifier).update((s) => s.copyWith(roomHeightMeters: v));
                    }),
                    const SizedBox(height: 12),
                    _buildRow('Ear Level:', blueprintState.listeningHeightMeters, (v) {
                      ref.read(blueprintProvider.notifier).update((s) => s.copyWith(listeningHeightMeters: v));
                    }),"""
content = content.replace(target2, replacement2)

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'w') as f:
    f.write(content)
