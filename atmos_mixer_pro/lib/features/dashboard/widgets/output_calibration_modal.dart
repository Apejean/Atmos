import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:atmos_mixer_pro/core/theme/colors.dart';
import 'package:atmos_mixer_pro/features/dashboard/state/output_routing_state.dart';
import 'package:atmos_mixer_pro/src/rust/common/config.dart' as rust_config;

class OutputCalibrationModal extends ConsumerWidget {
  const OutputCalibrationModal({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final channels = ref.watch(outputRoutingProvider);

    return Dialog(
      backgroundColor: Colors.transparent,
      child: Container(
        width: 1100,
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
                  const Text('Output Calibration Mixer', style: TextStyle(color: Colors.white, fontSize: 18, fontWeight: FontWeight.bold)),
                  Row(
                    children: [
                      ElevatedButton.icon(
                        style: ElevatedButton.styleFrom(
                          backgroundColor: Colors.white10,
                          foregroundColor: Colors.white,
                        ),
                        icon: const Icon(Icons.download, size: 16),
                        label: const Text('Import Calibration Data'),
                        onPressed: () {
                          // Dummy action for CSV import
                          ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(content: Text('Import functionality coming soon.')),
                          );
                        },
                      ),
                      const SizedBox(width: 16),
                      IconButton(
                        icon: const Icon(Icons.close, color: Colors.white54),
                        onPressed: () => Navigator.of(context).pop(),
                      ),
                    ],
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
                  SizedBox(width: 120, child: Text('Delay (ms)', style: TextStyle(color: Colors.white70, fontWeight: FontWeight.bold))),
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
      _update(widget.channel.copyWith(delayMs: parsed.clamp(0.0, 100.0)));
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
            width: 120,
            child: Padding(
              padding: const EdgeInsets.only(right: 16.0),
              child: Row(
                children: [
                  Expanded(
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
                ],
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
                showDialog(
                  context: context,
                  builder: (ctx) => _EqPopup(
                    channel: widget.channel,
                    onUpdate: (bands) {
                      _update(widget.channel.copyWith(eqBands: bands));
                    },
                  ),
                );
              },
              child: const Text('EQ', style: TextStyle(fontSize: 12)),
            ),
          ),
        ],
      ),
    );
  }
}

class _EqPopup extends StatefulWidget {
  final OutputChannelModel channel;
  final Function(List<rust_config.EqBand>) onUpdate;

  const _EqPopup({required this.channel, required this.onUpdate});

  @override
  State<_EqPopup> createState() => _EqPopupState();
}

class _EqPopupState extends State<_EqPopup> {
  late List<rust_config.EqBand> _bands;

  @override
  void initState() {
    super.initState();
    _bands = List.from(widget.channel.eqBands.map((b) => rust_config.EqBand(
      enabled: b.enabled,
      freq: b.freq,
      gain: b.gain,
      qFactor: b.qFactor,
      filterType: b.filterType,
    )));
  }

