import re

def main():
    path = "lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart"
    with open(path, "r") as f:
        content = f.read()

    old_footer = """          // Footer
          Padding(
            padding: const EdgeInsets.all(16),
            child: OutlinedButton(
              onPressed: () {
                ref.read(speakerLayoutProvider.notifier).removeSpeaker(speaker!.id);
                widget.onClose();
              },
              style: OutlinedButton.styleFrom(
                side: const BorderSide(color: Colors.redAccent),
                minimumSize: const Size(double.infinity, 48),
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
              ),
              child: const Text('Remove Speaker', style: TextStyle(color: Colors.redAccent, fontWeight: FontWeight.bold)),
            ),
          ),"""

    new_footer = """          // Footer
          Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Expanded(
                  child: ElevatedButton.icon(
                    onPressed: () {
                      _updateSpeaker(speaker!, isFixed: !speaker.isFixed);
                    },
                    icon: Icon(
                      speaker.isFixed ? Icons.lock_rounded : Icons.lock_open_rounded,
                      size: 18,
                      color: speaker.isFixed ? const Color(0xFFF59E0B) : Colors.white70,
                    ),
                    label: Text(
                      speaker.isFixed ? 'FIX ON' : 'FIX OFF',
                      style: TextStyle(
                        fontWeight: FontWeight.bold,
                        color: speaker.isFixed ? const Color(0xFFF59E0B) : Colors.white70,
                      ),
                    ),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: const Color(0xFF0F172A),
                      padding: const EdgeInsets.symmetric(vertical: 14),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(8),
                        side: BorderSide(
                          color: speaker.isFixed ? const Color(0xFFF59E0B) : const Color(0xFF334155),
                          width: 1.5,
                        ),
                      ),
                      elevation: 0,
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: OutlinedButton.icon(
                    onPressed: () {
                      ref.read(speakerLayoutProvider.notifier).removeSpeaker(speaker!.id);
                      widget.onClose();
                    },
                    icon: const Icon(Icons.delete_outline_rounded, size: 18, color: Colors.redAccent),
                    label: const Text('Remove', style: TextStyle(color: Colors.redAccent, fontWeight: FontWeight.bold)),
                    style: OutlinedButton.styleFrom(
                      side: const BorderSide(color: Colors.redAccent, width: 1.5),
                      padding: const EdgeInsets.symmetric(vertical: 14),
                      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                    ),
                  ),
                ),
              ],
            ),
          ),"""

    content = content.replace(old_footer, new_footer)

    old_update = """  void _updateSpeaker(SpeakerNode speaker, {double? x, double? y, double? z, double? pan, double? tilt, double? rot, double? disp, double? rev}) {
    ref.read(speakerLayoutProvider.notifier).updateSpeaker(speaker.copyWith(
      x: x ?? speaker.x,
      y: y ?? speaker.y,
      heightZ: z ?? speaker.heightZ,
      pitchTilt: tilt ?? speaker.pitchTilt,
      rotation: rot ?? speaker.rotation,
      panDeg: pan ?? speaker.panDeg,
      dispersionAngle: disp ?? speaker.dispersionAngle,
      reverbSend: rev ?? speaker.reverbSend,
    ));
  }"""

    new_update = """  void _updateSpeaker(SpeakerNode speaker, {double? x, double? y, double? z, double? pan, double? tilt, double? rot, double? disp, double? rev, bool? isFixed}) {
    ref.read(speakerLayoutProvider.notifier).updateSpeaker(speaker.copyWith(
      x: x ?? speaker.x,
      y: y ?? speaker.y,
      heightZ: z ?? speaker.heightZ,
      pitchTilt: tilt ?? speaker.pitchTilt,
      rotation: rot ?? speaker.rotation,
      panDeg: pan ?? speaker.panDeg,
      dispersionAngle: disp ?? speaker.dispersionAngle,
      reverbSend: rev ?? speaker.reverbSend,
      isFixed: isFixed ?? speaker.isFixed,
    ));
  }"""

    content = content.replace(old_update, new_update)

    with open(path, "w") as f:
        f.write(content)

main()
