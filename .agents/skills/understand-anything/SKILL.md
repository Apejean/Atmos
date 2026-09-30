---
name: understand-anything
description: Generates interactive knowledge graphs, domain mappings, and diff impact analysis for deep comprehension of the Atmos Mixer Pro codebase in Claude Code.
---

# 🗺️ Skill: Understand-Anything Codebase Knowledge Graph

Understand-Anything maps repository files, structs, traits, and dependencies into an interactive knowledge graph, enabling semantic exploration, guided architecture tours, and diff impact prediction.

---

## 🔍 Core Capabilities

### 1. Codebase Knowledge Graph (`/understand`)
- Automatically indexes Rust modules (`audio/`, `osc/`, `api/`) and Flutter Riverpod providers.
- Visualizes dependencies and data flow across the FFI bridge.

### 2. Guided Architectural Tours (`/understand-chat`)
- Query the architecture using semantic questions:
  - *"How does audio routing flow from the track strip to the LR4 crossover?"*
  - *"Where are the 3D mannequin ear coordinates injected into the DBAP calculation?"*

### 3. Diff Impact Analysis (`/understand-diff`)
- Predicts ripple effects before committing modifications.
- Verifies whether changing a Rust struct breaks Dart FFI `#[repr(C)]` layouts or Riverpod listeners.

---

## 🛠️ Claude Code Plugin Installation
- Install: `/plugin marketplace add Lum1104/Understand-Anything`
- Dashboard: `/understand-dashboard`
