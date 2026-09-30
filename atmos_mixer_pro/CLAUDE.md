# CLAUDE.md - Atmos Mixer Pro

## 📌 Project Overview
- **Name:** Atmos Mixer Pro
- **Description:** Professional 3D Spatial Audio Mixer & DAW for commercial B2B immersive exhibitions (d'strict, Silo Lab) with zero tolerance for audio dropouts.
- **Tech Stack:**
  - **Frontend:** Flutter (macOS desktop target), Riverpod state management, Custom Painter & 3D Vector Math.
  - **Backend / DSP:** Rust 2021 (`rust_lib_atmos_mixer_pro`), CPAL (CoreAudio/ASIO), ebur128, sofa-reader, rtrb (lock-free ring buffers).
  - **FFI Bridge:** `flutter_rust_bridge` (=2.12.0).

---

## ⚡ Global Pro Audio Engineering: 3 Immutable Laws (`DSP_RULES.md`)
All audio engine and DSP code MUST strictly follow these 3 laws without exception:

1. **Law 1: NO Heap Allocation in Audio Thread (100% Forbidden)**
   - NEVER use `vec![]`, `Vec::new()`, `push()`, `.clone()`, `Box::new()`, `String`, or any heap-allocating code inside `process()`, `process_interleaved()`, or any real-time DSP callback loops.
   - **Solution:** Pre-allocate all buffers in `new()`. Use mutable slices (`&mut [f32]`), static arrays, or ring buffers during the hot loop.

2. **Law 2: NO Blocking in Audio Thread (100% Forbidden)**
   - NEVER use `Mutex::lock()`, `RwLock::read()`, `RwLock::write()`, `println!()`, file I/O, or thread sleeping in the audio thread.
   - **Solution:** Use lock-free data structures (`rtrb` ring buffers, `crossbeam-channel`, atomic primitives like `AtomicU32`, `AtomicBool`).

3. **Law 3: Sample-Accurate Interpolation (Lerp/Smoothing)**
   - NEVER snap parameters (gain, delay time, EQ freq, 3D coordinates) instantaneously. Snapping causes audible zipper noise/clicks.
   - **Solution:** Always apply Linear Interpolation (Lerp) or Low-pass Smoothing over the buffer length to smoothly transition sample-by-sample.

---

## 🔄 Mandatory 7-Step Auto-Coding & QA Verification Loop
Never consider a feature "complete" just because it compiles. Follow this mandatory workflow:

1. **Step 1 (Brainstorm & Plan):** `system-architect` analyzes requirements, checks edge cases, and drafts atomic tasks.
2. **Step 2 (Surgical Implementation):** `front-engineer` and `back-engineer` implement code adhering to DSP rules, Riverpod patterns, and Karpathy surgical principles.
3. **Step 3 (Code Audit):** `code-reviewer` audits code against 3 DSP laws and checks diff impacts.
4. **Step 4 (Rigorous Testing):** `test-runner` executes `cargo test`, `flutter analyze`, and 1kHz sine / digital silence injection.
5. **Step 5 (Supervision & Fixes):** `code-reviewer` reviews test outputs and orders immediate root-cause fixes if any flaws exist.
6. **Step 6 (Loop Repeat):** Repeat steps 2–5 until 100% flawless (0 errors, 0 warnings).
7. **Step 7 (Final Report & Memory Persist):** Record decisions in `agentmemory` MCP and deliver final proof-of-execution report.

---

## 🛠️ Build, Test, and Quality Assurance Commands

### Rust DSP Backend
```bash
# Move to rust directory
cd rust

# Syntax & Lint Check
cargo check
cargo clippy

# Run All Audio Unit Tests (Limiter/Crossover/EBU R128/SOFA HRTF 등 tests/ 하위 전체 포함)
cargo test -- --nocapture

# Run a single isolated DSP test file (e.g. bass-management crossover regression)
cargo test --test test_crossover_bass_management -- --nocapture

# Standalone probe/bench binaries (src/bin/, 자동 테스트가 아닌 수동 조사용)
cargo run --bin mixer_bench
cargo run --bin probe_cpal
```

### Flutter Frontend & FFI
```bash
# Analyze Flutter Code (Must be 0 issues)
flutter analyze

# Run Flutter Unit/Widget Tests
flutter test

# Run macOS Desktop App
flutter run -d macos

# Re-generate FFI bindings (Only when rust/src/api/ changes)
flutter_rust_bridge_codegen generate
```

---

## 🎯 Coding Conventions & Architecture Standards
- **Rust Comments & Docs:** 100% Korean comments and docstrings. Comprehensive error handling with `anyhow` / `Result` (NO `unwrap()` in production audio code).
- **Flutter / Riverpod:** Decoupled business logic in providers. High-frequency 60fps/120Hz canvas updates must use `Listenable` / `RepaintBoundary` to avoid full-screen rebuilds.
- **Data Persistence:** `config.json` serialization/deserialization between Rust `AppConfig` and Dart models must maintain 1:1 parity.

---

## 👥 Dedicated Subagents (`.claude/agents/`)
Claude Code delegates work to these 5 specialized subagents, each mapped with dedicated skills:

1. **`system-architect` (@Architect):**
   - **Assigned Skills:** `superpowers-workflow` (`/brainstorm`, `/write-plan`), `understand-anything`
   - **Role:** High-level architectural blueprints in `docs/`, data model definitions (`config.json` parity), $1.2\text{m}$ mannequin acoustics, and task breakdown.
2. **`front-engineer` (@Front):**
   - **Assigned Skills:** `karpathy-guidelines` (Surgical Changes, Simplicity First)
   - **Role:** Flutter UI, 60fps/120Hz canvas optimization (`Listenable`/`RepaintBoundary`), 3D mannequin room viewer, FFI streams.
3. **`back-engineer` (@Back):**
   - **Assigned Skills:** `atmos-pro-audio-dsp`, `atmos-spatial-audio`, `karpathy-guidelines`
   - **Role:** Rust real-time DSP, zero-allocation audio loops, lock-free concurrency (`rtrb`), DBAP ($\sum g_i^2 = 1$), LR4 crossover ($0^\circ$).
4. **`code-reviewer` (@check):**
   - **Assigned Skills:** `understand-anything` (`/understand-diff`), `atmos-zero-defect-workflow`
   - **Role:** Strict static audit against 3 DSP laws (`grep`), Diff Impact Analysis, supervising tests, zero-tolerance flaw rejection.
5. **`test-runner` (@sub):**
   - **Assigned Skills:** `superpowers-workflow` (TDD Execution), `atmos-zero-defect-workflow`
   - **Role:** Automated execution of `cargo test`, `flutter analyze`, and 1kHz sine/silence mock signal injection (<0.001% error).

---

## 🧠 Advanced Engineering Hygiene: Karpathy Guidelines & Superpowers
All agents working in Claude Code must adhere to these unified engineering principles:

### 1. Karpathy Guidelines (카파시 4대 코딩 원칙)
1. **Think Before Coding:** Never guess ambiguous requirements. State assumptions or ask clarification before modifying files.
2. **Simplicity First:** Implement the minimal code strictly necessary. Forbid speculative abstractions or unrequested generic wrappers.
3. **Surgical Changes:** Modify ONLY the lines/functions required for the task. Never reformat, reorder, or alter adjacent unrelated code.
4. **Goal-Driven Execution:** Frame every task around machine-verifiable goals (write failing test -> implement fix -> verify pass).

### 2. Knowledge & Memory Layer
- **Understand-Anything:** Use `/understand` and `/understand-diff` to analyze cross-module dependencies across Rust and Flutter before committing changes.
- **AgentMemory (MCP):** Configured via `.mcp.json` (`@agentmemory/agentmemory`). Persist architectural decisions and session handoffs so context carries across Claude Code restarts.

---

## 📚 Key Technical Skills & References
For full technical equations, architectural blueprints, and model paths, refer to:
- **Core Skills & Tech Stack:** `docs/01_Architecture/CORE_SKILLS_AND_TECH_STACK.md` (DBAP, LR4, SOFA, EBU R128, Catmull-Rom Spline, Real-time audio engine).
- **Master Agent Workflow & Spec:** `docs/03_Protocols_and_Workflows/AGENT_WORKFLOW_AND_SPECIFICATION_MASTER.md`.
- **3D Mannequin & Coordinates:** `docs/report/3d_listener_implementation_guide.md` & `dynamic_3d_room_guide.md` (Model: `assets/models/listener_head.glb`, Center: `[W/2, D/2, 1.2m]`).
- **Zero-Defect Protocol:** `docs/03_Protocols_and_Workflows/ZERO_DEFECT_PROTOCOL.md` (Sine/Silence/Impulse injection tests).
- **Claude Code Setup Guide:** `docs/03_Protocols_and_Workflows/CLAUDE_CODE_SETUP_GUIDE.md`.
