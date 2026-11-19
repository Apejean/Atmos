# 비상 전환 제거(안정성 ⑤) 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. — 이 저장소는 서브에이전트를 쓰지 않는다(Windows Main이 혼자 순서대로, superpowers:executing-plans).

**Goal:** 설정 장치를 열지 못해도 기본 장치(PC 출력, −40dB)로 바꿔 틀지 않고, 무음으로 엔진 유지자가 설정 장치만 계속 찾게 한다.

**Architecture:** 비상 전환은 두 곳(`init_audio_system_for_user`의 첫 시작, `keeper_open`의 열기 실패)에서 `init_audio_system(None)`을 부르고, `is_failover_mode` 표시로 믹서 감쇠·상태 문자열·OSC 장치 이름·복귀 규칙(`core/device_return.rs`)을 움직인다. 두 호출을 지우면 실패가 그대로 유지자(`core/engine_keeper.rs`)로 가 대기·재시도가 되므로, 표시와 그것에 매달린 코드를 모두 지운다. 유지자·재생 거절·이어 가기는 바꾸지 않는다.

**Tech Stack:** Rust 2021(cpal 0.16), Flutter(Dart, Riverpod), flutter_rust_bridge 2.12.0(이 작업은 `rust/src/api/` 공개 함수 시그니처를 바꾸지 않는다 — codegen 불필요).

**Spec:** `docs/superpowers/specs/2026-11-14-engine-keeper-design.md` 15절(사용자 승인 2026-11-19 "응 그렇게 진행해").

## Global Constraints

- 오디오 스레드 3법칙(할당·블로킹 금지, 보간). 믹서 변경은 곱셈·램프를 빼는 것뿐이어야 한다.
- 운영 오디오 코드에 `unwrap()` 금지. Rust 주석·문서는 한국어.
- 외과적 변경: 비상 전환에 매달린 줄만 지운다. WASAPI·CoreAudio 장치 목록 감시(`device_list_changed`), 설정에 장치가 없을 때 기본 장치로 여는 길, `device_search::reject_before_search`는 그대로.
- `flutter analyze` 0건, `cargo clippy` 경고가 지금(31건)보다 늘지 않음.
- 브랜치 `win/failover-removal`은 #112 병합 뒤 origin/main을 합친 상태에서 시작한다. 커밋·푸시·PR은 PR 머지 작업 세션에 요청한다(Draft PR 먼저: 설계 15절 + 이 계획).
- 다른 Atmos 앱(Sub 시험·설치본)이 떠 있으면 전체 `cargo test`에서 `test_cpal_hosts`를 뺀다(`-- --skip test_cpal_hosts`).

## Review Focus

1. 설정에 장치가 없는 config(사용자가 기본 장치를 고름) → 기본 장치로 정상 재생. 지우지 않은 길이 함께 지워지면 안 된다 — Task 4 실기 ④.
2. 재생 중 USB 뽑기, **Windows 출력이 허용된 상태** → 기본 장치로 가지 않고 무음 대기 — Task 4 실기 ②. 지금까지의 실기는 출력을 막아 둔 상태였다.
3. 짧은 자기 재시작(워치독 1초 공백, 장치가 그대로 있음) → 같은 장치로 다시 열림. `device_for_self_restart`를 지운 뒤에도 `req.device`로 연다 — Task 1 기존 `test_engine_keeper` 자기 재시작 시험 + Task 4 실기 ③.
4. 환경설정에서 꽂혀 있지 않은 장치를 고름(첫 시작이 아님) → 지금처럼 대기 배너, 기본 장치로 가지 않음 — Task 4 실기 ⑤.
5. macOS에서 인터페이스 없이 시작 → 맥북 스피커 무음, 대기 배너 — PR 본문 Mac 세션 확인 요청(이 PC에서는 못 봄).

---

### Task 1: 엔진에서 비상 전환 지우기(Rust)

**Files:**
- Modify: `rust/src/api/simple.rs` — `api_create_stream_status_stream`(`Failover` 첫 값), `init_audio_system_for_user`(`remember_requested_device`, `take_first_start`, 비상 전환 match 팔), `keeper_open`(자기 재시작의 `device_for_self_restart`·장치 목록 읽기, 실패 뒤 기본 장치 열기 팔), 엔진 스레드 시작 성공 뒤 `requested_explicit_device`·비상 해제·`Failover` 방송
- Modify: `rust/src/core/state.rs` — `is_failover_mode` 필드와 초기값 삭제
- Modify: `rust/src/audio/mixer.rs` — `failover_gain` 필드(140행), 초기값(344행), `failover_start`·`failover_target`(1084~1089행), 곱셈(1114행), 전진(1133행) 삭제. 주석의 "페일오버" 언급도 정리
- Modify: `rust/src/osc/reply.rs` — `engine_state_and_device`의 `failover` 조건과 문서 주석
- Modify: `rust/src/core/event_log.rs` 8행 — 상태 목록에서 `Failover`
- Modify: `rust/src/core/mod.rs` — `pub mod device_return;` 삭제
- Delete: `rust/src/core/device_return.rs`, `rust/tests/test_failover_return_device.rs`
- Modify: `rust/tests/test_master_switch_ramps.rs` — `페일오버_감쇠는_딸깍_없이_걸리고_풀린다` 삭제

