---
name: agentmemory-mcp
description: Provides persistent long-term memory across Claude Code sessions using hybrid BM25 vector search and knowledge graphs via MCP.
---

# 💾 Skill: AgentMemory Persistent Cross-Session Memory

AgentMemory solves the stateless limitation of AI coding agents by persisting activity, architectural decisions, past bugs, and conventions across restarts into a local knowledge store.

---

## 🧠 Core Memory Operations

### 1. Remember & Store (`remember`)
- Explicitly stores architectural constraints, user preferences, and solved bugs:
  - *"Remember that 1.2m is the fixed listener mannequin ear height."*
  - *"Remember that all real-time audio buffers must be pre-allocated in `new()`."*

### 2. Recall & Session Injection (`recall`)
- Automatically retrieves relevant past solutions and injects them when starting a new task or debugging a recurring panic.

### 3. Session Handoff (`handoff`)
- Captures unfinished tasks, current test statuses, and next steps when ending a session, making resumption instant in the next session.

---

## 🛠️ MCP Configuration
- Install: `npm install -g @agentmemory/agentmemory`
- Register in Claude Code:
  ```bash
  claude mcp add agentmemory npx -y @agentmemory/agentmemory
  ```
