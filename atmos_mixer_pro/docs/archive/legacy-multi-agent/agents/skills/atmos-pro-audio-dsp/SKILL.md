---
name: atmos-pro-audio-dsp
description: Enforces the 3 Immutable Laws of Pro Audio Engineering (Zero-Allocation, Lock-Free Concurrency, Sample-Accurate Smoothing) and real-time DSP safety audits in Atmos Mixer Pro.
---

# 🎧 Skill: Pro Audio Real-Time DSP Engineering

This skill provides strict protocols and actionable patterns for developing and auditing real-time audio code in Atmos Mixer Pro. Target environments include top-tier B2B commercial installations (d'strict, Silo Lab) with zero tolerance for audio dropouts.

---

## ⚡ The 3 Immutable Laws (Zero-Dropout Standard)

### 1. Law 1: Zero Heap Allocation in Audio Thread (100% Forbidden)
- **Forbidden Functions in hot DSP loops (`process()`, `process_interleaved()`):**
  - `Vec::new()`, `vec![]`, `Vec::with_capacity()`
  - `.push()`, `.clone()`, `Box::new()`
  - `String::from()`, `format!()`, string concatenations
  - Any method calling `malloc` / `realloc` / OS memory allocators
- **Standard Solution:**
  - Pre-allocate all buffers, arrays, and delay lines in the constructor `new()`.
  - Use mutable slices (`&mut [f32]`), static arrays, or ring buffers during audio callbacks.

### 2. Law 2: Lock-Free Concurrency in Audio Thread (100% Forbidden)
- **Forbidden Operations:**
  - `Mutex::lock()`, `RwLock::read()`, `RwLock::write()`
  - `println!()`, `eprintln!()` (stdout/stderr is blocking I/O)
  - Disk / File I/O (`File::open`, `write!()`)
  - Thread sleeping / yielding (`thread::sleep`)
- **Standard Solution:**
  - Use lock-free SPSC ring buffers (`rtrb`).
  - Use atomic primitives (`AtomicU32`, `AtomicBool`, `AtomicPtr`) with `Ordering::Relaxed` or `Ordering::Acquire`/`Release`.
  - Use lock-free message channels (`crossbeam-channel`) for command ingestion outside the audio thread.

### 3. Law 3: Sample-Accurate Parameter Smoothing (Anti-Zipper Noise)
- **Forbidden:**
  - Instant parameter snapping when 60fps UI/OSC updates arrive (causes audible clicks/zipper noise).
- **Standard Solution:**
  - Apply linear interpolation (Lerp) or single-pole low-pass filtering over the buffer size:
    ```rust
    // Sample-accurate Linear Interpolation Example
    let step = (target_gain - current_gain) / buffer_len as f32;
    for sample in buffer.iter_mut() {
        current_gain += step;
        *sample *= current_gain;
    }
    ```

---

## 🔍 Static Audit Script for Real-Time Violations

When reviewing audio code changes, run this terminal sweep:

```bash
# Check for heap allocations in DSP
grep -rn "Vec::new" rust/src/audio/
grep -rn "vec!\[" rust/src/audio/
grep -rn "Box::new" rust/src/audio/

# Check for blocking primitives
grep -rn "Mutex::lock" rust/src/audio/
grep -rn "println\!" rust/src/audio/
```

---

## 🧪 Verification Protocol
- All DSP components must pass automated `cargo test` runs injecting:
  1. **1kHz Sine Wave:** 0dBFS peak, non-target bins below -140dBFS.
  2. **Digital Silence (0.0):** Output stabilized at -140dBFS, zero CPU denormal spikes.
  3. **Impulse (1.0, 0.0...):** Mathematical filter impulse response match within 0.001% tolerance.