**Interfaces:**
- Consumes: `core::engine_keeper::{user_started, StartFailure, OpenRequest, RETRY_SEARCH}`, `classify_start_error` — 바꾸지 않는다.
- Produces: `keeper_open(req: &OpenRequest) -> Result<(), StartFailure>`는 시그니처 그대로, 동작은 "`req.device`만 연다(자기 재시작이면 찾기 30초·짧은 부팅 램프·`HotReloading` 방송은 그대로)". `init_audio_system_for_user(device_name: Option<String>) -> Result<(), AtmosError>`는 `stop_audio_engine` → `restart_resume::cancel_all` → `start_audio_system(device_name)` 결과를 그대로 돌려준다. 상태 문자열 `Failover`는 더 나오지 않는다(Task 2가 기댄다).

- [ ] **Step 1: 지우기 전 기준 기록**

Run(`rust/`): `cargo test --test test_engine_keeper --test test_master_switch_ramps --test test_show_resume_waiting -- --nocapture`
Expected: 모두 통과(지운 뒤와 비교할 기준). 통과 수를 적어 둔다.

- [ ] **Step 2: 위 Files의 삭제·수정**

`is_failover_mode` 필드를 먼저 지워 컴파일 오류로 남은 사용처를 모두 찾는다. `keeper_open`의 `first` 결과는 `Ok`/`Err(failure)` 그대로 돌려준다(`Err(StartFailure::Hung)` 따로 다룰 필요 없음). `keeper_open`·`init_audio_system_for_user`의 문서 주석에서 비상 전환 설명을 지우고 "장치를 못 열면 유지자가 대기·재시도한다(⑤, 설계 15절)"로 바꾼다.

- [ ] **Step 3: 빌드와 남은 언급 확인**

Run(`rust/`): `cargo build` → 오류 0.
Run(저장소 루트): `git grep -n "is_failover_mode\|failover_gain\|device_return\|init_audio_system(None)" -- atmos_mixer_pro/rust` → 결과 없음.

- [ ] **Step 4: 시험**

Run(`rust/`): `cargo test -- --nocapture`(다른 Atmos 앱이 떠 있으면 `--skip test_cpal_hosts`)
Expected: 실패 0. 통과 수 = Step 1 기준 포함 전체 − 삭제한 시험(`test_failover_return_device` 6개 + 램프 1개). `test_engine_keeper`의 대기 → 복귀·자기 재시작·열기 3연속 실패가 그대로 통과.
Run: `cargo clippy` → 경고 31건 이하.

- [ ] **Step 5: 커밋(PR 세션에 요청)**

`refactor(engine): stop falling back to the default device when the configured device cannot open (stability step 5)`

### Task 2: 화면에서 `Failover` 지우기(Dart)

**Files:**
- Modify: `lib/features/dashboard/screens/dashboard_screen.dart` — `status == 'Failover'` 스낵바 블록 삭제
- Modify: `lib/core/state/global_state.dart:614` — `engineRecoveredStatus`는 `status == 'Running'`만
- Modify: `lib/features/dashboard/widgets/engine_down_banner.dart` 5·43·85행, `lib/features/dashboard/widgets/load_warning_banner.dart` 50·76행 — `Failover` 조건·값 계약 주석에서 삭제
- Test: `test/global_error_engine_clear_test.dart`, `test/engine_down_banner_test.dart`(97~99행), `test/load_warning_banner_test.dart`(48~50행)

**Interfaces:**
- Consumes: Task 1 — 상태 문자열에 `Failover`가 없다.
- Produces: 없음.

- [ ] **Step 1: 시험을 먼저 바꾼다**

`global_error_engine_clear_test.dart`: `expect(engineRecoveredStatus('Failover'), isTrue)` 줄을 지우고 주석의 "(Running·Failover)"를 "(Running)"으로. `engine_down_banner_test.dart`: 시험 이름 `'clears on Running'`, 반복 목록 `['Running']`. `load_warning_banner_test.dart`: 이름 `'clears on LoadNormal and Running'`, 목록 `['LoadNormal', 'Running']`.

- [ ] **Step 2: 코드 수정(위 Files)**

- [ ] **Step 3: 확인**

