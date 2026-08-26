import re

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'r') as f:
    content = f.read()

target = """                  // 3D VIRTUAL REVERB"""

expert_fields = """                  // POWER & COVERAGE
                  _buildSectionTitle('POWER & COVERAGE'),
                  const SizedBox(height: 16),
                  _buildRow('Coverage Dist', '${widget.selectedSpeaker!.dispersionDistance.toStringAsFixed(1)} m'),
                  Slider(
                    value: widget.selectedSpeaker!.dispersionDistance,
                    min: 1.0, max: 100.0,
                    onChanged: (v) => _updateSpeaker(widget.selectedSpeaker!.copyWith(dispersionDistance: v)),
                  ),
                  const SizedBox(height: 16),
                  const Divider(color: AppColors.lightGrey),
                  const SizedBox(height: 16),
                  
                  // EXPERT SPEC MODE
                  _buildSectionTitle('EXPERT SPEC MODE'),
                  const SizedBox(height: 16),
                  _buildRow('Max SPL', '${widget.selectedSpeaker!.maxSPL.toStringAsFixed(1)} dB'),
                  Slider(
                    value: widget.selectedSpeaker!.maxSPL,
                    min: 90.0, max: 150.0,
                    onChanged: (v) => _updateSpeaker(widget.selectedSpeaker!.copyWith(maxSPL: v)),
                  ),
                  _buildRow('Dispersion H', '${widget.selectedSpeaker!.dispersionAngle.toStringAsFixed(0)}°'),
                  Slider(
                    value: widget.selectedSpeaker!.dispersionAngle,
                    min: 10.0, max: 360.0,
                    onChanged: (v) => _updateSpeaker(widget.selectedSpeaker!.copyWith(dispersionAngle: v)),
                  ),
                  _buildRow('Dispersion V', '${widget.selectedSpeaker!.dispersionAngleV.toStringAsFixed(0)}°'),
                  Slider(
                    value: widget.selectedSpeaker!.dispersionAngleV,
                    min: 10.0, max: 360.0,
                    onChanged: (v) => _updateSpeaker(widget.selectedSpeaker!.copyWith(dispersionAngleV: v)),
                  ),
                  _buildRow('Low Cut', '${widget.selectedSpeaker!.lowCutHz.toStringAsFixed(0)} Hz'),
                  Slider(
                    value: widget.selectedSpeaker!.lowCutHz,
                    min: 20.0, max: 200.0,
                    onChanged: (v) => _updateSpeaker(widget.selectedSpeaker!.copyWith(lowCutHz: v)),
                  ),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      const Text('Boundary', style: TextStyle(color: Colors.white70, fontSize: 13)),
                      DropdownButton<String>(
                        value: widget.selectedSpeaker!.boundaryType,
                        dropdownColor: AppColors.cardSurface,
                        style: const TextStyle(color: AppColors.primaryBlue, fontWeight: FontWeight.bold),
                        underline: const SizedBox(),
                        items: ['Free', 'Wall', 'Corner'].map((e) => DropdownMenuItem(value: e, child: Text(e))).toList(),
                        onChanged: (val) {
                          if (val != null) _updateSpeaker(widget.selectedSpeaker!.copyWith(boundaryType: val));
                        },
                      ),
                    ],
                  ),
                  _buildRow('DSP Latency', '${widget.selectedSpeaker!.dspLatencyMs.toStringAsFixed(1)} ms'),
                  Slider(
                    value: widget.selectedSpeaker!.dspLatencyMs,
                    min: 0.0, max: 10.0,
                    onChanged: (v) => _updateSpeaker(widget.selectedSpeaker!.copyWith(dspLatencyMs: v)),
                  ),
                  const SizedBox(height: 16),
                  const Divider(color: AppColors.lightGrey),
                  const SizedBox(height: 16),
                  
                  // 3D VIRTUAL REVERB"""

content = content.replace(target, expert_fields)

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'w') as f:
    f.write(content)
