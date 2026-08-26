import os
import re

files = [
    "rust/src/api/simple.rs",
    "rust/src/api/scene.rs",
    "rust/src/api/osc.rs",
    "rust/src/audio/engine.rs",
    "rust/src/audio/streaming.rs",
    "rust/src/audio/offline.rs",
    "rust/src/osc/listener.rs",
    "rust/src/core/state.rs",
]

for fpath in files:
    if not os.path.exists(fpath): continue
    with open(fpath, "r") as f:
        content = f.read()

    # Just replace all `println!` with `flutter_rust_bridge::frb_log::info!`? No, we don't have that macro.
    # What if we just comment out all the println! statements that are not critical?
    # Or just replace `println!` with `crate::core::state::GLOBAL_STATE.log(format!`?
    # Wait, `GLOBAL_STATE.log(format!(...))` is not a macro, it's a function call. It returns `()`.
    
    # Let's replace `println!(` with `log_print!(` and define `log_print!` in `core/state.rs`
    
    content = content.replace("println!(", "crate::log_print!(")
    content = content.replace("eprintln!(", "crate::log_print!(")
    
    with open(fpath, "w") as f:
        f.write(content)

