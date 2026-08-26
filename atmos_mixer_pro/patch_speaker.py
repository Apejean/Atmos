import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# Extract everything between "_editSpeaker" and "_editTrajectory"
pattern = r"(  Future<void> _editSpeaker\(SpeakerNode node\) async \{.*?\n  \})\n\n  Future<void> _editTrajectory"
match = re.search(pattern, content, re.DOTALL)

if not match:
    print("Could not find _editSpeaker")
    exit(1)

new_edit_speaker = """  Future<void> _editSpeaker(SpeakerNode node) async {
    double dispAngle = node.dispersionAngle;
    double dispDist = node.dispersionDistance;
    double heightZ = node.heightZ;
    double pitchTilt = node.pitchTilt;
    double rotation = node.rotation;
    int channel = node.channel;
    String selectedPreset = 'Custom';

    final TextEditingController heightCtrl = TextEditingController(text: heightZ.toStringAsFixed(2));
    final TextEditingController tiltCtrl = TextEditingController(text: pitchTilt.toStringAsFixed(2));
    final TextEditingController panCtrl = TextEditingController(text: rotation.toStringAsFixed(2));

    await showDialog(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) {
          void updateHeight(String val) {
            final p = double.tryParse(val);
            if (p != null) {
              setDialogState(() {
                heightZ = p.clamp(0.0, 50.0);
                heightCtrl.text = heightZ.toStringAsFixed(2);
                selectedPreset = 'Custom';
              });
            } else {
              heightCtrl.text = heightZ.toStringAsFixed(2);
            }
          }
          void updateTilt(String val) {
            final p = double.tryParse(val);
            if (p != null) {
              setDialogState(() {
                pitchTilt = p.clamp(-90.0, 90.0);
                tiltCtrl.text = pitchTilt.toStringAsFixed(2);
                selectedPreset = 'Custom';
              });
            } else {
              tiltCtrl.text = pitchTilt.toStringAsFixed(2);
            }
          }
          void updatePan(String val) {
            final p = double.tryParse(val);
            if (p != null) {
              setDialogState(() {
                rotation = p.clamp(-180.0, 180.0);
                panCtrl.text = rotation.toStringAsFixed(2);
              });
            } else {
              panCtrl.text = rotation.toStringAsFixed(2);
            }
          }

          return AlertDialog(
            backgroundColor: AppColors.cardSurface,
            title: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  '3D 정밀 인스펙터 [Ch ${channel + 1}]',
                  style: const TextStyle(
                    color: Colors.white,
                    fontWeight: FontWeight.bold,
                  ),
                ),
                IconButton(
                  icon: const Icon(Icons.close, color: Colors.white54),
                  onPressed: () {
                    setState(() => _selectedRoomId = null);
                    Navigator.pop(context);
                  },
                ),
              ],
            ),
            content: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text(
                    '채널 할당',
                    style: TextStyle(color: Colors.white70, fontSize: 13),
                  ),
                  const SizedBox(height: 8),
                  DropdownButton<int>(
                    value: channel,
                    dropdownColor: AppColors.cardSurface,
                    isExpanded: true,
                    style: const TextStyle(color: Colors.white),
                    items: List.generate(
                      128,
                      (index) => DropdownMenuItem(
                        value: index,
                        child: Text('Ch ${index + 1}'),
                      ),
                    ),
                    onChanged: (val) {
                      if (val != null) {
                        setDialogState(() => channel = val);
                      }
                    },
                  ),
                  const SizedBox(height: 16),
                  const Divider(color: Colors.white24),
                  const SizedBox(height: 8),
                  const Text(
                    '스피커 음향 프리셋',
                    style: TextStyle(color: Colors.white70, fontSize: 13),
                  ),
                  const SizedBox(height: 8),
                  DropdownButton<String>(
                    value: selectedPreset,
                    dropdownColor: AppColors.cardSurface,
                    isExpanded: true,
                    style: const TextStyle(color: Colors.white),
                    items: const [
                      DropdownMenuItem(
                        value: 'Custom',
                        child: Text('사용자 지정 설정'),
                      ),
                      DropdownMenuItem(
                        value: 'PointSource',
                        child: Text('Point Source (90° Beam, 3.5m Height)'),
                      ),
                      DropdownMenuItem(
                        value: 'LineArray',
                        child: Text(
                          'Line Array (60° Narrow Beam, 6.0m Height)',
                        ),
                      ),
                      DropdownMenuItem(
                        value: 'Subwoofer',
                        child: Text(
                          'Subwoofer (180° Omnidirectional, 0.5m Height)',
                        ),
                      ),
                      DropdownMenuItem(
                        value: 'CeilingAtmos',
                        child: Text(
                          'Ceiling Overhead (120° Wide Beam, 4.0m Height)',
                        ),
                      ),
                    ],
                    onChanged: (val) {
                      if (val != null) {
                        setDialogState(() {
                          selectedPreset = val;
                          if (val == 'PointSource') {
                            dispAngle = 90.0;
                            dispDist = 220.0;
                            heightZ = 3.5;
                            pitchTilt = 15.0;
                          } else if (val == 'LineArray') {
                            dispAngle = 60.0;
                            dispDist = 450.0;
                            heightZ = 6.0;
                            pitchTilt = 10.0;
                          } else if (val == 'Subwoofer') {
                            dispAngle = 180.0;
                            dispDist = 150.0;
                            heightZ = 0.5;
                            pitchTilt = 0.0;
                          } else if (val == 'CeilingAtmos') {
                            dispAngle = 120.0;
                            dispDist = 180.0;
                            heightZ = 4.0;
                            pitchTilt = 30.0;
                          }
                          heightCtrl.text = heightZ.toStringAsFixed(2);
                          tiltCtrl.text = pitchTilt.toStringAsFixed(2);
                        });
                      }
                    },
                  ),
                  const SizedBox(height: 16),
                  const Divider(color: Colors.white24),
                  
                  // Height
                  Row(
                    children: [
                      const SizedBox(width: 60, child: Text('Height (m)', style: TextStyle(color: Colors.white, fontSize: 12))),
                      Expanded(
                        child: Slider(
                          value: heightZ,
                          min: 0.0,
                          max: 20.0,
                          activeColor: AppColors.primaryNeon,
                          onChanged: (val) {
                            setDialogState(() {
                              heightZ = val;
                              heightCtrl.text = val.toStringAsFixed(2);
                              selectedPreset = 'Custom';
                            });
                          },
                        ),
                      ),
                      SizedBox(
                        width: 60,
                        child: TextField(
                          controller: heightCtrl,
                          style: const TextStyle(color: Colors.white, fontSize: 12),
                          decoration: const InputDecoration(isDense: true, contentPadding: EdgeInsets.all(8)),
                          keyboardType: TextInputType.number,
                          onSubmitted: updateHeight,
                          onTapOutside: (_) => updateHeight(heightCtrl.text),
                        ),
                      ),
                    ],
                  ),
                  
                  // Tilt
                  Row(
                    children: [
                      const SizedBox(width: 60, child: Text('Tilt (deg)', style: TextStyle(color: Colors.white, fontSize: 12))),
                      Expanded(
                        child: Slider(
                          value: pitchTilt,
                          min: -90.0,
                          max: 90.0,
                          activeColor: AppColors.primaryNeon,
                          onChanged: (val) {
                            setDialogState(() {
                              pitchTilt = val;
                              tiltCtrl.text = val.toStringAsFixed(2);
                              selectedPreset = 'Custom';
                            });
                          },
                        ),
                      ),
                      SizedBox(
                        width: 60,
                        child: TextField(
                          controller: tiltCtrl,
                          style: const TextStyle(color: Colors.white, fontSize: 12),
                          decoration: const InputDecoration(isDense: true, contentPadding: EdgeInsets.all(8)),
                          keyboardType: TextInputType.number,
                          onSubmitted: updateTilt,
                          onTapOutside: (_) => updateTilt(tiltCtrl.text),
                        ),
                      ),
                    ],
                  ),
                  
                  // Pan
                  Row(
                    children: [
                      const SizedBox(width: 60, child: Text('Pan (deg)', style: TextStyle(color: Colors.white, fontSize: 12))),
                      Expanded(
                        child: Slider(
                          value: rotation,
                          min: -180.0,
                          max: 180.0,
                          activeColor: AppColors.primaryNeon,
                          onChanged: (val) {
                            setDialogState(() {
                              rotation = val;
                              panCtrl.text = val.toStringAsFixed(2);
                            });
                          },
                        ),
                      ),
                      SizedBox(
                        width: 60,
                        child: TextField(
                          controller: panCtrl,
                          style: const TextStyle(color: Colors.white, fontSize: 12),
                          decoration: const InputDecoration(isDense: true, contentPadding: EdgeInsets.all(8)),
                          keyboardType: TextInputType.number,
                          onSubmitted: updatePan,
                          onTapOutside: (_) => updatePan(panCtrl.text),
                        ),
                      ),
                    ],
                  ),
                  
                ],
              ),
            ),
            actions: [
              TextButton(
                onPressed: () {
                  ref
                      .read(speakerLayoutProvider.notifier)
                      .removeSpeaker(node.id);
                  Navigator.pop(context);
                },
                child: const Text(
                  '스피커 삭제',
                  style: TextStyle(color: Colors.redAccent),
                ),
              ),
              TextButton(
                onPressed: () {
                  setState(() => _selectedRoomId = null);
                  Navigator.pop(context);
                },
                child: const Text(
                  '취소',
                  style: TextStyle(color: Colors.white70),
                ),
              ),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.primaryNeon,
                  foregroundColor: Colors.black,
                ),
                onPressed: () {
                  ref
                      .read(speakerLayoutProvider.notifier)
                      .updateSpeaker(
                        node.copyWith(
                          dispersionAngle: dispAngle,
                          dispersionDistance: dispDist,
                          heightZ: heightZ,
                          pitchTilt: pitchTilt,
                          rotation: rotation,
                          channel: channel,
                        ),
                        immediate: true,
                      );
                  _syncSpatialConfigRealtime();
                  setState(() => _selectedRoomId = null);
                  Navigator.pop(context);
                },
                child: const Text(
                  '설정 저장',
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
              ),
            ],
          );
        },
      ),
    );
  }
"""

content = content.replace(match.group(1), new_edit_speaker)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
