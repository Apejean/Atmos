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

서브에이전트 정의 `atmos_mixer_pro/.claude/agents/`(커밋된 5개)도 옛 방식이지만 그 자리에 둔다.

`claude-agents-uncommitted/`: 공유 작업 폴더에 커밋되지 않은 채 남아 있던 두 파일(2026-10-01 01:59 생성, 이 프로젝트 Claude Code 세션 기록에 쓴 흔적 없음)을 2026-10-07에 그대로 보관했다.
- `code-reviewer.md`: 커밋된 `.claude/agents/code-reviewer.md`를 크게 늘린 수정본(커밋본은 원래대로 둠).
- `check.md`: 위 수정본의 복사본에 `name`만 `check`로 바꾼 것.
