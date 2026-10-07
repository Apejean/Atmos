# 옛 다중 에이전트 방식 문서 (보관)

2026-10-07에 보관했다. 지금은 따르지 않는다. 현재 규칙은 저장소 루트 `CLAUDE.md`, 현재 상태는 `docs/HANDOFF.md`가 기준이다.

| 보관 위치 | 원래 위치 | 내용 |
|---|---|---|
| `agents/` | 저장소 루트 `.agents/` | 이전 도구(Antigravity)용 7단계 루프 규칙, 옛 대화 ID, 스킬 |
| `gemini/` | `atmos_mixer_pro/.gemini/` | 이전 도구(Gemini)용 규칙·스킬 |
| `AGENT_WORKFLOW_AND_SPECIFICATION_MASTER.md` | `docs/03_Protocols_and_Workflows/` | 서브에이전트 5개·7단계 워크플로 규격 |
| `CLAUDE_CODE_SETUP_GUIDE.md` | `docs/03_Protocols_and_Workflows/` | 옛 Claude Code 시작 안내 |
| `loop.md` | `docs/03_Protocols_and_Workflows/` | 범용 자율 감독 루프 점검표 |
| `task.md` | `docs/02_Planning_and_Specs/` | 빈 파일(`PAN_DEG_AND_EARLY_REFLECTIONS_SPEC.md`가 참조하던 진단 문서 자리) |
| `claude-agents/check.md` | `atmos_mixer_pro/.claude/agents/check.md`(미추적이었다) | 서브에이전트 정의. 2026-10-01 01:59에 만들어졌고(code-reviewer.md와 16초 간격) `code-reviewer.md` 복사본에 `name`만 `check`로 바꾼 것. 이 프로젝트의 Claude Code 세션 기록에 쓴 흔적이 없다 |
| `claude-agents/code-reviewer.md` | `atmos_mixer_pro/.claude/agents/code-reviewer.md` | 서브에이전트 정의. 2026-10-01 01:59에 만들어졌고 커밋본(afc06a4) 대비 +170/−13인 미커밋 수정본(공유 작업 폴더의 보관 시점 내용 그대로). 이 프로젝트의 Claude Code 세션 기록에 쓴 흔적이 없다 |

나머지 서브에이전트 정의 4개(`back-engineer.md`, `front-engineer.md`, `system-architect.md`, `test-runner.md`)는 `atmos_mixer_pro/.claude/agents/`에 그대로 있다. 같은 옛 방식이지만 사용자가 위 두 파일만 보관하기로 했다. 위 두 파일은 `.claude/agents/*`를 커밋에서 빼는 CLAUDE.md 규칙의 예외로, 사용자 지시(2026-10-07)에 따라 보관했다.
