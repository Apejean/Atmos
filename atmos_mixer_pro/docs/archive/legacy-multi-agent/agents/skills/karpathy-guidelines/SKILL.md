---
name: karpathy-guidelines
description: Enforces the 4 Karpathy Coding Principles (Think Before Coding, Simplicity First, Surgical Changes, Goal-Driven Verification) to prevent AI hallucinations, over-engineering, and unnecessary refactoring.
---

# 🧠 Skill: Karpathy Engineering Guidelines

Derived from Andrej Karpathy's insights on AI coding agent traps, this skill enforces rigorous engineering hygiene to ensure minimal, verified, and precise code modifications.

---

## 🏛️ The 4 Core Principles

### 1. Think Before Coding (코딩 전 깊은 사고)
- **Do NOT** silently fill in ambiguities or make unverified assumptions.
- If a requirement, audio buffer parameter, or UI flow is unclear, explicitly state assumptions or stop and clarify before writing code.
- Analyze dependencies and downstream effects before touching files.

### 2. Simplicity First (단순함 최우선 / 오버엔지니어링 금지)
- Implement the absolute minimum code required to solve the task.
- **Do NOT** build speculative abstractions, unrequested "future-proof" generic layers, or unnecessary helper wrappers.
- Less code = fewer bugs, especially in real-time audio threads.

### 3. Surgical Changes (외과수술식 최소 변경)
- Touch **ONLY** the specific lines and functions strictly required for the task.
- **Do NOT** "improve", reformat, reorder, or refactor adjacent code, comments, or styling.
- Clean up only the orphans or unused imports that your own changes created.

### 4. Goal-Driven Execution (목표 주도 검증 루프)
- Replace vague completion claims with concrete, machine-verifiable criteria:
  - Step 1: Write an automated test that reproduces the bug or asserts the new spec.
  - Step 2: Implement the code until the test passes.
  - Step 3: Run full regression checks (`cargo test`, `flutter analyze`).
