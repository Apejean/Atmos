import os

files = [
    "rust/src/api/simple.rs",
    "rust/src/api/scene.rs",
    "rust/src/api/osc.rs",
    "rust/src/audio/engine.rs",
    "rust/src/audio/streaming.rs",
    "rust/src/audio/offline.rs",
    "rust/src/osc/listener.rs",
    "rust/src/core/state.rs",
    "rust/src/test_race.rs",
    "rust/src/bin/mixer_bench.rs",
    "rust/src/bin/probe.rs",
    "rust/src/bin/probe_cpal.rs",
]

for fpath in files:
    if not os.path.exists(fpath): continue
    with open(fpath, "r") as f:
        content = f.read()
    
    # We will replace `println!(...)` with `crate::core::state::GLOBAL_STATE.log(format!(...));` 
    # except in tests or bin directories where stdout is fine.
    if "test_race" in fpath or "bin/" in fpath:
        continue
    
    # Simple regex to catch `println!("some string")` -> `crate::core::state::GLOBAL_STATE.log(format!("some string"))`
    import re
    
    # A bit complex because of macros, but let's try a simple approach for `println!` and `eprintln!`
    def replacer(match):
        macro_name = match.group(1) # println or eprintln
        args = match.group(2)
        if "GLOBAL_STATE" in args:
            return "" # skip if it's already using it
        # Sometimes args is just `"string"`, sometimes it has format args.
        # Just wrap in format!()
        return f"crate::core::state::GLOBAL_STATE.log(format!({args}));"
    
    content = re.sub(r'(e?println)!\(([\s\S]*?)\);', replacer, content)
    
    with open(fpath, "w") as f:
        f.write(content)

