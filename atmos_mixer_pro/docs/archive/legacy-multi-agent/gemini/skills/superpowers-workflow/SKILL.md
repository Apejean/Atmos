---
name: superpowers-workflow
description: Implements the Superpowers structured engineering framework (Brainstorming, Plan-Driven Development, Test-Driven Development, and Subagent-Driven Execution) in Claude Code.
---

# ⚡ Skill: Superpowers Engineering Framework

Superpowers transforms Claude Code from a raw code generator into a disciplined Senior Software Engineer through structured development phases and subagent delegation.

---

## 🔄 Core Development Phases

### 1. Phase 1: Brainstorming & Requirement Clarification (`/brainstorm`)
- Explore architectural design options before committing to an approach.
- Evaluate trade-offs (CPU load, lock-free safety, memory footprints).
- Produce a clear architectural direction approved by the user.

### 2. Phase 2: Plan-Driven Specification (`/write-plan`)
- Draft an atomic, step-by-step implementation plan with verifiable checkpoints.
- Specify exact file paths, modified functions, and test scenarios.
- Do not proceed to code generation without an approved plan.

### 3. Phase 3: Subagent-Driven Execution
- Delegate atomic tasks to specialized, context-isolated subagents:
  - `front-engineer` for Flutter UI
  - `back-engineer` for Rust DSP
- Each subagent operates in a clean context window, preventing context pollution.

### 4. Phase 4: Systematic TDD & Verification (`/execute-plan`)
- Write failing tests first.
- Implement minimal code to pass tests.
- Re-run test sweeps to ensure zero regressions.

---

## 🛠️ Claude Code Plugin Commands
- Install: `/plugin install superpowers@claude-plugins-official`
- Commands: `/brainstorm`, `/write-plan`, `/execute-plan`, `/sup`