Run: `flutter analyze` → No issues found.
Run: `flutter test` → All tests passed(지금 284개 기준, 수가 줄지 않음).
Run(저장소 루트): `git grep -n "Failover" -- atmos_mixer_pro/lib atmos_mixer_pro/test` → 결과 없음.

- [ ] **Step 4: 커밋(PR 세션에 요청)**

`refactor(dashboard): drop the default-device fallback notice and Failover status (stability step 5)`

### Task 3: 문서

**Files:**
- Modify: `docs/HANDOFF.md` — 확정된 결정 "대체 출력은 쓰지 않는다 — 대체 예정, 구현 전, 지금 동작은 그대로"를 구현됨(PR 번호, 2026-11-19)으로. 2026-10-22 "앱을 켤 때 … 기본 장치로 비상 전환"과 2026-10-07 "−40dB 안전 패드" 줄에 "⑤로 대체됨" 표시. ⚠️ 15번(비상 전환과 복귀) 끝에 대체 표시. 남은 일 16 단계별 현황 ⑤.
- Modify: `docs/superpowers/specs/2026-11-14-engine-keeper-design.md` — 15절 머리에 "구현됨(브랜치, 커밋)", 1.3 첫 항목·D3·D7의 "⑤까지 유지"에 "⑤에서 지움(15절)".
- Modify: `docs/03_Protocols_and_Workflows/OSC_INTERFACE_SPEC.md` 89·91행 — `Failover` 상태 문자열과 "비상 전환 중은 Running … 장치 이름 빈 문자열" 설명을 지운다.
- Modify: `docs/WINDOWS_FIELD_CHECKLIST.md` 45행(켤 때 인터페이스 없음 → 무음 대기 배너, 기본 장치로 가지 않음), 75행(비상 전환과 복귀 → USB 뽑기 무음 대기·다시 꽂으면 건너뛴 자리에서 이어 감), 196행 중단 기준의 "비상 전환" 삭제.
- Modify: `docs/superpowers/specs/2026-10-05-crash-relaunch-supervisor-design.md` 7절 "ASIO 장치 뽑기 → 오디오 멈춤 재실행이 3번 뒤 멈춤" → "ASIO 장치 뽑기 → 장치 대기, 다시 띄우지 않음(⑥ S2, 2026-11-19 확인)", 그리고 점검표 56행의 같은 문장.

- [ ] **Step 1: 위 수정**
- [ ] **Step 2: 확인** — Run(저장소 루트): `git grep -n "Failover\|비상 전환" -- atmos_mixer_pro/docs/HANDOFF.md atmos_mixer_pro/docs/03_Protocols_and_Workflows atmos_mixer_pro/docs/WINDOWS_FIELD_CHECKLIST.md` → 남은 것은 "대체됨"·역사 기록 줄뿐.
- [ ] **Step 3: 커밋(PR 세션에 요청)** — `docs: record the default-device fallback removal (stability step 5)`

### Task 4: Release 빌드·앱 확인·실기(Windows, 사용자와)

**Files:** 없음(증거는 `D:\dev\logs\<날짜>\`).

- [ ] **Step 1: 빌드** — `flutter build windows --release`(백그라운드, run.ps1). 성공.
- [ ] **Step 2: 실기 준비** — 다른 Atmos 앱 없음 확인, `backup-appdata.ps1`, 사용자가 Windows 출력(Realtek·SAMSUNG)을 **허용**으로 둔 것 확인, 소리 나는 단계 전 장치·음량 확인.
- [ ] **Step 3: 실기** — 별도 로그 폴더(`ATMOS_LOG_DIR`), 감시 프로그램 함께.
  - ① UFX+ 끈 채 `--logon` 시작 → PC 스피커 무음, 주황 대기 배너, 하트비트 `WaitingForDevice`, 앱 로그에 "비상 전환" 없음. UFX+ 켬 → 저장 위치부터 이어 감(귀로).
  - ② 재생 중 USB 뽑기 → 무음 대기(기본 장치 열림 없음, 로그 확인). 30초 넘게 둔다.
  - ③ 다시 꽂기 → 끊긴 시간만큼 건너뛴 자리에서 이어 감(로그의 위치 계산 + 귀로).
  - ④ 설정에 장치가 없는 config(백업 사본의 `device_name`을 비운 시험용, 시험 뒤 되돌림) → 기본 장치로 정상 재생.
  - ⑤ 환경설정에서 꺼진 장치를 고름 → 대기 배너, 기본 장치로 가지 않음.
- [ ] **Step 4: 정리** — 앱 닫기(감시 먼저), 앱 데이터 되돌리기는 사용자 확인 뒤, 결과를 PR 세션에(PROGRESS·RESULTS·PR 본문 검증 칸). 병합은 사용자 지시 뒤.