  void _notifyUpdate() {
    widget.onUpdate(_bands);
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      backgroundColor: Colors.transparent,
      child: Container(
        width: 800,
        height: 600,
        decoration: BoxDecoration(
          color: const Color(0xFF1E1E1E),
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: Colors.white12),
        ),
        child: Column(
          children: [
            Container(
              padding: const EdgeInsets.all(16),
              decoration: const BoxDecoration(
                border: Border(bottom: BorderSide(color: Colors.white12)),
              ),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text('8-Band Parametric EQ - ${widget.channel.name}', style: const TextStyle(color: Colors.white, fontSize: 16)),
                  IconButton(
                    icon: const Icon(Icons.close, color: Colors.white54),
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                ],
              ),
            ),
            // Header for EQ
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              color: Colors.white.withValues(alpha: 0.05),
              child: const Row(
                children: [
                  SizedBox(width: 40, child: Text('Band', style: TextStyle(color: Colors.white70, fontSize: 12))),
                  SizedBox(width: 60, child: Text('On', style: TextStyle(color: Colors.white70, fontSize: 12))),
                  SizedBox(width: 120, child: Text('Type', style: TextStyle(color: Colors.white70, fontSize: 12))),
                  SizedBox(width: 100, child: Text('Freq (Hz)', style: TextStyle(color: Colors.white70, fontSize: 12))),
                  SizedBox(width: 100, child: Text('Gain (dB)', style: TextStyle(color: Colors.white70, fontSize: 12))),
                  SizedBox(width: 100, child: Text('Q Factor', style: TextStyle(color: Colors.white70, fontSize: 12))),
                ],
              ),
            ),
            Expanded(
              child: ListView.builder(
                itemCount: _bands.length,
                itemBuilder: (context, i) {
                  final band = _bands[i];
                  return Container(
                    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                    decoration: const BoxDecoration(
                      border: Border(bottom: BorderSide(color: Colors.white10)),
                    ),
                    child: Row(
                      children: [
                        SizedBox(
                          width: 40,
                          child: Text('${i + 1}', style: const TextStyle(color: Colors.white)),
                        ),
                        SizedBox(
                          width: 60,
                          child: Switch(
                            value: band.enabled,
                            onChanged: (v) {
                              setState(() {
                                _bands[i] = rust_config.EqBand(
                                  enabled: v,
                                  freq: band.freq,
                                  gain: band.gain,
                                  qFactor: band.qFactor,
                                  filterType: band.filterType,
                                );
                              });
                              _notifyUpdate();
                            },
                          ),
                        ),
                        SizedBox(
                          width: 120,
                          child: DropdownButton<rust_config.EqType>(
                            value: band.filterType,
                            dropdownColor: const Color(0xFF2C2C2C),
                            style: const TextStyle(color: Colors.white, fontSize: 12),
                            isDense: true,
                            items: rust_config.EqType.values.map((t) {
                              return DropdownMenuItem(value: t, child: Text(t.name));
                            }).toList(),
                            onChanged: (v) {
                              if (v != null) {
                                setState(() {
                                  _bands[i] = rust_config.EqBand(
                                    enabled: band.enabled,
                                    freq: band.freq,
                                    gain: band.gain,
                                    qFactor: band.qFactor,
                                    filterType: v,
                                  );
                                });
                                _notifyUpdate();
                              }
                            },
                          ),
                        ),
                        SizedBox(
                          width: 100,
                          child: Padding(
                            padding: const EdgeInsets.only(right: 16),
                            child: TextFormField(
                              initialValue: band.freq.toStringAsFixed(1),
                              style: const TextStyle(color: Colors.white, fontSize: 12),
                              decoration: const InputDecoration(isDense: true, border: OutlineInputBorder()),
                              keyboardType: TextInputType.number,
                              onFieldSubmitted: (v) {
                                final p = double.tryParse(v);
                                if (p != null) {
                                  setState(() {
                                    _bands[i] = rust_config.EqBand(
                                      enabled: band.enabled,
                                      freq: p.clamp(20.0, 20000.0),
                                      gain: band.gain,
                                      qFactor: band.qFactor,
                                      filterType: band.filterType,
                                    );
                                  });
                                  _notifyUpdate();
                                }
                              },
                            ),
                          ),
                        ),
                        SizedBox(
                          width: 100,
                          child: Padding(
                            padding: const EdgeInsets.only(right: 16),
                            child: TextFormField(
                              initialValue: band.gain.toStringAsFixed(1),
                              style: const TextStyle(color: Colors.white, fontSize: 12),
                              decoration: const InputDecoration(isDense: true, border: OutlineInputBorder()),
                              keyboardType: TextInputType.number,
                              onFieldSubmitted: (v) {
                                final p = double.tryParse(v);
                                if (p != null) {
                                  setState(() {
                                    _bands[i] = rust_config.EqBand(
                                      enabled: band.enabled,
                                      freq: band.freq,
                                      gain: p.clamp(-24.0, 24.0),
                                      qFactor: band.qFactor,
                                      filterType: band.filterType,
                                    );
                                  });
                                  _notifyUpdate();
                                }
                              },
                            ),
                          ),
                        ),
                        SizedBox(
                          width: 100,
                          child: Padding(
                            padding: const EdgeInsets.only(right: 16),
                            child: TextFormField(
                              initialValue: band.qFactor.toStringAsFixed(2),
                              style: const TextStyle(color: Colors.white, fontSize: 12),
                              decoration: const InputDecoration(isDense: true, border: OutlineInputBorder()),
                              keyboardType: TextInputType.number,
                              onFieldSubmitted: (v) {
                                final p = double.tryParse(v);
                                if (p != null) {
                                  setState(() {
                                    _bands[i] = rust_config.EqBand(
                                      enabled: band.enabled,
                                      freq: band.freq,
                                      gain: band.gain,
                                      qFactor: p.clamp(0.1, 18.0),
                                      filterType: band.filterType,
                                    );
                                  });
                                  _notifyUpdate();
                                }
                              },
                            ),
                          ),
                        ),
                      ],
                    ),
                  );
                },
              ),
            ),
          ],
        ),
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
