import re

with open('rust/src/audio/mixer.rs', 'r') as f:
    content = f.read()

# Replace line 264
content = content.replace(
    'if inst.output_channel == usize::MAX && inst.current_position.is_some() {',
    'if inst.output_channel == usize::MAX {'
)

# Replace line 598
content = content.replace(
    'if instance.output_channel == usize::MAX && instance.current_position.is_some() {',
    'if instance.output_channel == usize::MAX {'
)

with open('rust/src/audio/mixer.rs', 'w') as f:
    f.write(content)
