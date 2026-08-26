import re
path = "rust/src/core/state.rs"
with open(path, "r") as f:
    content = f.read()

content = content.replace('crate::log_print!("{}", msg);', '// removed print')
with open(path, "w") as f:
    f.write(content)
