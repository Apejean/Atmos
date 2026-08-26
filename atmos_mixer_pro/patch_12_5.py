import re

with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'r') as f:
    content = f.read()

target_channel = """                  _buildRow('CHANNEL', 'CH ${widget.selectedSpeaker!.channel}'),"""

replacement_channel = """                  // Channel Dropdown
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      const Text('CHANNEL', style: TextStyle(color: Colors.white70, fontSize: 13)),
                      DropdownButton<int>(
                        value: widget.selectedSpeaker!.channel,
                        dropdownColor: AppColors.cardSurface,
                        style: const TextStyle(color: AppColors.primaryBlue, fontWeight: FontWeight.bold),
                        underline: const SizedBox(),
                        items: List.generate(64, (index) {
                          return DropdownMenuItem(
                            value: index,
                            child: Text('HW OUT ${index + 1}'),
                          );
                        }),
                        onChanged: (val) {
                          if (val != null) {
                            _updateSpeaker(widget.selectedSpeaker!.copyWith(channel: val));
                          }
                        },
                      ),
                    ],
                  ),"""

if target_channel in content:
    content = content.replace(target_channel, replacement_channel)

# Replace build to use SingleChildScrollView
target_build = """  Widget build(BuildContext context) {
    if (widget.selectedSpeaker == null) return const SizedBox.shrink();

    return Positioned(
      right: 0,
      top: 0,
      bottom: 0,
      child: Container(
        width: 300,
        decoration: const BoxDecoration(
          color: AppColors.panelBackground,
          border: Border(left: BorderSide(color: AppColors.lightGrey, width: 1)),
        ),
        child: Column(
          children: [
            // Header
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
              color: AppColors.cardSurfaceSolid,
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Row(
                    children: [
                      const Icon(Icons.settings_input_component, color: AppColors.primaryBlue, size: 20),
                      const SizedBox(width: 8),
                      Text(
                        'SPEAKER S${widget.selectedSpeaker!.id.padLeft(2, '0')}',
                        style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 14),
                      ),
                    ],
                  ),
                  IconButton(
                    icon: const Icon(Icons.close, color: Colors.white54, size: 20),
                    onPressed: widget.onClose,
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(),
                  ),
                ],
              ),
            ),
            
            // Body
            Expanded(
              child: Padding(
                padding: const EdgeInsets.all(16.0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: ["""
                  
replacement_build = """  Widget build(BuildContext context) {
    if (widget.selectedSpeaker == null) return const SizedBox.shrink();

    return Positioned(
      right: 0,
      top: 0,
      bottom: 0,
      child: Container(
        width: 300,
        decoration: const BoxDecoration(
          color: AppColors.panelBackground,
          border: Border(left: BorderSide(color: AppColors.lightGrey, width: 1)),
        ),
        child: Column(
          children: [
            // Header
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
              color: AppColors.cardSurfaceSolid,
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Row(
                    children: [
                      const Icon(Icons.settings_input_component, color: AppColors.primaryBlue, size: 20),
                      const SizedBox(width: 8),
                      Text(
                        'SPEAKER S${widget.selectedSpeaker!.id.padLeft(2, '0')}',
                        style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 14),
                      ),
                    ],
                  ),
                  IconButton(
                    icon: const Icon(Icons.close, color: Colors.white54, size: 20),
                    onPressed: widget.onClose,
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(),
                  ),
                ],
              ),
            ),
            
            // Body
            Expanded(
              child: SingleChildScrollView(
                padding: const EdgeInsets.all(16.0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: ["""

content = content.replace(target_build, replacement_build)

target_reverb = """                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }"""

replacement_reverb = """                  ),
                  const SizedBox(height: 24),
                  const Divider(color: AppColors.lightGrey),
                  const SizedBox(height: 16),
                  
                  // 3D VIRTUAL REVERB
                  _buildSectionTitle('3D VIRTUAL REVERB'),
                  const SizedBox(height: 16),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      const Text('Reverb Toggle', style: TextStyle(color: Colors.white70)),
                      Switch(value: true, onChanged: (v) {}),
                    ],
                  ),
                  const SizedBox(height: 8),
                  const Text('RT60 (Decay)', style: TextStyle(color: Colors.white54, fontSize: 12)),
                  Slider(
                    value: 1.5,
                    min: 0.1,
                    max: 5.0,
                    onChanged: (v) {
                      // We can just call rust API directly for now with fixed mix
                      rust_api.apiSetReverbParams(mix: 0.5, decay: v);
                    },
                  ),
                  const Text('Wet Mix', style: TextStyle(color: Colors.white54, fontSize: 12)),
                  Slider(
                    value: 0.5,
                    min: 0.0,
                    max: 1.0,
                    onChanged: (v) {
                      rust_api.apiSetReverbParams(mix: v, decay: 1.5);
                    },
                  ),
                ],
              ),
              ),
            ),
          ],
        ),
      ),
    );
  }"""
  
content = content.replace(target_reverb, replacement_reverb)
with open('lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart', 'w') as f:
    f.write(content)
