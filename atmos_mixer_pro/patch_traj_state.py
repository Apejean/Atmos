import re

with open('lib/features/exhibition/state/trajectory_state.dart', 'r') as f:
    content = f.read()

old_json = """              'current_position': {
                'x': trajectories.first.getCurrentPositionMeter().dx,
                'y': trajectories.first.getCurrentPositionMeter().dy,
                'z': trajectories.first.getCurrentHeightZ(),
              },
              'audio_file_path': trajectories.first.audioFilePath,
            }"""

new_json = """              'current_position': {
                'x': trajectories.first.getCurrentPositionMeter().dx,
                'y': trajectories.first.getCurrentPositionMeter().dy,
                'z': trajectories.first.getCurrentHeightZ(),
              },
              'size': trajectories.first.size,
              'audio_file_path': trajectories.first.audioFilePath,
            }"""

if old_json in content:
    content = content.replace(old_json, new_json)
else:
    print("Failed to find trajectory state block")

with open('lib/features/exhibition/state/trajectory_state.dart', 'w') as f:
    f.write(content)
