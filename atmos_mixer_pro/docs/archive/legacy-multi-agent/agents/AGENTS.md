# Atmos Project Agent Rules

## Mandatory 7-Step Auto-Coding Workflow Enforcement
**CRITICAL RULE:** For ALL coding, debugging, or testing tasks in this project, the agent MUST NOT work unilaterally. The agent MUST strictly follow the `docs/build/8_AUTO_CODING_WORKFLOW.md` document and utilize the 7-Step Coding Loop.

### Workflow Summary:
1. **Implementation:** `@Front` and `@Back` (or the main agent acting as them) implement the code.
2. **Review Request:** Hand over to `@check` for code review.
3. **Coordination:** `@check` spawns/instructs `@sub` for testing.
4. **Testing:** `@sub` runs rigorous tests (e.g., `cargo test`, `flutter analyze`, manual QA simulation) and reports back to `@check`.
5. **Supervision:** `@check` reviews test results. If any flaws exist, it commands `@Front`/`@Back` to fix them.
6. **Repeat:** Loop steps 2-5 until 100% flawless.
7. **Final Report:** Only after `@check` confirms 100% flawlessness can the main agent (`@Main`) report back to the user.

**NEVER skip to completion without `@check` and `@sub` performing their QA process. NEVER act alone ("독단적으로 행동") when a bug is reported or a feature is requested.**

### Designated Agent Communication
**CRITICAL RULE:** Do NOT use `invoke_subagent` to spawn new subagents for the 7-Step Workflow roles. You MUST use the `send_message` tool to communicate with the following specific, pre-existing conversation IDs:
- **`@Front`** (Frontend Engineer): `ba0eea7c-3322-491d-90df-13915beb4c84`
- **`@Back`** (Backend Engineer): `a3ded415-d30c-416d-9b4f-3b9f8124c618`
- **`@sub`** (Tester/QA): `64964cfb-f2d6-4424-b447-1d3e1b6b62e6`
- **`@check`** (Code Reviewer & Quality Supervisor): `4420f854-6f64-433b-a32a-d398478e82dd`

When a task requires delegation or QA, use `send_message` to send instructions to the exact Conversation ID mapped to the respective role.

## Pre-Build / Pre-Upload Protocol
**CRITICAL RULE:** Before pushing code to GitHub, creating a release tag, or running a build, the `@Main` operator MUST command `@check` and `@sub` to perform a final, comprehensive deep-debugging sweep. 
- You MUST ensure there are absolutely no remaining bugs, memory leaks, or unhandled edge cases.
- GitHub upload and building are strictly blocked until `@check` officially reports a 100% flawlessness rate.

## Pro Audio DSP Rules
All agents must read and strictly follow the 3 Immutable Laws of Pro Audio Engineering outlined in  when writing or reviewing audio backend code. No heap allocations or locks in the DSP thread.


## Pro Audio DSP Rules
All agents must read and strictly follow the 3 Immutable Laws of Pro Audio Engineering outlined in `.agents/DSP_RULES.md` when writing or reviewing audio backend code. No heap allocations or locks in the DSP thread.
