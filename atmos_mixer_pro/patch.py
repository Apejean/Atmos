import re

with open('rust/src/api/simple.rs', 'r') as f:
    content = f.read()

# We will just see if we can add eprintln!
content = content.replace(
    'Err(_) => continue,',
    'Err(e) => { eprintln!("Failed to get output devices for host {}: {:?}", host.id().name(), e); continue },'
)

with open('rust/src/api/simple.rs', 'w') as f:
    f.write(content)
