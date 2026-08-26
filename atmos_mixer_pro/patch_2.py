import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

target_header = """                    // LOGO
                    Row(
                      children: [
                        Icon(Icons.graphic_eq, color: neonCyan, size: 28),
                        const SizedBox(width: 8),
                        Column(
                          mainAxisAlignment: MainAxisAlignment.center,
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: const [
                            Text('3D AUDIO', style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 16, height: 1.0)),
                            Text('SIMULATOR', style: TextStyle(color: Colors.white70, fontSize: 14, height: 1.0)),
                          ],
                        ),
                      ],
                    ),"""
                    
replacement_header = """                    // BACK & LOGO
                    Row(
                      children: [
                        IconButton(
                          icon: const Icon(Icons.arrow_back, color: Colors.white),
                          onPressed: () => Navigator.of(context).pop(),
                        ),
                        const SizedBox(width: 8),
                        Icon(Icons.graphic_eq, color: neonCyan, size: 28),
                        const SizedBox(width: 8),
                        const Text(
                          'Atmos Mixer Pro',
                          style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 18),
                        ),
                      ],
                    ),"""

content = content.replace(target_header, replacement_header)

target_person = """                    // USER ICON
                    const CircleAvatar(
                      backgroundColor: Colors.white10,
                      child: Icon(Icons.person, color: Colors.white70),
                    ),"""
                    
replacement_person = """"""
content = content.replace(target_person, replacement_person)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)
