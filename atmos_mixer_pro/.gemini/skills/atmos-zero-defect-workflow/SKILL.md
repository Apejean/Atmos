---
name: atmos-zero-defect-workflow
description: Enforces the mandatory 7-Step Auto-Coding QA Loop and 9-point nano-verification checklist before declaring any coding task complete in Atmos Mixer Pro.
---

# 🔄 Skill: Zero-Defect Auto-Coding Workflow & QA Verification

This skill governs the quality gatekeeper process for all code modifications in Atmos Mixer Pro. Agents are strictly forbidden from acting unilaterally or declaring completion without passing every stage.

---

## 🔁 The Mandatory 7-Step Coding Loop

1. **Step 1: Implementation (`@Front`, `@Back`)**
   - Implement features following architectural specs and 3 Pro Audio Laws.
2. **Step 2: Review Request (`@Front`/`@Back` ➡️ `@check`)**
   - Formally submit all modified files for review.
3. **Step 3: Code Audit & Coordination (`@check`)**
   - Audit code against 3 DSP laws, memory leaks, and thread safety. Instruct `@sub` to run tests.
4. **Step 4: Rigorous Testing (`@sub` ➡️ `@check`)**
   - Execute:
     - `cd rust && cargo test -- --nocapture`
     - `flutter analyze`
     - Isolated DSP injection tests.
5. **Step 5: Supervision & Order Fixes (`@check` ➡️ `@Front`/`@Back`)**
   - If any warning, failure, or flaw exists, order immediate root-cause fixes.
6. **Step 6: Fix & Repeat Loop**
   - Repeat Steps 2 through 5 until 100% flawlessness (0 errors, 0 warnings).
7. **Step 7: Final Verification & Report (`@check` ➡️ `@Main`)**
   - Only report completion after 100% flawlessness is proven with execution logs.

---

## 🛑 9-Point Nano Verification Checklist (Pass mark: 100/100)

1. **PRD Compliance:** Zero-allocation in audio loop, lock-free, zero disk I/O in DSP.
2. **User Flow Parity:** Reset, theme start, and OSC triggers operate without deadlock.
3. **Screen Layout:** No "RIGHT OVERFLOWED BY XXX PIXELS" on window resize. Riverpod state decoupled from 60fps canvas.
4. **Data Parity:** `config.json` 1:1 parity between Rust `AppConfig` and Dart models.
5. **Design System:** No hardcoded random colors; strict adherence to `AppColors` and glow animations.
6. **Task Parity:** No leftover `unimplemented!()` or `TODO` shortcuts.
7. **Conventions:** 100% Korean docstrings/comments in Rust; robust `Result` error handling (no `unwrap()`).
8. **Build & Lint:** 0 warnings in `cargo check`, `cargo clippy`, and `flutter analyze`.
9. **Risk Mitigation:** No CPU spikes, race conditions, or unhandled edge cases.
