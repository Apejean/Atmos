# CLAUDE.md - Atmos Mixer Pro

## 프로젝트
- 몰입형 전시(d'strict, Silo Lab)용 3D 공간 음향 믹서·DAW. 오디오 끊김은 허용되지 않는다.
- 설계·개발은 macOS, **현장 운영은 Windows**(ASIO/WASAPI). macOS 결과를 현장 보증으로 보지 않는다.
- 스택: Flutter(macOS·Windows 데스크톱, Riverpod) + Rust 2021(`rust_lib_atmos_mixer_pro`, cpal 0.16, rtrb) + `flutter_rust_bridge` =2.12.0.
- 앱 코드는 `atmos_mixer_pro/`: `lib/`(Dart), `rust/`(엔진), `supervisor/`(충돌 후 재실행 감시), `integration_test/`, `docs/`.

## 오디오 스레드 3법칙 (예외 없음)
1. **할당 금지**: `process()` 등 실시간 콜백 안에서 `Vec::new()`·`push()`·`clone()`·`Box::new()`·`String` 금지. 버퍼는 `new()`에서 미리 잡는다.
2. **블로킹 금지**: `Mutex`·`RwLock`·`println!`·파일 I/O·sleep 금지. rtrb·crossbeam·원자형을 쓴다.
3. **보간**: 게인·지연·EQ·좌표를 한 번에 바꾸지 않는다. 버퍼 길이에 걸쳐 샘플 단위로 보간한다.

Rust·Flutter 세부 규칙은 해당 파일을 다룰 때 `.claude/rules/`에서 자동으로 실린다.

## 명령 (`atmos_mixer_pro/` 기준)
```bash
cd rust && cargo test -- --nocapture        # 전체 Rust 테스트 (디스크 여유 수 GB 필요)
cd rust && cargo test --test <이름> -- --nocapture
cd rust && cargo clippy
flutter analyze                             # 0건 유지
flutter test
flutter test integration_test/app_flow_test.dart -d macos   # 수동, 실제 소리 남, 다른 Atmos 앱 먼저 끄기
flutter_rust_bridge_codegen generate        # rust/src/api/ 바꿨을 때만
```

## 작업 방식: Main / Sub 두 세션
사용자 화면의 "Main"·"Sub" 세션이 역할을 나눈다. 세션 안에서 서브에이전트·워크플로는 쓰지 않는다(토큰 비용).

- **Main**: 요구 이해, 계획, 위험도 판단, 설계·디버깅, 실기 시험, Sub 결과 검수, HANDOFF 갱신 판단.
  - 국소적·저위험(구조 변경 없음, 별도 PR 불필요, 사용자와 실시간 조정)이면 직접 처리한다. 줄 수가 아니라 범위·위험으로 판단한다.
- **Sub**: Main이 넘긴 일의 브랜치·커밋·PR·CI 확인·병합, HANDOFF·점검표 기록, 범위가 정해진 구현과 검증.
- **Main → Sub 전달**: 대화 내역을 붙이지 않고 다음만 보낸다 — 목표, 범위·대상 파일, 건드리지 말 것, 관련 문서, 완료 조건, 검증(Main이 이미 돌린 결과 포함). Sub는 나머지를 이 파일·HANDOFF·코드에서 찾는다.
- **Sub 에스컬레이션**: API 계약(FRB `rust/src/api/`)·`config.json` 구조·엔진 구조 변경, 광범위한 수정, 원인 불명, 요구와 구조 충돌, 3법칙 위반 발견 시 억지로 진행하지 않고 Main에 보고한다: 현재 작업 / 발견 / 근거 / 위험 / 권하는 다음 단계.
- **검증 보고**: 실행한 명령과 결과 수치만 보고한다. 돌리지 않은 검증을 통과라고 쓰지 않는다.
- **Git**: 공유 작업 폴더(`/Users/Allweno/Projects/GitHub/atmos`)의 다른 세션 미커밋 파일은 건드리지 않고 임시 worktree에서 커밋한다. `.DS_Store`, `.claude/agents/*`, `rust/.ua/`, `rust/data/`는 커밋에서 뺀다.

## 모델 선택 (역할과 모델은 분리, 모델별 세션은 만들지 않는다)
- **Haiku**: 탐색, 단순 분류·문서 정리.
- **Sonnet**: 기본값. 일반 구현·수정, 테스트 작성, PR·병합·기록.
- **Opus**: 오디오 엔진·실시간 경로, 원인 불명 디버깅, 장치·재시작·FFI 경계, 여러 모듈에 걸친 변경.
- **Fable**: 아주 큰 장시간 자율 작업일 때만.
- 모델을 올리기 전에 조사·컨텍스트·테스트 부족이 원인인지 먼저 본다. Main은 Sub에 넘길 때 필요하면 권하는 모델을 함께 적고, 세션 모델은 사용자가 앱에서 바꾼다.

## 컨텍스트 읽는 순서
1. 이 파일 → 2. `atmos_mixer_pro/docs/HANDOFF.md`(현재 상태·확정된 결정·남은 일의 기준) → 3. 관련 코드.
그 밖의 문서는 작업과 관련 있을 때만 연다.

| 정보 | 기준 문서 |
|---|---|
| 현재 진행·남은 일·확정된 결정 | `docs/HANDOFF.md` |
| Windows 실기 점검 / Windows PC 작업 | `docs/WINDOWS_FIELD_CHECKLIST.md` / `docs/windows/WINDOWS_HARNESS.md`·`WINDOWS_QUICKSTART.md` |
| 기능 설계·계획 | `docs/superpowers/specs/`, `docs/superpowers/plans/`, `docs/02_Planning_and_Specs/` |
| DSP 이론(DBAP·LR4·SOFA·EBU R128) | `docs/01_Architecture/CORE_SKILLS_AND_TECH_STACK.md`, `CROSSOVER_LR24_SPEC.md` |
| 3D 방·청취자 좌표 | 저장소 루트 `docs/report/` (청취자 중심 `[W/2, D/2, 1.2m]`, 모델 `assets/models/listener_head.glb`) |
| 신호 주입 검증 방식 | `docs/03_Protocols_and_Workflows/ZERO_DEFECT_PROTOCOL.md` |

(`docs/`는 `atmos_mixer_pro/docs/`. 이전 도구용 문서 `.agents/`, `.gemini/`, `docs/03_Protocols_and_Workflows/`의 `AGENT_WORKFLOW_AND_SPECIFICATION_MASTER.md`·`CLAUDE_CODE_SETUP_GUIDE.md`·`loop.md`는 옛 다중 에이전트 방식이라 따르지 않는다.)

## 문서 갱신은 일이 생겼을 때만
- HANDOFF: 작업 완료·범위 변경·중요한 발견·막힘·사용자 결정이 있을 때. 사소한 수정마다 고치지 않는다.
- 스펙·설계 문서: 실제 구조가 바뀔 때만. 새 문서보다 기존 문서 갱신을 먼저 한다.
- 과거 대화는 기준이 아니다. 오래 필요한 지식은 프로젝트 파일에 남긴다.

## 코딩 규칙
- Rust 주석·문서는 한국어. 운영 오디오 코드에서 `unwrap()` 금지, `anyhow`/`Result`로 처리.
- Rust `AppConfig`와 Dart 모델의 `config.json`은 1:1로 맞춘다.
- 외과적 변경: 필요한 줄만 고치고, 요청 없는 추상화·재정렬·서식 변경을 하지 않는다. 가능하면 실패하는 테스트 → 수정 → 통과 순서.
