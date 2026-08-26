import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/features/dashboard/state/output_routing_state.dart';

class OutputRoutingMatrixModal extends ConsumerWidget {
  const OutputRoutingMatrixModal({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final channels = ref.watch(outputRoutingProvider);

    return Dialog(
      backgroundColor: Colors.transparent,
      child: Container(
        width: 1000,
        height: 800,
        decoration: BoxDecoration(
          color: AppColors.background,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: Colors.white12),
        ),
        child: Column(
          children: [
            // Header
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 16),
              decoration: const BoxDecoration(
                border: Border(bottom: BorderSide(color: Colors.white12)),
              ),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  const Text('Output Routing Matrix', style: TextStyle(color: Colors.white, fontSize: 18, fontWeight: FontWeight.bold)),
                  IconButton(
                    icon: const Icon(Icons.close, color: Colors.white54),
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                ],
              ),
            ),
            
            // Table Header
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
              color: Colors.white.withValues(alpha: 0.05),
              child: const Row(
                children: [
                  SizedBox(width: 40, child: Text('Ch', style: TextStyle(color: Colors.white70, fontWeight: FontWeight.bold))),
                  SizedBox(width: 120, child: Text('Name', style: TextStyle(color: Colors.white70, fontWeight: FontWeight.bold))),
                  SizedBox(width: 100, child: Text('Mute/Solo', style: TextStyle(color: Colors.white70, fontWeight: FontWeight.bold))),
                  SizedBox(width: 60, child: Text('Phase', style: TextStyle(color: Colors.white70, fontWeight: FontWeight.bold))),
                  SizedBox(width: 100, child: Text('Delay (ms)', style: TextStyle(color: Colors.white70, fontWeight: FontWeight.bold))),
                  Expanded(child: Text('Gain (dB)', style: TextStyle(color: Colors.white70, fontWeight: FontWeight.bold))),
                  SizedBox(width: 80, child: Text('EQ', style: TextStyle(color: Colors.white70, fontWeight: FontWeight.bold))),
                ],
              ),
            ),
            
            // Table Body
            Expanded(
              child: ListView.builder(
                itemCount: channels.length,
                itemBuilder: (context, index) {
                  return _OutputChannelRow(channel: channels[index]);
                },
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _OutputChannelRow extends ConsumerStatefulWidget {
  final OutputChannelModel channel;

  const _OutputChannelRow({required this.channel});

  @override
  ConsumerState<_OutputChannelRow> createState() => _OutputChannelRowState();
}

class _OutputChannelRowState extends ConsumerState<_OutputChannelRow> {
  late TextEditingController _delayController;
  late TextEditingController _gainController;

  @override
  void initState() {
    super.initState();
    _delayController = TextEditingController(text: widget.channel.delayMs.toStringAsFixed(1));
    _gainController = TextEditingController(text: widget.channel.gainDb.toStringAsFixed(1));
  }

  @override
  void didUpdateWidget(covariant _OutputChannelRow oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.channel.delayMs != widget.channel.delayMs && !_delayController.text.endsWith('.')) {
      _delayController.text = widget.channel.delayMs.toStringAsFixed(1);
    }
    if (oldWidget.channel.gainDb != widget.channel.gainDb && !_gainController.text.endsWith('.')) {
      _gainController.text = widget.channel.gainDb.toStringAsFixed(1);
    }
  }

  @override
  void dispose() {
    _delayController.dispose();
    _gainController.dispose();
    super.dispose();
  }

  void _update(OutputChannelModel updated) {
    ref.read(outputRoutingProvider.notifier).updateChannel(updated);
  }

  void _submitDelay(String value) {
    final parsed = double.tryParse(value);
    if (parsed != null) {
      _update(widget.channel.copyWith(delayMs: parsed.clamp(0.0, 500.0)));
    } else {
      _delayController.text = widget.channel.delayMs.toStringAsFixed(1);
    }
  }

  void _submitGain(String value) {
    final parsed = double.tryParse(value);
    if (parsed != null) {
      _update(widget.channel.copyWith(gainDb: parsed.clamp(-60.0, 12.0)));
    } else {
      _gainController.text = widget.channel.gainDb.toStringAsFixed(1);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      decoration: const BoxDecoration(
        border: Border(bottom: BorderSide(color: Colors.white10)),
      ),
      child: Row(
        children: [
          SizedBox(
            width: 40,
            child: Text(
              widget.channel.id.toString(),
              style: const TextStyle(color: Colors.white54, fontWeight: FontWeight.bold),
            ),
          ),
          SizedBox(
            width: 120,
            child: Text(
              widget.channel.name,
              style: const TextStyle(color: Colors.white),
              overflow: TextOverflow.ellipsis,
            ),
          ),
          SizedBox(
            width: 100,
            child: Row(
              children: [
                _ToggleButton(
                  label: 'M',
                  isActive: widget.channel.isMuted,
                  activeColor: Colors.redAccent,
                  onTap: () => _update(widget.channel.copyWith(isMuted: !widget.channel.isMuted)),
                ),
                const SizedBox(width: 8),
                _ToggleButton(
                  label: 'S',
                  isActive: widget.channel.isSoloed,
                  activeColor: Colors.amberAccent,
                  onTap: () => _update(widget.channel.copyWith(isSoloed: !widget.channel.isSoloed)),
                ),
              ],
            ),
          ),
          SizedBox(
            width: 60,
            child: _ToggleButton(
              label: 'ø',
              isActive: widget.channel.isPhaseInverted,
              activeColor: Colors.blueAccent,
              onTap: () => _update(widget.channel.copyWith(isPhaseInverted: !widget.channel.isPhaseInverted)),
            ),
          ),
          SizedBox(
            width: 100,
            child: Padding(
              padding: const EdgeInsets.only(right: 16.0),
              child: TextField(
                controller: _delayController,
                style: const TextStyle(color: Colors.white, fontSize: 12),
                decoration: const InputDecoration(
                  isDense: true,
                  contentPadding: EdgeInsets.symmetric(horizontal: 8, vertical: 8),
                  border: OutlineInputBorder(),
                  focusedBorder: OutlineInputBorder(borderSide: BorderSide(color: AppColors.primaryNeon)),
                ),
                keyboardType: TextInputType.number,
                textInputAction: TextInputAction.next,
                onSubmitted: _submitDelay,
                onTapOutside: (_) => _submitDelay(_delayController.text),
              ),
            ),
          ),
          Expanded(
            child: Row(
              children: [
                SizedBox(
                  width: 80,
                  child: TextField(
                    controller: _gainController,
                    style: const TextStyle(color: Colors.white, fontSize: 12),
                    decoration: const InputDecoration(
                      isDense: true,
                      contentPadding: EdgeInsets.symmetric(horizontal: 8, vertical: 8),
                      border: OutlineInputBorder(),
                      focusedBorder: OutlineInputBorder(borderSide: BorderSide(color: AppColors.primaryNeon)),
                    ),
                    keyboardType: TextInputType.number,
                    textInputAction: TextInputAction.next,
                    onSubmitted: _submitGain,
                    onTapOutside: (_) => _submitGain(_gainController.text),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: SliderTheme(
                    data: SliderThemeData(
                      trackHeight: 2,
                      thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
                      overlayShape: const RoundSliderOverlayShape(overlayRadius: 12),
                    ),
                    child: Slider(
                      value: widget.channel.gainDb,
                      min: -60.0,
                      max: 12.0,
                      activeColor: AppColors.primaryNeon,
                      inactiveColor: Colors.white24,
                      onChanged: (v) {
                        _update(widget.channel.copyWith(gainDb: v));
                      },
                    ),
                  ),
                ),
              ],
            ),
          ),
          SizedBox(
            width: 80,
            child: ElevatedButton(
              style: ElevatedButton.styleFrom(
                backgroundColor: Colors.white10,
                foregroundColor: Colors.white,
                padding: const EdgeInsets.symmetric(horizontal: 0, vertical: 0),
                minimumSize: const Size(60, 30),
              ),
              onPressed: () {
                // Dummy button for EQ popup
              },
              child: const Text('EQ', style: TextStyle(fontSize: 12)),
            ),
          ),
        ],
      ),
    );
  }
}

class _ToggleButton extends StatelessWidget {
  final String label;
  final bool isActive;
  final Color activeColor;
  final VoidCallback onTap;

  const _ToggleButton({
    required this.label,
    required this.isActive,
    required this.activeColor,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      child: Container(
        width: 28,
        height: 28,
        decoration: BoxDecoration(
          color: isActive ? activeColor : Colors.white10,
          borderRadius: BorderRadius.circular(4),
          border: Border.all(
            color: isActive ? activeColor : Colors.white24,
          ),
        ),
        child: Center(
          child: Text(
            label,
            style: TextStyle(
              color: isActive ? Colors.black : Colors.white54,
              fontWeight: FontWeight.bold,
              fontSize: 12,
            ),
          ),
        ),
      ),
    );
  }
}
