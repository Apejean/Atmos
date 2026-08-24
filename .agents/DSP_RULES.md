# Global Pro Audio Engineering - 3 Immutable Laws

All agents (@Front, @Back, @sub, @check, etc.) working on the Atmos Mixer Pro project MUST strictly adhere to the following rules when developing or reviewing audio-related code. This project targets top-tier B2B environments (e.g., d'strict, Silo Lab) with zero tolerance for audio dropouts.

## Rule 1: NO Heap Allocation in Audio Thread (100% Forbidden)
- **Do NOT** use `vec![]`, `Vec::new()`, `push()`, `.clone()`, `Box::new()`, `String`, or any standard library functions that call `malloc` or allocate heap memory inside the audio `process()`, `process_interleaved()`, or any DSP callback loops.
- **Solution:** Pre-allocate all memory (buffers, arrays) inside the component's `new()` constructor (which runs outside the audio thread). Use mutable slices (`&mut [f32]`), static arrays, or ring buffers during the hot loop.

## Rule 2: NO Blocking in Audio Thread (100% Forbidden)
- **Do NOT** use `Mutex::lock()`, `RwLock::read()`, `RwLock::write()`, `println!()`, file I/O, or thread sleeping/blocking inside the audio thread.
- **Solution:** Use lock-free data structures (e.g., `rtrb` ring buffers, `crossbeam-channel`, atomic variables like `AtomicU32` or `AtomicBool`) to pass parameters into the audio thread asynchronously.

## Rule 3: Sample-Accurate Interpolation (Lerp) for Parameter Changes
- **Do NOT** instantly snap parameters (like volume, gain, delay time, or EQ frequencies) inside the audio loop when receiving 60fps UI/OSC updates. This causes immediate audible "clicks" or "zipper noise".
- **Solution:** Always apply Linear Interpolation (Lerp) or Low-pass Smoothing over the buffer length to smoothly transition from the old parameter value to the new target value sample-by-sample.

## Quality Assurance & Testing Protocol (Zero-Defect Protocol)
- Do not rely on "Vibe Coding" or manual visual inspection.
- The AI must always write and pass automated `cargo test` scripts (e.g., `test_audio_precision.rs`) using 1kHz sine waves and silence injection.
- Tests must computationally verify that processing errors are below `0.001%` before pushing code.

By enforcing this Spec-Driven AI Engineering, we guarantee a zero-defect, world-class 3D audio engine.
