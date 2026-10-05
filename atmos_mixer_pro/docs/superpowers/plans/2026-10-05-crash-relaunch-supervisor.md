# 충돌·멈춤 후 자동 재실행 — 구현 계획 (Task 1~11은 구현 파일 기준 코드)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **상태(2026-10-05, Sub가 이어 씀):** 작업 번호는 최종 12개다. **Task 1~11의 코드 블록은 작업 트리의 실제 파일을 옮겼다**(새 파일은 전문, 기존 파일은 `git diff`, Task 4는 Task 3 시점 블록과의 차이). 생성 파일(FRB codegen 결과)은 옮기지 않았다. 코드 블록은 작성 시점의 복사본이라 이후 파일이 바뀌면 파일이 기준이다. "통과"로 적은 시험 결과는 Main이 보고한 것이고 Sub가 돌린 것이 아니다. 아래 "구현 상태" 표가 기준이다.

## 구현 상태 (2026-10-05 작업 트리 기준)

| # | 작업 | 구현 | 계획의 코드 블록 |
|---|---|---|---|
| 1 | 감시 크레이트 + 하트비트 형식 + 기록 회전 | 구현·통과(Main 보고) | 파일에서 옮김 |
| 2 | 재실행 대기·멈춤 판단 | 구현·통과(Main 보고) | 파일에서 옮김 |
| 3 | 감시 루프 기본 | 구현·통과(Main 보고) | 파일에서 옮김 |
| 4 | 넘겨받기·중복·두 번째 감시 | 구현·통과(Main 보고) | 파일·diff에서 옮김 |
| 5 | 첫 방 테마 시작 | 구현·통과(Main 보고) | 파일·diff에서 옮김 |
| 6 | 앱 로그 파일 | 구현·통과(Main 보고) | 파일·diff에서 옮김 |
| 7 | 신호·시작 관문 | 구현·통과(Main 보고) | 파일·diff에서 옮김 |
| 8 | 공연 상태 저장·이어 가기 | 구현·통과(Main 보고) | 파일·diff에서 옮김 |
| 9 | 무음 경고 | 구현·통과(Main 보고) | 파일·diff에서 옮김 |
| 10 | Dart | 구현·통과(Main 보고, analyze는 기존 5건) | 파일·diff에서 옮김 |
| 11 | installer·CI | 구현(정적 확인만, Windows 빌드는 첫 PR에서) | 파일·diff에서 옮김 |
| 12 | 끝까지 확인·인계 | 점검표·HANDOFF 완료(Sub), 소리 없는 실제 앱 확인 12개 통과(Main 보고), 소리 시나리오는 사용자 필요 | 스크립트 옮김 |

**Goal:** 앱이 충돌하거나 멈추면 별도 감시 프로그램이 앱을 다시 띄우고 공연을 멈춘 위치부터 이어 가게 한다. 운영자가 일부러 닫으면 다시 띄우지 않는다.

**Architecture:**
- 새 Rust 크레이트 `supervisor/`(바이너리 `atmos_supervisor`)가 앱을 띄우거나 넘겨받는다. 앱이 남기는 신호 파일(`app.lock`, `app.pid`, `heartbeat`, `clean_exit`)로 판단하고, 종료 코드에는 기대지 않는다.
- 앱(Rust) 쪽 추가: 시작 관문, 하트비트, 정상 종료 표시, 공연 상태 저장·이어 가기, 무음 경고, 첫 방 테마 시작.
- 앱(Dart) 쪽 추가:
  - `main(args)`에서 관문을 먼저 부르고, 공연 상태를 읽어 둔다.
  - 2초 하트비트를 보낸다.
  - 창을 닫을 때 정상 종료를 표시한다.
  - 스플래시 뒤 이어 가기를 판단한다.
  - 환경설정에 "로그인할 때 공연 자동 시작" 스위치를 둔다.

**Tech Stack:** Rust 2021(`File::try_lock`, std 1.89+; 로컬 1.98), windows-sys 0.61, libc 0.2, chrono 0.4.45, flutter_rust_bridge 2.12.0, Riverpod 3.4, shared_preferences, Inno Setup, GitHub Actions

**Spec:** `docs/superpowers/specs/2026-10-05-crash-relaunch-supervisor-design.md`

## Global Constraints

- DSP 3법칙: 오디오 스레드는 건드리지 않는다. `watchdog_last_callback`, `vu_levels`, `ENGINE_ACTIVE`, `master_mute`는 원자 값으로 읽기만 한다.
- Rust 주석은 한국어로 짧게 쓴다. 제품 코드에 `unwrap()`을 쓰지 않는다(테스트는 괜찮다).
- 공유 파일 이름(스펙 4.2):
  - 앱 로그 폴더(`core::log_file::log_dir()` = `temp_dir()/atmos_mixer_pro_logs`): `app.lock`, `app.pid`, `heartbeat`, `clean_exit`, `supervisor.lock`, `supervisor.log`.
  - 앱 지원 폴더: `show_state.json`.
- 하트비트 형식은 `키=값` 줄이다. 키는 `pid`, `ui_ms`, `audio_cb_ms`, `engine_active`(0/1), `silent_warnings` 다섯 개다.
- 인자는 `--supervised`, `--auto-relaunched`, `--logon`이다. 종료 코드: 3은 중복 실행, 0은 감시로 넘김.
- 수치(스펙 5.1):

  | 항목 | 값 |
  |---|---|
  | 하트비트 간격 | 2초 |
  | 멈춤 판단 | 30초 |
  | 첫 하트비트 기한 | 90초(넘겨받은 앱은 30초) |
  | 오디오 멈춤 | 120초 |
  | 확인 간격 | 1초 |
  | 재실행 대기 | 2초 → ×2 → 최대 60초, 5분 안정 뒤 2초로 |
  | 공연 상태 저장 | 5초 |
  | 이어 가기 유효 | 10분 |
  | 무음 경고 | 60초, 이후 10분마다 |
  | 오디오 멈춤 재실행 | 연속 3번까지 |

- 환경설정 키는 `auto_resume_on_logon`이다(기본 true, 표시 이름 "로그인할 때 공연 자동 시작"). 이 컴퓨터의 운영 설정이라 프로젝트 파일(`kExhibitionPrefsKeys`)에 넣지 않는다.
- `flutter_rust_bridge`는 `=2.12.0`이다. `rust/src/api/`를 바꾸면 `flutter_rust_bridge_codegen generate`를 돌린다.
- 테스트는 실제 로그 폴더·실제 config.json·실제 환경설정을 건드리지 않는다. 신호 파일은 항상 임시 폴더를 주입한다.
- 커밋은 사용자가 요청할 때만 한다. 각 작업 끝의 "체크포인트"는 `git status`·`git diff --stat` 확인까지다.
  - 커밋에서 뺄 것: `.DS_Store`, `.claude/agents/*`, `rust/.ua/`, `rust/data/`.
- 빌드·장치 테스트 전에 두 가지를 확인한다.
  - 다른 Atmos 앱이 없어야 한다: `pgrep -fl atmos_mixer_pro.app`.
  - 디스크 여유가 3GB 이상이어야 한다: `df -h /System/Volumes/Data`. 계획 작성 시점에는 **2.9GB**였다. 모자라면 사용자에게 확보를 부탁하고 캐시를 직접 지우지 않는다.

## Review Focus

1. **저장 스레드가 이어 가기보다 먼저 `show_state.json`을 덮어씀.** 이어 가기는 앱 시작 때 읽어 둔 지난 상태를 써야 한다. → Task 8 테스트 2단계: 저장 스레드가 파일을 빈 상태로 덮어쓴 뒤에도 지난 위치로 이어 간다.
2. **운영자가 닫는 중 앱이 멈춤.** clean_exit 뒤 하트비트가 멈춘 경우다. 강제 종료하고 다시 띄우지 않으며 감시도 끝낸다. → Task 3 `close-hang` 테스트.
3. **사람이 PC를 절전시켰다 깨움.** 확인 간격이 5초 넘게 벌어지면 타이머를 다시 잰다. 멀쩡한 앱을 죽이면 안 된다. → Task 2 gap 테스트.
4. **죽은 앱의 PID를 다른 프로세스가 물려받음.** 넘겨받거나 죽이면 안 된다. 넘겨받기는 `app.lock`이 잡혀 있을 때만 한다. → Task 4 `bystander` 테스트 + 잠금 확인을 빼는 변이 확인.
5. **공연 중 새 버전 설치(업그레이드).** installer가 감시를 먼저 끝내고 앱을 끝내야 한다. 그렇지 않으면 감시가 앱을 다시 띄워 파일이 잠긴다. → Task 11 `PrepareToInstall` 정적 확인 + 현장 점검표 항목.

## 스펙에서 더 정한 점(구현하며 정함 — 검토 요청)

- **넘겨받기 조건.** `app.lock`이 잡혀 있고 `app.pid`의 프로세스가 살아 있어야 넘겨받는다. PID 재사용으로 엉뚱한 프로세스를 넘겨받아 30초 뒤 죽이는 일을 막는다. 잠금 확인은 Windows·macOS 모두에 적용한다.
- **두 번째 감시.** 바로 끝나기 전에 앱을 인자 없이 한 번 띄운다. 그러면 앱의 시작 관문이 기존 창을 앞으로 가져오고 종료 코드 3으로 끝난다. 바로가기가 감시 exe를 가리키므로 이렇게 해야 점검표의 "바로가기 두 번 → 기존 창 앞으로"가 된다. 앱이 내려가 있는 사이라면 바로 뜨고, 돌고 있는 감시가 넘겨받는다.
- **닫는 중 멈춤.** clean_exit의 PID가 이 앱이면, 멈춤으로 강제 종료하더라도 다시 띄우지 않고 감시를 끝낸다.
- **중복 종료가 이어질 때.** 종료 코드 3 뒤에는 바로 대상을 다시 정한다. 그래도 연속 3번이면 재실행 대기를 둔다. 권한 문제로 넘겨받지 못할 때 1초마다 앱을 띄우는 일을 막는다.
- **공연 상태 읽기 시점.** `api_start_show_state(dir)`를 Dart `main`에서 관문 통과 직후 부른다. 이 함수는 지난 상태를 메모리에 읽어 두고 저장 스레드를 시작하며, 첫 저장은 한 간격 뒤에 한다. `api_resume_show()`는 인자 없이 읽어 둔 상태를 한 번만 쓴다.
- **엔진 재시작 중 저장.** 엔진 재시작 복원을 기다리는 트랙(`restart_resume::pending_entries()`)도 저장한다. 재시작 중에는 재생 목록이 잠깐 비기 때문이다.
- **로그 내보내기.** `supervisor.log`(밀린 것 포함)도 함께 복사한다(`core::log_file::export_to`).
- **감시 크레이트 의존성.** 스펙의 windows-sys·libc 외에 `chrono`(clock 기능)를 더한다. 감시 기록 시각을 앱 로그와 같은 현지 시각으로 남기기 위해서다.
- **테스트 전용 지원.**
  - 인자 `--dir`, `--tick-ms`, `--backoff-min-ms`를 더한다.
  - 테스트용 가짜 앱 바이너리 `fake_app`을 감시 크레이트에 둔다(`src/bin/fake_app.rs`). 배포에는 넣지 않는다. CI는 `atmos_supervisor.exe`만 복사한다.
  - 가짜 앱이 Rust라 `app.lock`을 실제로 쥔다. 그래서 감시 테스트를 Windows CI에서도 돌린다(넘겨받기 핸들 경로 검증).
- **OSC.** `OscAction::ThemeStart(String, Vec<String>)`를 단위 변형 `ThemeStart`로 바꾸고 `api::show::api_theme_start()`를 부른다.
- **Dart 하트비트.** 앞 호출이 끝나지 않았으면 건너뛴다. Rust가 막히면 하트비트가 끊겨야 감시가 알아챈다.
- **`ui_ms` 값.** 같은 밀리초에 불러도 반드시 커지게 한다.
- **스펙대로 두는 점(알림).** 대기 중인 앱이 충돌하면 `--auto-relaunched`로 다시 뜬다. 재생 중이던 게 없었으면 첫 방 테마로 시작한다. 점검 중이라면 환경설정 스위치만 꺼서는 이를 막지 못한다. 현장 점검표에 적는다.
- **확인한 사실.** macOS Flutter는 `dartEntrypointArguments`를 따로 정하지 않으면 프로세스 인자를 Dart `main`에 넘긴다(`FlutterDartProject.h`). 그래서 macOS 끝까지 확인에서도 인자가 전달된다.
- **기록 회전은 줄을 버리지 않는다(Sub 제안 반영).** 감시 기록(Task 1)과 앱 로그(Task 6)가 같은 규칙이다: 지금 파일이 크기를 넘으면 먼저 지금 파일을 `<이름>.rotating`으로 옮겨 본다. 옮기지 못하면(Windows에서 다른 프로그램이 기록 파일을 쥐고 있을 때) 아무것도 건드리지 않고 그 줄은 지금 파일에 쓴다. 예전 방식은 오래된 파일부터 밀다가 실패해 줄마다 오래된 기록을 하나씩 지웠다. 스펙 5.4에 반영했다.
- **첫 하트비트와 3D 웹뷰는 무관하다.** 하트비트는 `AtmosMixerProApp.initState`의 2초 `Timer.periodic`이 보낸다. 3D 웹뷰 로딩이나 엔진 기동을 기다리지 않으므로, Windows에서 3D가 실패해도 하트비트는 간다. 스펙 5.1 표를 바로잡았다.

## 사용자 결정이 필요한 것(이 계획에서 구현하지 않음)

계획에는 선택 작업이나 남은 결정으로만 둔다. HANDOFF "남은 일"에도 있다.

- **installer 방화벽 규칙.** OSC가 `0.0.0.0`으로 열려(`osc/listener.rs`) 무인 PC의 첫 실행에서 Windows 방화벽 허용 창이 뜬다. 설치 때 규칙을 넣을지, 넣는다면 설치 권한을 어떻게 할지 정한다.
- **WebView2 런타임 설치.** 3D 방 뷰어를 Windows에 넣으면 WebView2 런타임이 필요하다. Windows 11에는 기본으로 있지만 Windows 10 PC에는 없을 수 있다. 현장이 오프라인이면 installer에서 설치나 확인 단계가 필요하다.
- **3D 방 뷰어 Windows 구현 순서.** 사용자 결정: 3D 뷰어는 무조건 Windows에 들어간다. `webview_flutter`가 Windows를 지원하지 않아(Android·iOS·macOS만) 대체 구현이 필요하고, 후보는 `webview_windows` 0.4.0, `flutter_inappwebview` 6.1.5다. Windows PC에서 작은 실험으로 정한다. 이 계획의 범위 밖이다.
- **v1.1.3 태그와 main의 버전 차이.** `v1.1.3`은 `main`의 조상이 아니다(8/26에 갈라졌고 `main`에 없는 커밋이 32개). `main`의 앱 버전은 1.1.2다. 의도한 것인지, 현장에 설치된 버전이 무엇인지 확인한다.

## 파일 구조

| 파일 | 책임 |
|---|---|
| `supervisor/Cargo.toml`, `supervisor/.gitignore` | 새 크레이트(배포 바이너리 `atmos_supervisor`, 테스트용 `fake_app`) |
| `supervisor/src/lib.rs` | 모듈 목록 |
| `supervisor/src/heartbeat.rs` | 신호 파일 이름, `Heartbeat` 읽기·쓰기, `read_pid`, `atomic_write` |
| `supervisor/src/logfile.rs` | `supervisor.log` 기록·회전(10MB, 4개) |
| `supervisor/src/backoff.rs` | 재실행 대기 시간 |
| `supervisor/src/watch.rs` | 멈춤 판단(단조 시계, 간격 벌어짐 처리) |
| `supervisor/src/process.rs` | 대상 프로세스(자식/넘겨받음), 잠금 확인, 넘겨받기 |
| `supervisor/src/run.rs` | 설정·인자, 감시 루프 |
| `supervisor/src/main.rs` | 진입점(`#![windows_subsystem = "windows"]`) |
| `supervisor/src/bin/fake_app.rs` | 테스트용 가짜 앱 |
| `supervisor/tests/rules.rs`, `supervisor/tests/supervise.rs` | 규칙 테스트, 프로세스 테스트 |
| `rust/src/api/show.rs` | `api_theme_start`, `api_start_show_state`, `api_resume_show` |
| `rust/src/api/lifecycle.rs` | `StartupDecision`, `api_startup_gate`, `api_heartbeat`, `api_mark_clean_exit`, 창 앞으로(Windows) |
| `rust/src/core/app_signals.rs` | 신호 파일, 하트비트 값, 시작 관문 판단, `SILENT_WARNINGS`, `now_ms`, `atomic_write` |
| `rust/src/core/show_state.rs` | 저장·읽기·이어 가기 판단·저장 스레드·무음 판단 |
| `rust/src/osc/router.rs`, `rust/src/osc/listener.rs` | 테마 시작을 `api_theme_start`로 |
| `rust/src/core/log_file.rs` | 내보내기에 감시 기록 포함, 회전 실패해도 줄을 버리지 않음 |
| `rust/Cargo.toml` | windows 기능 `Win32_UI_WindowsAndMessaging` |
| `lib/core/state/launch_mode.dart` | `launchArgsProvider`, `shouldResumeShow`, 환경설정 읽기·쓰기, `showStateDir` |
| `lib/main.dart`, 스플래시, 환경설정 모달 | 관문·하트비트·정상 종료·이어 가기·스위치 |
| `integration_test/app_flow_test.dart` | `app.main(const [])` |
| `windows/installer.iss`, `../.github/workflows/build_release.yml` | 배포·CI |
| `tool/supervisor_e2e_macos.sh`(Main), `docs/WINDOWS_FIELD_CHECKLIST.md`·`docs/HANDOFF.md`(Sub) | 확인·인계 |

## 작업 목록(최종 번호)

1. **감시 크레이트 + 하트비트 형식 + 기록 회전**
   - 기록 회전은 지금 파일을 `<이름>.rotating`으로 먼저 옮겨 본다. 실패하면 아무것도 건드리지 않고 그 줄은 지금 파일에 쓴다.
2. **재실행 대기·멈춤 판단(순수)**
   - 대기 시간: `Backoff::new(min, max, stable)`, `next_delay(uptime) -> Duration`. uptime ≥ stable이면 min으로 돌아간다.
   - 판단 결과: `Verdict {Ok, StartupHang, UiHang, AudioStall}`, `Observation { verdict, audio_progressed, gap }`.
   - 관찰: `Watch::new(now, first_heartbeat, hang, audio_stall)`, `observe(now: Instant, hb: Option<&Heartbeat>) -> Observation`.
3. **감시 루프 기본(띄우기, 끝남, 멈춤, 오디오 멈춤 한도, `--logon`) + `fake_app` + 프로세스 테스트**
   - 대상: `process.rs`의 `Target::Child`, `spawn(app, args)`(표준 입출력 null, current_dir = 앱 폴더), `pid`, `try_exit -> Option<Option<i32>>`, `kill`.
   - 설정: `run.rs`의 `Config::parse(args, exe_dir)`(기본값은 스펙 5.1), `default_dir()`.
   - 감시: `run(cfg)`. `supervisor.lock`을 잡는다(`OpenOptions` + `truncate(false)`). 띄우기 전에 heartbeat·clean_exit을 지운다. `--logon`은 첫 실행 성공에만 붙인다. 끝나면 clean_exit PID가 같을 때 감시를 끝내고, 아니면 충돌로 본다. 멈추면 강제 종료하고 닫는 중이었으면 감시를 끝낸다. 오디오 멈춤은 `AUDIO_STALL_LIMIT = 3`까지만 다시 띄우고 콜백이 다시 돌면 초기화한다. 무음 경고 수가 늘면 기록한다.
   - 진입점과 가짜 앱: `main.rs`, `fake_app.rs`(모드: ok, crash, clean, hang, close-hang, silent, audio-stall, front, dup, dup-after-peer, bystander).
4. **넘겨받기(`app.lock`이 잡혀 있을 때만), 중복 종료 코드 3, 두 번째 감시**
5. **첫 방 테마 시작(`api::show::api_theme_start`) + OSC `ThemeStart`를 단위 변형으로**
6. **앱 로그 파일** — 감시 기록(`supervisor.log`)도 내보내기에 포함하고, 회전에 실패해도 줄을 버리지 않는다(Sub 제안 반영).
7. **신호와 시작 관문(`core::app_signals`, `api::lifecycle`)**
8. **공연 상태 저장과 이어 가기(`core::show_state`, `api::show`)**
9. **무음 경고**
10. **Dart** — 관문, 하트비트, `clean_exit`, 스플래시에서 이어 가기, 환경설정 스위치, 통합 테스트는 `app.main(const [])`.
11. **installer와 CI**
12. **끝까지 확인·인계** — macOS 끝까지 확인 스크립트(Main), Windows 현장 점검표·HANDOFF(Sub).

---

### Task 1: 감시 크레이트 + 하트비트 형식 + 기록 회전

**상태:** 구현 파일·시험 있음(코드는 작업 트리의 파일을 옮겼다). Main 보고: 감시 크레이트 테스트 21개 통과, clippy 경고 0.

**Files:**
- Create: `supervisor/Cargo.toml`, `supervisor/.gitignore`, `supervisor/src/lib.rs`, `supervisor/src/heartbeat.rs`, `supervisor/src/logfile.rs`
- Test: `supervisor/tests/rules.rs`

**Interfaces:**
- Produces:
  - 상수 `heartbeat::{APP_LOCK, APP_PID, HEARTBEAT, CLEAN_EXIT, SUPERVISOR_LOCK}`.
  - `Heartbeat { pid: u32, ui_ms: u64, audio_cb_ms: u64, engine_active: bool, silent_warnings: u32 }`와 `parse(&str) -> Option<Heartbeat>`, `render(&self) -> String`.
  - `read_heartbeat(dir) -> Option<Heartbeat>`, `read_pid(path) -> Option<u32>`(숫자만이거나 `pid=` 형식), `atomic_write(path, &str) -> io::Result<()>`.
  - `logfile::{SUPERVISOR_LOG, MAX_LOG_BYTES, KEEP_ROTATED, append_line(dir, name, line, max, keep), log(dir, msg)}`.

**기록 회전 규칙(Sub 제안 반영):** 지금 파일이 크기를 넘으면 먼저 지금 파일을 `<이름>.rotating`으로 옮겨 본다. 옮기지 못하면 아무것도 건드리지 않고 그 줄은 지금 파일에 쓴다. 옮겼으면 오래된 것부터 한 칸씩 밀고 `.rotating`이 `.1`이 된다. 예전 방식은 오래된 파일부터 밀다가 지금 파일에서 실패하면 줄마다 오래된 기록을 하나씩 지웠다.

- [ ] **Step 1: 실패하는 테스트를 쓴다** — `supervisor/tests/rules.rs`

```rust
//! 감시 판단 규칙: 하트비트 형식, 기록 회전. 프로세스를 띄우지 않는다.
use atmos_supervisor::heartbeat::{atomic_write, read_heartbeat, read_pid, Heartbeat, HEARTBEAT};
use atmos_supervisor::logfile::{append_line, SUPERVISOR_LOG};
use std::path::{Path, PathBuf};

fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("atmos_sup_rules_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> =
        std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

#[test]
fn 하트비트는_다섯_키가_모두_있어야_읽는다() {
    let hb = Heartbeat {
        pid: 42,
        ui_ms: 1_700_000_000_123,
        audio_cb_ms: 1_700_000_000_100,
        engine_active: true,
        silent_warnings: 2,
    };
    assert_eq!(Heartbeat::parse(&hb.render()), Some(hb.clone()));
    assert_eq!(
        Heartbeat::parse("silent_warnings=0\nextra=x\nengine_active=0\naudio_cb_ms=0\nui_ms=5\npid=7\n"),
        Some(Heartbeat { pid: 7, ui_ms: 5, audio_cb_ms: 0, engine_active: false, silent_warnings: 0 }),
        "순서가 달라도, 모르는 키가 섞여도 읽는다"
    );
    assert_eq!(Heartbeat::parse("pid=7\nui_ms=5\naudio_cb_ms=0\nengine_active=1\n"), None, "키가 빠짐");
    assert_eq!(
        Heartbeat::parse("pid=7\nui_ms=5\naudio_cb_ms=0\nengine_active=1\nsilent_warnings="),
        None,
        "값이 잘림"
    );
    assert_eq!(
        Heartbeat::parse("pid=7\nui_ms=5\naudio_cb_ms=0\nengine_active=yes\nsilent_warnings=0\n"),
        None,
        "engine_active는 0/1"
    );
    assert_eq!(Heartbeat::parse(""), None);

    let dir = fresh_dir("hb");
    assert_eq!(read_heartbeat(&dir), None, "파일이 없으면 None");
    atomic_write(&dir.join(HEARTBEAT), &hb.render()).unwrap();
    assert_eq!(read_heartbeat(&dir), Some(hb));
    assert!(!dir.join(format!("{HEARTBEAT}.tmp")).exists(), "임시 파일이 남았다");

    std::fs::write(dir.join("a"), "123\n").unwrap();
    std::fs::write(dir.join("b"), "pid=456\n").unwrap();
    std::fs::write(dir.join("c"), "pid=\n").unwrap();
    assert_eq!(read_pid(&dir.join("a")), Some(123), "app.pid 형식");
    assert_eq!(read_pid(&dir.join("b")), Some(456), "clean_exit 형식");
    assert_eq!(read_pid(&dir.join("c")), None);
    assert_eq!(read_pid(&dir.join("none")), None);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 감시_기록은_크기를_넘으면_밀고_정한_개수만_남긴다() {
    let dir = fresh_dir("log");
    // 한 줄 10바이트("line-0000" + 줄바꿈), 파일당 30바이트를 넘으면 민다, 밀린 파일은 2개까지
    for i in 0..20 {
        append_line(&dir, SUPERVISOR_LOG, &format!("line-{i:04}"), 30, 2).unwrap();
    }
    assert_eq!(
        names(&dir),
        vec![SUPERVISOR_LOG.to_string(), format!("{SUPERVISOR_LOG}.1"), format!("{SUPERVISOR_LOG}.2")]
    );
    let current = std::fs::read_to_string(dir.join(SUPERVISOR_LOG)).unwrap();
    assert!(current.ends_with("line-0019\n"), "지금 파일 끝: {current:?}");
    let older = std::fs::read_to_string(dir.join(format!("{SUPERVISOR_LOG}.1"))).unwrap();
    assert!(older.contains("line-0015"), ".1 내용: {older:?}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 지금_파일을_옮기지_못하면_밀지_않고_그_줄은_지금_파일에_남긴다() {
    let dir = fresh_dir("log_stuck");
    std::fs::write(dir.join(format!("{SUPERVISOR_LOG}.1")), "old\n").unwrap();
    // 옮길 자리에 비어 있지 않은 폴더가 있어 지금 파일의 이름 바꾸기가 실패한다.
    // Windows에서 다른 프로그램이 기록 파일을 쥐고 있을 때와 같다.
    std::fs::create_dir_all(dir.join(format!("{SUPERVISOR_LOG}.rotating")).join("x")).unwrap();
    for i in 0..10 {
        append_line(&dir, SUPERVISOR_LOG, &format!("line-{i:04}"), 30, 2).unwrap();
    }
    let current = std::fs::read_to_string(dir.join(SUPERVISOR_LOG)).unwrap();
    assert_eq!(current.lines().count(), 10, "밀지 못해도 줄을 버리면 안 된다: {current:?}");
    assert_eq!(
        std::fs::read_to_string(dir.join(format!("{SUPERVISOR_LOG}.1"))).unwrap(),
        "old\n",
        "밀지 못했는데 오래된 기록이 바뀌었다"
    );
    assert!(!dir.join(format!("{SUPERVISOR_LOG}.2")).exists(), "밀지 못했는데 오래된 기록을 옮겼다");
    let _ = std::fs::remove_dir_all(dir);
}
```

- [ ] **Step 2: 실패를 확인한다**

Run(`atmos_mixer_pro/`에서): `cargo test --manifest-path supervisor/Cargo.toml --test rules`
Expected: 매니페스트(또는 모듈)가 없어 실패한다.

- [ ] **Step 3: 구현**

`supervisor/.gitignore`:
```
/target
```

`supervisor/Cargo.toml`:
```toml
[package]
name = "atmos_supervisor"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
# 감시 기록 시각을 앱 로그(chrono::Local)와 같은 현지 시각으로 남긴다.
chrono = { version = "0.4.45", default-features = false, features = ["clock"] }
```

`supervisor/src/lib.rs`(Task 2에서 `backoff`, `watch`를 더한다):
```rust
//! 충돌·멈춤 뒤 Atmos Mixer Pro를 다시 띄우는 감시 프로그램
//! (docs/superpowers/specs/2026-10-05-crash-relaunch-supervisor-design.md).
//! 판단 규칙은 프로세스 없이 테스트할 수 있게 모듈로 나눈다.
pub mod heartbeat;
pub mod logfile;
```

`supervisor/src/heartbeat.rs`:
```rust
//! 앱이 남기는 신호 파일(앱 로그 폴더). 앱(rust/src/core/app_signals.rs)과 이름·형식이 같아야 한다.
use std::path::Path;

pub const APP_LOCK: &str = "app.lock";
pub const APP_PID: &str = "app.pid";
pub const HEARTBEAT: &str = "heartbeat";
pub const CLEAN_EXIT: &str = "clean_exit";
pub const SUPERVISOR_LOCK: &str = "supervisor.lock";

/// 앱이 2초마다 쓰는 하트비트(`키=값` 줄).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heartbeat {
    pub pid: u32,
    /// UI가 하트비트를 부른 시각(밀리초). 값이 바뀌는지만 본다.
    pub ui_ms: u64,
    /// 마지막 오디오 콜백 시각(밀리초). 값이 바뀌는지만 본다.
    pub audio_cb_ms: u64,
    pub engine_active: bool,
    /// 앱이 남긴 재생 중 무음 경고 누적 횟수.
    pub silent_warnings: u32,
}

impl Heartbeat {
    /// 다섯 키가 모두 있어야 읽는다(빠진 키·잘린 값은 None).
    pub fn parse(text: &str) -> Option<Heartbeat> {
        let (mut pid, mut ui_ms, mut audio_cb_ms, mut engine_active, mut silent_warnings) =
            (None, None, None, None, None);
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else { continue };
            let value = value.trim();
            match key.trim() {
                "pid" => pid = value.parse().ok(),
                "ui_ms" => ui_ms = value.parse().ok(),
                "audio_cb_ms" => audio_cb_ms = value.parse().ok(),
                "engine_active" => {
                    engine_active = match value {
                        "1" => Some(true),
                        "0" => Some(false),
                        _ => None,
                    }
                }
                "silent_warnings" => silent_warnings = value.parse().ok(),
                _ => {}
            }
        }
        Some(Heartbeat {
            pid: pid?,
            ui_ms: ui_ms?,
            audio_cb_ms: audio_cb_ms?,
            engine_active: engine_active?,
            silent_warnings: silent_warnings?,
        })
    }

    pub fn render(&self) -> String {
        format!(
            "pid={}\nui_ms={}\naudio_cb_ms={}\nengine_active={}\nsilent_warnings={}\n",
            self.pid,
            self.ui_ms,
            self.audio_cb_ms,
            u8::from(self.engine_active),
            self.silent_warnings
        )
    }
}

/// [dir]의 하트비트. 없거나 형식이 맞지 않으면 None.
pub fn read_heartbeat(dir: &Path) -> Option<Heartbeat> {
    Heartbeat::parse(&std::fs::read_to_string(dir.join(HEARTBEAT)).ok()?)
}

/// 숫자만 있는 파일(app.pid) 또는 `pid=` 한 줄(clean_exit)에서 PID를 읽는다.
pub fn read_pid(path: &Path) -> Option<u32> {
    let text = std::fs::read_to_string(path).ok()?;
    let text = text.trim();
    text.strip_prefix("pid=").unwrap_or(text).trim().parse().ok()
}

/// 임시 파일에 쓰고 이름을 바꾼다. 읽는 쪽이 반쯤 쓴 내용을 보지 않는다.
pub fn atomic_write(path: &Path, contents: &str) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}
```

`supervisor/src/logfile.rs`:
```rust
//! 감시 기록(supervisor.log). 앱 로그(rust/src/core/log_file.rs)와 같은 규칙으로 민다: 10MB를 넘으면
//! .1, .2 …로 밀고 4개까지 남긴다. 감시는 앱 라이브러리를 쓰지 않으므로 규칙을 여기에 둔다.
use std::io::Write;
use std::path::Path;

pub const SUPERVISOR_LOG: &str = "supervisor.log";
pub const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;
pub const KEEP_ROTATED: usize = 4;

/// [dir]/[name]에 한 줄을 덧붙인다. 지금 파일이 [max_bytes] 이상이면 먼저 민다. 밀기에 실패해도
/// (Windows에서 다른 프로그램이 기록 파일을 쥐고 있으면 이름 바꾸기가 실패한다) 그 줄은 지금 파일에 남긴다.
pub fn append_line(dir: &Path, name: &str, line: &str, max_bytes: u64, keep: usize) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let current = dir.join(name);
    if std::fs::metadata(&current).map(|m| m.len() >= max_bytes).unwrap_or(false) {
        let _ = rotate(dir, name, keep);
    }
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&current)?;
    writeln!(file, "{line}")
}

/// 지금 파일을 옆 이름으로 먼저 옮겨 본다. 그게 안 되면 아무것도 건드리지 않는다 — 오래된 파일부터 밀면
/// 지금 파일에서 실패할 때마다(줄마다) 다시 밀면서 오래된 기록을 하나씩 지운다.
fn rotate(dir: &Path, name: &str, keep: usize) -> std::io::Result<()> {
    let current = dir.join(name);
    if keep == 0 {
        return std::fs::remove_file(current);
    }
    let rotating = dir.join(format!("{name}.rotating"));
    std::fs::rename(&current, &rotating)?;
    let rotated = |n: usize| dir.join(format!("{name}.{n}"));
    let _ = std::fs::remove_file(rotated(keep));
    for n in (1..keep).rev() {
        let from = rotated(n);
        if from.exists() {
            let _ = std::fs::rename(&from, rotated(n + 1));
        }
    }
    std::fs::rename(rotating, rotated(1))
}

/// 감시 기록 한 줄(현지 시각 + 내용). 기록에 실패해도 감시는 계속한다.
pub fn log(dir: &Path, msg: &str) {
    let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let _ = append_line(dir, SUPERVISOR_LOG, &format!("[{time}] {msg}"), MAX_LOG_BYTES, KEEP_ROTATED);
}
```

- [ ] **Step 4: 통과를 확인한다**

Run: `cargo test --manifest-path supervisor/Cargo.toml --test rules` → 위 시험 세 개 통과(이 시점에는 이 세 개만 있다).
Run: `cargo clippy --manifest-path supervisor/Cargo.toml --all-targets -- -D warnings` → 경고 0.

- [ ] **Step 5: 체크포인트** — `git status`에 `supervisor/` 새 파일만 보인다(`supervisor/target/`은 무시된다). 커밋은 사용자 요청 시.

---

### Task 2: 재실행 대기·멈춤 판단(순수)

**상태:** 구현 파일·시험 있음(코드는 작업 트리의 파일을 옮겼다). Main 보고: 감시 크레이트 테스트 21개 통과, clippy 경고 0.

**Files:**
- Create: `supervisor/src/backoff.rs`, `supervisor/src/watch.rs`
- Modify: `supervisor/src/lib.rs`(`pub mod backoff;`, `pub mod watch;` 추가)
- Test: `supervisor/tests/rules.rs`(추가)

**Interfaces:**
- Produces:
  - `backoff::Backoff::new(min, max, stable)`, `next_delay(&mut self, uptime: Duration) -> Duration`. uptime ≥ stable이면 min으로 돌아간다.
  - `watch::{SLEEP_GAP, Verdict {Ok, StartupHang, UiHang, AudioStall}, Observation { verdict, audio_progressed, gap }, Watch}`.
  - `Watch::new(now, first_heartbeat, hang, audio_stall)`, `observe(&mut self, now: Instant, hb: Option<&Heartbeat>) -> Observation`.
- Consumes: Task 1의 `Heartbeat`.

- [ ] **Step 1: 실패하는 테스트를 쓴다** — `supervisor/tests/rules.rs`에 더한다

import를 더한다:
```rust
//! (파일 머리 주석을 "…재실행 대기, 멈춤 판단…"으로 고친다.)
use atmos_supervisor::backoff::Backoff;
use atmos_supervisor::watch::{Verdict, Watch};
use std::time::{Duration, Instant};
```

시험을 더한다(보조 함수 `hb`, `S` 포함):
```rust
#[test]
fn 대기_시간은_두_배씩_늘고_최대에서_멈추며_오래_잘_돌면_처음으로_돌아간다() {
    let s = Duration::from_secs;
    let mut b = Backoff::new(s(2), s(60), s(300));
    let delays: Vec<u64> = (0..7).map(|_| b.next_delay(s(10)).as_secs()).collect();
    assert_eq!(delays, vec![2, 4, 8, 16, 32, 60, 60]);
    assert_eq!(b.next_delay(s(300)).as_secs(), 2, "5분 이상 잘 돌았으면 처음 값");
    assert_eq!(b.next_delay(s(299)).as_secs(), 4, "5분이 안 되면 이어서 늘어난다");
}

fn hb(ui: u64, audio: u64, engine: bool) -> Heartbeat {
    Heartbeat { pid: 1, ui_ms: ui, audio_cb_ms: audio, engine_active: engine, silent_warnings: 0 }
}

const S: fn(u64) -> Duration = Duration::from_secs;

#[test]
fn ui_하트비트가_30초_넘게_안_바뀌면_멈춤이고_바뀌면_괜찮다() {
    let t0 = Instant::now();
    let mut w = Watch::new(t0, S(90), S(30), S(120));
    let beat = hb(100, 0, false);
    // 1초마다 본다(간격이 5초를 넘지 않게). ui 100을 처음 본 것은 1초.
    for sec in 1..=31 {
        assert_eq!(w.observe(t0 + S(sec), Some(&beat)).verdict, Verdict::Ok, "{sec}초");
    }
    assert_eq!(w.observe(t0 + S(32), Some(&beat)).verdict, Verdict::UiHang);
    assert_eq!(w.observe(t0 + S(33), Some(&hb(101, 0, false))).verdict, Verdict::Ok, "값이 바뀌면 괜찮다");
}

#[test]
fn 하트비트가_한_번도_안_오면_기한_뒤_시작_중_멈춤이다() {
    let t0 = Instant::now();
    let mut w = Watch::new(t0, S(90), S(30), S(120));
    for sec in 1..=90 {
        assert_eq!(w.observe(t0 + S(sec), None).verdict, Verdict::Ok, "{sec}초");
    }
    assert_eq!(w.observe(t0 + S(91), None).verdict, Verdict::StartupHang);
}

#[test]
fn 엔진이_켜져_있는데_콜백_시각이_120초_넘게_멈추면_오디오_멈춤이다() {
    let t0 = Instant::now();
    let mut w = Watch::new(t0, S(90), S(30), S(120));
    // UI는 매초 바뀌고 콜백 시각(500)은 그대로. 500을 처음 본 것은 1초.
    for sec in 1..=121 {
        let seen = w.observe(t0 + S(sec), Some(&hb(sec, 500, true)));
        assert_eq!(seen.verdict, Verdict::Ok, "{sec}초");
        assert!(!seen.audio_progressed, "{sec}초: 콜백 시각이 안 바뀌었는데 돈다고 했다");
    }
    assert_eq!(w.observe(t0 + S(122), Some(&hb(122, 500, true))).verdict, Verdict::AudioStall);
    let seen = w.observe(t0 + S(123), Some(&hb(123, 501, true)));
    assert_eq!(seen.verdict, Verdict::Ok);
    assert!(seen.audio_progressed, "콜백 시각이 바뀌면 콜백이 돈다고 알려야 한다");

    // 엔진이 꺼져 있으면 오디오는 판단하지 않는다(장치 없음, 엔진 재기동 중)
    let mut w = Watch::new(t0, S(90), S(30), S(120));
    for sec in 1..=200 {
        assert_eq!(w.observe(t0 + S(sec), Some(&hb(sec, 500, false))).verdict, Verdict::Ok, "{sec}초");
    }
    // 엔진이 다시 켜지면 그때부터 잰다
    for sec in 201..=321 {
        assert_eq!(w.observe(t0 + S(sec), Some(&hb(sec, 500, true))).verdict, Verdict::Ok, "{sec}초");
    }
    assert_eq!(w.observe(t0 + S(322), Some(&hb(322, 500, true))).verdict, Verdict::AudioStall);
}

#[test]
fn 확인_간격이_벌어지면_절전에서_깨어난_것으로_보고_멈춤_시간을_다시_잰다() {
    let t0 = Instant::now();
    let mut w = Watch::new(t0, S(90), S(30), S(120));
    let beat = hb(100, 7, true);
    assert_eq!(w.observe(t0 + S(1), Some(&beat)).verdict, Verdict::Ok);
    // 10분 동안 확인이 없었다(사람이 절전시킴). 깨어난 직후 멀쩡한 앱을 죽이면 안 된다.
    let seen = w.observe(t0 + S(601), Some(&beat));
    assert!(seen.gap);
    assert_eq!(seen.verdict, Verdict::Ok);
    // 그 뒤로도 값이 안 바뀌면 다시 잰 시점부터 30초 뒤 멈춤
    for sec in 602..=631 {
        assert_eq!(w.observe(t0 + S(sec), Some(&beat)).verdict, Verdict::Ok, "{sec}초");
    }
    assert_eq!(w.observe(t0 + S(632), Some(&beat)).verdict, Verdict::UiHang);

    // 첫 하트비트를 기다리는 중에 벌어져도 기한을 다시 잰다
    let mut w = Watch::new(t0, S(90), S(30), S(120));
    assert_eq!(w.observe(t0 + S(1), None).verdict, Verdict::Ok);
    let seen = w.observe(t0 + S(500), None);
    assert!(seen.gap);
    assert_eq!(seen.verdict, Verdict::Ok);
}
```

- [ ] **Step 2: 실패를 확인한다**

Run: `cargo test --manifest-path supervisor/Cargo.toml --test rules`
Expected: `backoff`, `watch` 모듈이 없어 컴파일 실패.

- [ ] **Step 3: 구현**

`supervisor/src/lib.rs`에 `pub mod backoff;`와 `pub mod watch;`를 더한다(모듈 목록은 알파벳 순).

`supervisor/src/backoff.rs`:
```rust
//! 다시 띄우기 전 대기 시간. 실패할 때마다 두 배로 늘리고 최대에서 멈춘다. 앱이 오래 잘 돌았으면 처음
//! 값으로 돌아간다. 시작하자마자 죽는 경우 장치를 계속 두드리지 않게 한다.
use std::time::Duration;

pub struct Backoff {
    min: Duration,
    max: Duration,
    stable: Duration,
    next: Duration,
}

impl Backoff {
    pub fn new(min: Duration, max: Duration, stable: Duration) -> Self {
        Self { min, max, stable, next: min }
    }

    /// 앱이 [uptime] 동안 돌고 끝났다(충돌·멈춤). 이번에 기다릴 시간을 돌려준다.
    pub fn next_delay(&mut self, uptime: Duration) -> Duration {
        if uptime >= self.stable {
            self.next = self.min;
        }
        let delay = self.next;
        self.next = (self.next * 2).min(self.max);
        delay
    }
}
```

`supervisor/src/watch.rs`:
```rust
//! 앱 하나를 지켜보며 멈춤을 판단한다. 하트비트 값이 바뀐 시각을 감시 프로그램의 단조 시계로 잰다 —
//! 파일의 시각 값을 벽시계와 비교하지 않으므로 시계 조정(NTP, 시간대)에 영향받지 않는다.
use crate::heartbeat::Heartbeat;
use std::time::{Duration, Instant};

/// 확인 간격이 이보다 벌어지면(절전에서 깨어남 등) 멈춤 타이머를 다시 잰다.
pub const SLEEP_GAP: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Ok,
    /// 첫 하트비트가 기한 안에 오지 않았다.
    StartupHang,
    /// UI 하트비트(ui_ms)가 멈췄다.
    UiHang,
    /// 엔진이 켜져 있는데 오디오 콜백 시각이 멈췄다.
    AudioStall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Observation {
    pub verdict: Verdict,
    /// 이번에 오디오 콜백 시각이 바뀌었다(콜백이 돈다).
    pub audio_progressed: bool,
    /// 확인 간격이 벌어져 타이머를 다시 쟀다.
    pub gap: bool,
}

pub struct Watch {
    first_heartbeat: Duration,
    hang: Duration,
    audio_stall: Duration,
    started: Instant,
    last_tick: Instant,
    /// 마지막으로 본 ui_ms와 그 값으로 바뀐 시각.
    ui: Option<(u64, Instant)>,
    /// 마지막으로 본 audio_cb_ms와 그 값으로 바뀐 시각. 엔진이 꺼져 있으면 None(판단하지 않음).
    audio: Option<(u64, Instant)>,
}

impl Watch {
    /// [first_heartbeat]는 첫 하트비트 기한이다(새로 띄운 앱 90초, 넘겨받은 앱은 멈춤 판단과 같은 30초).
    pub fn new(now: Instant, first_heartbeat: Duration, hang: Duration, audio_stall: Duration) -> Self {
        Self { first_heartbeat, hang, audio_stall, started: now, last_tick: now, ui: None, audio: None }
    }

    /// 확인할 때마다 부른다. [hb]는 이 앱(PID가 같은)의 하트비트다.
    pub fn observe(&mut self, now: Instant, hb: Option<&Heartbeat>) -> Observation {
        let gap = now.saturating_duration_since(self.last_tick) > SLEEP_GAP;
        self.last_tick = now;
        if gap {
            self.started = now;
            if let Some(ui) = self.ui.as_mut() {
                ui.1 = now;
            }
            if let Some(audio) = self.audio.as_mut() {
                audio.1 = now;
            }
        }
        let mut audio_progressed = false;
        if let Some(hb) = hb {
            if self.ui.map(|(value, _)| value) != Some(hb.ui_ms) {
                self.ui = Some((hb.ui_ms, now));
            }
            if !hb.engine_active {
                self.audio = None;
            } else {
                match self.audio {
                    Some((value, _)) if value == hb.audio_cb_ms => {}
                    Some(_) => {
                        audio_progressed = hb.audio_cb_ms != 0;
                        self.audio = Some((hb.audio_cb_ms, now));
                    }
                    None => self.audio = Some((hb.audio_cb_ms, now)),
                }
            }
        }
        let since = |t: Instant| now.saturating_duration_since(t);
        let verdict = match self.ui {
            None if since(self.started) > self.first_heartbeat => Verdict::StartupHang,
            Some((_, changed)) if since(changed) > self.hang => Verdict::UiHang,
            _ => match self.audio {
                Some((_, changed)) if since(changed) > self.audio_stall => Verdict::AudioStall,
                _ => Verdict::Ok,
            },
        };
        Observation { verdict, audio_progressed, gap }
    }
}
```

- [ ] **Step 4: 통과를 확인한다**

Run: `cargo test --manifest-path supervisor/Cargo.toml --test rules` → 시험 8개 통과.
Run: `cargo clippy --manifest-path supervisor/Cargo.toml --all-targets -- -D warnings` → 경고 0.

- [ ] **Step 5: 체크포인트** — `git status`·`git diff --stat`. 커밋은 사용자 요청 시.

---

### Task 3: 감시 루프 기본(띄우기, 끝남, 멈춤, 오디오 멈춤 한도, `--logon`) + `fake_app` + 프로세스 테스트

**상태:** 구현 파일·시험 있음(코드는 작업 트리의 파일을 옮겼다). Main 보고: 감시 크레이트 테스트 21개 통과, clippy 경고 0.

**Files:**
- Create: `supervisor/src/process.rs`, `supervisor/src/run.rs`, `supervisor/src/main.rs`(`#![windows_subsystem = "windows"]`), `supervisor/src/bin/fake_app.rs`
- Modify: `supervisor/src/lib.rs`(`pub mod process;`, `pub mod run;` 추가)
- Test: `supervisor/tests/rules.rs`(`Config::parse` 시험 추가), `supervisor/tests/supervise.rs`
- `Cargo.toml`은 바뀌지 않는다. `src/main.rs`가 배포 바이너리 `atmos_supervisor`, `src/bin/fake_app.rs`가 테스트용 `fake_app`이다(배포에는 넣지 않는다).

**Interfaces:**
- Produces:
  - `process::Target::Child`: `spawn(app, args)`(표준 입출력 null, current_dir = 앱 폴더), `pid`, `try_exit -> Option<Option<i32>>`, `kill`.
  - `run::Config::parse(args, exe_dir)`: 기본값은 스펙 5.1. 인자 `--app`, `--dir`, `--hang-secs`, `--first-heartbeat-secs`, `--audio-stall-secs`, `--logon`, 테스트 전용 `--tick-ms`, `--backoff-min-ms`. `default_dir()`.
  - `run::run(cfg)`: 감시 루프, `AUDIO_STALL_LIMIT = 3`.
- Consumes: Task 1·2의 `Heartbeat`, `logfile::log`, `Backoff`, `Watch`.

**동작(스펙 5절):** `supervisor.lock`을 잡는다(이미 잡혀 있으면 바로 끝). 띄우기 전에 heartbeat·clean_exit을 지운다. `--logon`은 첫 실행 성공에만 붙인다. 앱이 끝나면 clean_exit의 PID가 이 앱일 때 감시를 끝내고 아니면 충돌로 본다. 멈추면 강제 종료하고 닫는 중이었으면 감시를 끝낸다(Review Focus 2). 오디오 멈춤은 연속 3번까지만 다시 띄운다.

- [ ] **Step 1: 실패하는 테스트를 쓴다**

`supervisor/tests/rules.rs`에 import와 시험을 더한다:
```rust
use atmos_supervisor::run::Config;
```

```rust
#[test]
fn 인자가_없으면_스펙_기본값이고_판단_시간과_폴더를_바꿀_수_있다() {
    let exe_dir = Path::new("/opt/atmos");
    let cfg = Config::parse(&[], exe_dir).unwrap();
    assert_eq!((cfg.hang, cfg.first_heartbeat, cfg.audio_stall), (S(30), S(90), S(120)));
    assert_eq!(cfg.tick, S(1));
    assert_eq!((cfg.backoff_min, cfg.backoff_max, cfg.stable), (S(2), S(60), S(300)));
    assert!(!cfg.logon);
    assert_eq!(cfg.app.parent(), Some(exe_dir), "기본 앱은 감시 프로그램 옆에 있다");
    assert_eq!(cfg.dir, std::env::temp_dir().join("atmos_mixer_pro_logs"), "앱 로그 폴더와 같아야 한다");

    let args: Vec<String> = [
        "--logon", "--app", "/x/app", "--dir", "/tmp/d", "--hang-secs", "2.5", "--first-heartbeat-secs", "3",
        "--audio-stall-secs", "4", "--tick-ms", "100", "--backoff-min-ms", "200",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let cfg = Config::parse(&args, exe_dir).unwrap();
    assert!(cfg.logon);
    assert_eq!(cfg.app, PathBuf::from("/x/app"));
    assert_eq!(cfg.dir, PathBuf::from("/tmp/d"));
    assert_eq!(cfg.hang, Duration::from_millis(2500));
    assert_eq!((cfg.first_heartbeat, cfg.audio_stall), (S(3), S(4)));
    assert_eq!((cfg.tick, cfg.backoff_min), (Duration::from_millis(100), Duration::from_millis(200)));

    let bad = |list: &[&str]| {
        Config::parse(&list.iter().map(|s| s.to_string()).collect::<Vec<_>>(), exe_dir).is_err()
    };
    assert!(bad(&["--hang-secs"]), "값이 빠짐");
    assert!(bad(&["--hang-secs", "0"]), "0초");
    assert!(bad(&["--hang-secs", "abc"]));
    assert!(bad(&["--tick-ms", "-5"]));
    assert!(bad(&["--what"]), "모르는 인자");
}
```

`supervisor/tests/supervise.rs`(가짜 앱 `fake_app`을 실제 프로세스로 띄운다. 스펙 7절 1~5, 닫는 중 멈춤, `--logon` 첫 실행만, 그리고 `앱_경로를_상대_경로로_줘도_띄운다` — 실제 앱 확인에서 찾은 버그로, `--app`에 상대 경로를 주면 작업 폴더를 앱 폴더로 바꾼 뒤 그 기준으로 다시 찾아 앱을 못 띄웠다. 고치기 전에는 실패하는 것을 Main이 확인했다. 도우미 `supervisor_with_app`이 `supervisor()` 밑에 있다):
```rust
//! 감시 프로그램을 실제 프로세스로 띄워 확인한다. 앱 대신 가짜 앱(src/bin/fake_app.rs)을 띄운다.
//! 판단 시간은 짧게 준다(멈춤·첫 하트비트·오디오 멈춤 2초, 확인 0.1초, 첫 대기 0.2초).
//! 테스트마다 폴더가 따로라 함께 돌아도 된다.
use atmos_supervisor::heartbeat::{read_pid, APP_LOCK, APP_PID};

use std::path::{Path, PathBuf};

use std::process::{Child, Command, ExitStatus, Stdio};

use std::sync::atomic::{AtomicBool, Ordering};

use std::sync::Arc;

use std::time::{Duration, Instant};

const SUPERVISOR: &str = env!("CARGO_BIN_EXE_atmos_supervisor");

const FAKE_APP: &str = env!("CARGO_BIN_EXE_fake_app");

fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("atmos_sup_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 띄운 감시 프로그램. 테스트가 실패해도 남지 않게 drop에서 끝낸다.
struct Supervised(Child);

impl Supervised {
    fn wait_exit(&mut self, secs: u64, what: &str) -> ExitStatus {
        let end = Instant::now() + Duration::from_secs(secs);
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < end, "{what}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn running(&mut self) -> bool {
        self.0.try_wait().unwrap().is_none()
    }
}

impl Drop for Supervised {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// 감시 프로그램을 띄운다. [env]는 가짜 앱 동작(FAKE_FIRST 등)이고 감시가 띄우는 앱이 물려받는다.
fn supervisor(dir: &Path, extra: &[&str], env: &[(&str, &str)]) -> Supervised {
    supervisor_with_app(Path::new(FAKE_APP), None, dir, extra, env)
}

/// [app] 경로로 앱을 띄우는 감시 프로그램. [cwd]가 있으면 그 폴더에서 감시를 띄운다.
fn supervisor_with_app(
    app: &Path,
    cwd: Option<&Path>,
    dir: &Path,
    extra: &[&str],
    env: &[(&str, &str)],
) -> Supervised {
    let mut command = Command::new(SUPERVISOR);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command
        .arg("--app")
        .arg(app)
        .arg("--dir")
        .arg(dir)
        .args(["--hang-secs", "2", "--first-heartbeat-secs", "2", "--audio-stall-secs", "2"])
        .args(["--tick-ms", "100", "--backoff-min-ms", "200"])
        .args(extra)
        .env("FAKE_DIR", dir)
        .env_remove("FAKE_MODE")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (key, value) in env {
        command.env(key, value);
    }
    Supervised(command.spawn().unwrap())
}

/// 가짜 앱 실행 기록(줄마다 "PID 인자…")
fn launches(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("launches")).unwrap_or_default().lines().map(str::to_owned).collect()
}

/// 실행 기록의 인자 부분
fn launch_args(dir: &Path) -> Vec<String> {
    launches(dir)
        .iter()
        .map(|line| line.split_once(' ').map(|(_, args)| args.to_owned()).unwrap_or_default())
        .collect()
}

fn log_text(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("supervisor.log")).unwrap_or_default()
}

fn wait_until(what: &str, secs: u64, cond: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(secs);
    while !cond() {
        assert!(Instant::now() < end, "{what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// 지금 app.pid
fn app_pid(dir: &Path) -> Option<u32> {
    read_pid(&dir.join(APP_PID))
}

/// 가짜 앱 [pid]에 "crash"(표시 없이 끝남) 또는 "close"(clean_exit을 남기고 끝남)를 요청한다
fn request(dir: &Path, what: &str, pid: u32) {
    std::fs::write(dir.join(format!("{what}-{pid}")), "").unwrap();
}

#[test]
fn 닫는다는_표시를_남기고_끝나면_종료_코드가_1이어도_다시_띄우지_않고_감시도_끝난다() {
    let dir = fresh_dir("clean");
    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "clean")]);
    let status = sup.wait_exit(10, "운영자가 닫았는데 감시가 끝나지 않았다");
    assert!(status.success(), "{status:?}");
    assert_eq!(launch_args(&dir), vec!["--supervised"], "다시 띄웠다");
    assert!(log_text(&dir).contains("운영자가 앱을 닫았다"), "{}", log_text(&dir));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 표시_없이_끝나면_auto_relaunched로_다시_띄운다() {
    let dir = fresh_dir("crash");
    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "crash"), ("FAKE_NEXT", "clean")]);
    sup.wait_exit(15, "충돌 뒤 다시 띄운 앱이 닫혔는데 감시가 끝나지 않았다");
    assert_eq!(launch_args(&dir), vec!["--supervised", "--supervised --auto-relaunched"]);
    assert!(log_text(&dir).contains("충돌"), "{}", log_text(&dir));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 하트비트가_멈추면_강제_종료하고_다시_띄운다() {
    let dir = fresh_dir("hang");
    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "hang"), ("FAKE_NEXT", "clean")]);
    sup.wait_exit(20, "멈춘 앱을 다시 띄웠고 그 앱이 닫혔는데 감시가 끝나지 않았다");
    assert_eq!(launch_args(&dir), vec!["--supervised", "--supervised --auto-relaunched"]);
    assert!(log_text(&dir).contains("UI 멈춤"), "{}", log_text(&dir));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 하트비트가_한_번도_안_오면_첫_하트비트_기한_뒤_다시_띄운다() {
    let dir = fresh_dir("silent");
    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "silent"), ("FAKE_NEXT", "clean")]);
    sup.wait_exit(20, "하트비트 없는 앱을 다시 띄웠고 그 앱이 닫혔는데 감시가 끝나지 않았다");
    assert_eq!(launch_args(&dir), vec!["--supervised", "--supervised --auto-relaunched"]);
    assert!(log_text(&dir).contains("시작 중 멈춤"), "{}", log_text(&dir));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 운영자가_닫는_중에_멈추면_강제_종료하고_다시_띄우지_않는다() {
    let dir = fresh_dir("close_hang");
    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "close-hang"), ("FAKE_NEXT", "ok")]);
    let status = sup.wait_exit(15, "닫는 중 멈춘 앱을 정리하고 감시가 끝나야 한다");
    assert!(status.success(), "{status:?}");
    assert_eq!(launch_args(&dir), vec!["--supervised"], "운영자가 닫은 앱을 다시 띄웠다");
    assert!(log_text(&dir).contains("닫는 중 멈춘"), "{}", log_text(&dir));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 로그인_자동_실행이면_첫_실행에만_logon을_넘긴다() {
    let dir = fresh_dir("logon");
    let mut sup = supervisor(&dir, &["--logon"], &[("FAKE_FIRST", "crash"), ("FAKE_NEXT", "clean")]);
    sup.wait_exit(15, "감시가 끝나지 않았다");
    assert_eq!(launch_args(&dir), vec!["--supervised --logon", "--supervised --auto-relaunched"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 오디오_멈춤은_연속_3번까지만_다시_띄우고_콜백이_돌면_횟수를_초기화한다() {
    let dir = fresh_dir("audio");
    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "audio-stall"), ("FAKE_NEXT", "audio-stall")]);
    wait_until("오디오 멈춤 한도에 걸리지 않았다", 40, || {
        log_text(&dir).contains("더는 오디오 멈춤으로 다시 띄우지 않는다")
    });
    assert_eq!(launches(&dir).len(), 4, "첫 실행 + 다시 띄우기 3번: {}", log_text(&dir));
    assert!(log_text(&dir).contains("오디오 멈춤"), "{}", log_text(&dir));
    std::thread::sleep(Duration::from_secs(3)); // 오디오 멈춤 판단(2초)이 지나도
    assert_eq!(launches(&dir).len(), 4, "한도 뒤에도 오디오 멈춤으로 다시 띄웠다");
    assert!(sup.running(), "감시가 끝났다");

    // 장치가 돌아와 콜백이 다시 돈다
    std::fs::write(dir.join("audio-ok"), "").unwrap();
    wait_until("콜백이 다시 도는데 횟수를 초기화하지 않았다", 10, || log_text(&dir).contains("재실행 횟수를 초기화"));
    request(&dir, "close", app_pid(&dir).unwrap());
    sup.wait_exit(10, "앱을 닫았는데 감시가 끝나지 않았다");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 앱_경로를_상대_경로로_줘도_띄운다() {
    let dir = fresh_dir("relative");
    // target/에서 감시를 띄우고 앱을 "debug/fake_app"처럼 준다. 감시는 앱 폴더를 작업 폴더로 삼는다.
    let fake = Path::new(FAKE_APP);
    let base = fake.parent().unwrap().parent().unwrap();
    let relative = fake.strip_prefix(base).unwrap();
    let mut sup = supervisor_with_app(relative, Some(base), &dir, &[], &[("FAKE_FIRST", "clean")]);
    sup.wait_exit(10, "상대 경로로 준 앱이 떠서 닫혀야 감시가 끝난다");
    assert_eq!(launch_args(&dir), vec!["--supervised"], "{}", log_text(&dir));
    let _ = std::fs::remove_dir_all(dir);
}
```

- [ ] **Step 2: 실패를 확인한다**

Run(`atmos_mixer_pro/`에서): `cargo test --manifest-path supervisor/Cargo.toml --test rules --test supervise`
Expected: `run`, `process` 모듈과 바이너리가 없어 컴파일 실패.

- [ ] **Step 3: 구현**

`supervisor/src/lib.rs`는 아래 모듈 목록이 된다:
```rust
//! 충돌·멈춤 뒤 Atmos Mixer Pro를 다시 띄우는 감시 프로그램
//! (docs/superpowers/specs/2026-10-05-crash-relaunch-supervisor-design.md).
//! 판단 규칙은 프로세스 없이 테스트할 수 있게 모듈로 나눈다.
pub mod backoff;
pub mod heartbeat;
pub mod logfile;
pub mod process;
pub mod run;
pub mod watch;
```

`supervisor/src/process.rs`:
```rust
//! 감시 대상 앱 프로세스.
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// 감시 대상 앱.
pub enum Target {
    /// 감시가 띄운 앱.
    Child(Child),
}

impl Target {
    /// 앱을 띄운다. 감시는 창 없이 돌므로 표준 입출력은 쓰지 않는다.
    pub fn spawn(app: &Path, args: &[String]) -> std::io::Result<Target> {
        // 작업 폴더를 앱 폴더로 바꾸므로 상대 경로는 먼저 절대 경로로 만든다(유닉스는 바뀐 작업 폴더에서 찾는다).
        let app = std::path::absolute(app)?;
        let mut command = Command::new(&app);
        command.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        if let Some(dir) = app.parent().filter(|d| !d.as_os_str().is_empty()) {
            command.current_dir(dir);
        }
        Ok(Target::Child(command.spawn()?))
    }

    pub fn pid(&self) -> u32 {
        match self {
            Target::Child(child) => child.id(),
        }
    }

    /// 끝났으면 Some(종료 코드). 종료 코드를 알 수 없으면(신호로 끝남) Some(None).
    pub fn try_exit(&mut self) -> Option<Option<i32>> {
        match self {
            Target::Child(child) => match child.try_wait() {
                Ok(Some(status)) => Some(status.code()),
                Ok(None) => None,
                Err(_) => Some(None),
            },
        }
    }

    /// 강제 종료하고 끝날 때까지 기다린다.
    pub fn kill(&mut self) {
        match self {
            Target::Child(child) => {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}
```

`supervisor/src/run.rs`:
```rust
//! 감시 루프(스펙 5절). 앱을 띄워 [Config::tick]마다 보고, 충돌·멈춤이면 기다렸다 다시 띄운다.
//! 운영자가 닫았으면(clean_exit의 PID가 이 앱) 다시 띄우지 않고 감시도 끝낸다.
use crate::backoff::Backoff;
use crate::heartbeat::{read_heartbeat, read_pid, CLEAN_EXIT, HEARTBEAT, SUPERVISOR_LOCK};
use crate::logfile::log;
use crate::process::Target;
use crate::watch::{Verdict, Watch};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// 오디오 멈춤으로는 연속 이 횟수까지만 다시 띄운다(스펙 5.3).
pub const AUDIO_STALL_LIMIT: u32 = 3;

#[cfg(windows)]
const DEFAULT_APP: &str = "atmos_mixer_pro.exe";
#[cfg(not(windows))]
const DEFAULT_APP: &str = "atmos_mixer_pro";

/// 감시 설정. 기본값은 스펙 5.1이다. 시간 인자는 테스트와 macOS 확인에서 짧게 준다.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// 띄울 앱(기본: 감시 프로그램과 같은 폴더의 앱).
    pub app: PathBuf,
    /// 신호 파일과 감시 기록을 두는 폴더(기본: 앱 로그 폴더).
    pub dir: PathBuf,
    /// 로그인 자동 실행으로 시작했다(첫 실행에 --logon을 넘긴다).
    pub logon: bool,
    /// UI 하트비트가 이만큼 안 바뀌면 멈춤.
    pub hang: Duration,
    /// 새로 띄운 앱의 첫 하트비트 기한.
    pub first_heartbeat: Duration,
    /// 엔진이 켜져 있는데 오디오 콜백 시각이 이만큼 안 바뀌면 오디오 멈춤.
    pub audio_stall: Duration,
    /// 확인 간격.
    pub tick: Duration,
    pub backoff_min: Duration,
    pub backoff_max: Duration,
    /// 앱이 이만큼 잘 돌았으면 대기 시간을 처음 값으로 돌린다.
    pub stable: Duration,
}

/// 앱 로그 폴더. 앱의 `core::log_file::log_dir()`(rust/src/core/log_file.rs)와 같아야 한다.
pub fn default_dir() -> PathBuf {
    std::env::temp_dir().join("atmos_mixer_pro_logs")
}

impl Config {
    /// 인자를 읽는다. [exe_dir]는 감시 프로그램이 있는 폴더다.
    pub fn parse(args: &[String], exe_dir: &Path) -> Result<Config, String> {
        let mut cfg = Config {
            app: exe_dir.join(DEFAULT_APP),
            dir: default_dir(),
            logon: false,
            hang: Duration::from_secs(30),
            first_heartbeat: Duration::from_secs(90),
            audio_stall: Duration::from_secs(120),
            tick: Duration::from_secs(1),
            backoff_min: Duration::from_secs(2),
            backoff_max: Duration::from_secs(60),
            stable: Duration::from_secs(300),
        };
        let mut it = args.iter();
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--logon" => cfg.logon = true,
                "--app" => cfg.app = PathBuf::from(value(&mut it, arg)?),
                "--dir" => cfg.dir = PathBuf::from(value(&mut it, arg)?),
                "--hang-secs" => cfg.hang = secs(arg, value(&mut it, arg)?)?,
                "--first-heartbeat-secs" => cfg.first_heartbeat = secs(arg, value(&mut it, arg)?)?,
                "--audio-stall-secs" => cfg.audio_stall = secs(arg, value(&mut it, arg)?)?,
                "--tick-ms" => cfg.tick = millis(arg, value(&mut it, arg)?)?,
                "--backoff-min-ms" => cfg.backoff_min = millis(arg, value(&mut it, arg)?)?,
                other => return Err(format!("모르는 인자: {other}")),
            }
        }
        Ok(cfg)
    }
}

fn value<'a>(it: &mut std::slice::Iter<'a, String>, name: &str) -> Result<&'a str, String> {
    it.next().map(String::as_str).ok_or_else(|| format!("{name} 뒤에 값이 없다"))
}

fn secs(name: &str, text: &str) -> Result<Duration, String> {
    match text.parse::<f64>() {
        Ok(v) if v.is_finite() && v > 0.0 => Ok(Duration::from_secs_f64(v)),
        _ => Err(format!("{name} 값이 잘못됐다: {text}")),
    }
}

fn millis(name: &str, text: &str) -> Result<Duration, String> {
    match text.parse::<u64>() {
        Ok(v) if v > 0 => Ok(Duration::from_millis(v)),
        _ => Err(format!("{name} 값이 잘못됐다: {text}")),
    }
}

/// 감시 대상 하나를 지켜본 결과.
enum Ending {
    /// 프로세스가 끝났다(종료 코드를 모르면 None).
    Exited(Option<i32>),
    /// 멈춤으로 판단했다(아직 살아 있다).
    Hung(Verdict),
}

struct Supervisor<'a> {
    cfg: &'a Config,
    backoff: Backoff,
    /// 다음 실행에 --auto-relaunched를 붙인다(충돌·멈춤 뒤).
    relaunch: bool,
    /// 아직 앱을 한 번도 띄우지 못했다(--logon은 첫 실행에만 넘긴다).
    first_launch: bool,
    /// 오디오 멈춤으로 연속 다시 띄운 횟수. 콜백이 다시 돌면 0으로 돌아간다.
    audio_relaunches: u32,
    /// 한도에 걸린 뒤 "더는 다시 띄우지 않는다"를 한 번만 남긴다.
    audio_gave_up_logged: bool,
    /// 마지막으로 본 무음 경고 누적 횟수.
    silent_warnings: u32,
}

/// 감시를 돌린다. 운영자가 앱을 닫았거나 감시가 이미 돌고 있으면 돌아온다.
pub fn run(cfg: &Config) {
    let _ = std::fs::create_dir_all(&cfg.dir);
    let Some(_lock) = acquire_lock(&cfg.dir.join(SUPERVISOR_LOCK)) else {
        log(&cfg.dir, "감시가 이미 돌고 있다 — 끝낸다");
        return;
    };
    log(
        &cfg.dir,
        &format!(
            "감시 시작(앱 {}, 멈춤 {:?}, 첫 하트비트 {:?}, 오디오 멈춤 {:?}{})",
            cfg.app.display(),
            cfg.hang,
            cfg.first_heartbeat,
            cfg.audio_stall,
            if cfg.logon { ", 로그인 자동 실행" } else { "" }
        ),
    );
    Supervisor {
        cfg,
        backoff: Backoff::new(cfg.backoff_min, cfg.backoff_max, cfg.stable),
        relaunch: false,
        first_launch: true,
        audio_relaunches: 0,
        audio_gave_up_logged: false,
        silent_warnings: 0,
    }
    .run_loop();
}

/// [path]를 열어 잠근다. 다른 프로세스가 쥐고 있으면 None.
fn acquire_lock(path: &Path) -> Option<File> {
    let file = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(path).ok()?;
    file.try_lock().ok()?;
    Some(file)
}

impl Supervisor<'_> {
    fn run_loop(&mut self) {
        loop {
            let Some((mut target, adopted)) = self.next_target() else {
                // 앱을 띄우지 못했다(파일 없음 등). 기다렸다 다시 해 본다.
                self.wait_before_relaunch(Duration::ZERO);
                continue;
            };
            let started = Instant::now();
            match self.watch(&mut target, adopted) {
                Ending::Exited(code) if self.closed_by_operator(&target) => {
                    self.log(&format!("운영자가 앱을 닫았다(pid {}, 종료 코드 {code:?}) — 감시를 끝낸다", target.pid()));
                    return;
                }
                Ending::Exited(code) => {
                    self.log(&format!(
                        "앱이 닫는다는 표시 없이 끝났다(충돌, pid {}, 종료 코드 {code:?})",
                        target.pid()
                    ));
                }
                Ending::Hung(verdict) => {
                    let closing = self.closed_by_operator(&target);
                    target.kill();
                    if closing {
                        self.log(&format!(
                            "운영자가 닫는 중 멈춘 앱을 강제 종료했다(pid {}) — 감시를 끝낸다",
                            target.pid()
                        ));
                        return;
                    }
                    if verdict == Verdict::AudioStall {
                        self.audio_relaunches += 1;
                    }
                    self.log(&format!("{} — 앱을 강제 종료했다(pid {})", self.describe(verdict), target.pid()));
                }
            }
            self.relaunch = true;
            self.wait_before_relaunch(started.elapsed());
        }
    }

    /// 다음 감시 대상: (대상, 넘겨받았나).
    fn next_target(&mut self) -> Option<(Target, bool)> {
        self.launch().map(|target| (target, false))
    }

    fn launch(&mut self) -> Option<Target> {
        // 지난 앱의 신호를 새 앱의 것으로 읽지 않게 지운다.
        let _ = std::fs::remove_file(self.cfg.dir.join(HEARTBEAT));
        let _ = std::fs::remove_file(self.cfg.dir.join(CLEAN_EXIT));
        let mut args = vec!["--supervised".to_string()];
        if self.relaunch {
            args.push("--auto-relaunched".into());
        }
        if self.first_launch && self.cfg.logon {
            args.push("--logon".into());
        }
        match Target::spawn(&self.cfg.app, &args) {
            Ok(target) => {
                self.first_launch = false;
                self.log(&format!("앱을 띄웠다(pid {}, 인자 {})", target.pid(), args.join(" ")));
                Some(target)
            }
            Err(e) => {
                self.log(&format!("앱을 띄우지 못했다({}): {e}", self.cfg.app.display()));
                None
            }
        }
    }

    /// 앱이 끝나거나 멈출 때까지 본다.
    fn watch(&mut self, target: &mut Target, adopted: bool) -> Ending {
        // 넘겨받은 앱은 하트비트가 이미 있어야 하므로 첫 하트비트 기한을 멈춤 판단과 같게 둔다.
        let first = if adopted { self.cfg.hang } else { self.cfg.first_heartbeat };
        let mut watch = Watch::new(Instant::now(), first, self.cfg.hang, self.cfg.audio_stall);
        loop {
            std::thread::sleep(self.cfg.tick);
            if let Some(code) = target.try_exit() {
                return Ending::Exited(code);
            }
            let heartbeat = read_heartbeat(&self.cfg.dir).filter(|h| h.pid == target.pid());
            if let Some(h) = &heartbeat {
                self.note_silent_warnings(h.silent_warnings);
            }
            let seen = watch.observe(Instant::now(), heartbeat.as_ref());
            if seen.gap {
                self.log("확인 간격이 크게 벌어졌다(절전에서 깨어남 등) — 멈춤 판단 시간을 다시 잰다");
            }
            if seen.audio_progressed && self.audio_relaunches > 0 {
                self.log("오디오 콜백이 다시 돈다 — 오디오 멈춤 재실행 횟수를 초기화한다");
                self.audio_relaunches = 0;
                self.audio_gave_up_logged = false;
            }
            match seen.verdict {
                Verdict::Ok => {}
                Verdict::AudioStall if self.audio_relaunches >= AUDIO_STALL_LIMIT => {
                    if !self.audio_gave_up_logged {
                        self.log(&format!(
                            "오디오 멈춤이 이어지지만 연속 {AUDIO_STALL_LIMIT}번 다시 띄워도 그대로라 \
                             더는 오디오 멈춤으로 다시 띄우지 않는다(장치를 확인하세요)"
                        ));
                        self.audio_gave_up_logged = true;
                    }
                }
                verdict => return Ending::Hung(verdict),
            }
        }
    }

    fn closed_by_operator(&self, target: &Target) -> bool {
        read_pid(&self.cfg.dir.join(CLEAN_EXIT)) == Some(target.pid())
    }

    fn note_silent_warnings(&mut self, count: u32) {
        if count > self.silent_warnings {
            self.log(&format!("앱이 재생 중 무음 경고를 남겼다(누적 {count}회 — 앱 로그 참고)"));
        }
        self.silent_warnings = count;
    }

    fn describe(&self, verdict: Verdict) -> String {
        match verdict {
            Verdict::StartupHang => "시작 중 멈춤: 첫 하트비트가 기한 안에 오지 않았다".into(),
            Verdict::UiHang => {
                format!("UI 멈춤: 하트비트가 {:.0}초 넘게 바뀌지 않았다", self.cfg.hang.as_secs_f64())
            }
            Verdict::AudioStall => format!(
                "오디오 멈춤: 엔진이 켜져 있는데 오디오 콜백이 {:.0}초 넘게 돌지 않았다(연속 {}번째)",
                self.cfg.audio_stall.as_secs_f64(),
                self.audio_relaunches
            ),
            Verdict::Ok => "정상".into(),
        }
    }

    fn wait_before_relaunch(&mut self, uptime: Duration) {
        let delay = self.backoff.next_delay(uptime);
        self.log(&format!("{:.1}초 뒤 다시 띄운다", delay.as_secs_f64()));
        std::thread::sleep(delay);
    }

    fn log(&self, msg: &str) {
        log(&self.cfg.dir, msg);
    }
}
```

`supervisor/src/main.rs`:
```rust
// 감시 프로그램은 창 없이 돈다(Windows에서 콘솔 창이 뜨지 않게).
#![windows_subsystem = "windows"]
//! 충돌·멈춤 뒤 Atmos Mixer Pro를 다시 띄우는 감시 프로그램. 인자는 `run::Config::parse` 참고.
use atmos_supervisor::logfile::log;
use atmos_supervisor::run::{default_dir, run, Config};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    match Config::parse(&args, &exe_dir) {
        Ok(cfg) => run(&cfg),
        Err(msg) => {
            log(&default_dir(), &format!("감시를 시작하지 못했다: {msg}"));
            std::process::exit(2);
        }
    }
}
```

`supervisor/src/bin/fake_app.rs`(테스트용 가짜 앱):
```rust
//! 감시 프로그램 테스트용 가짜 앱. 앱처럼 app.lock을 쥐고 app.pid·heartbeat·clean_exit을 남긴다.
//! 배포에는 넣지 않는다(CI는 atmos_supervisor만 복사한다).
//!
//! 동작은 환경 변수로 정한다. FAKE_MODE가 있으면 그것을 쓴다. 없으면 --supervised가 없을 때 "front"
//! (앱 시작 관문의 중복 실행 흉내), --auto-relaunched면 FAKE_NEXT, 아니면 FAKE_FIRST다(기본 "ok").
//! 실행할 때마다 FAKE_DIR/launches에 "PID 인자…"를 남긴다. FAKE_DIR/crash-<PID>가 생기면 표시 없이 1로,
//! close-<PID>가 생기면 clean_exit을 남기고 0으로 끝난다. 테스트가 실패해 남더라도 2분 뒤에는 스스로 끝난다.
use atmos_supervisor::heartbeat::{atomic_write, read_pid, Heartbeat, APP_LOCK, APP_PID, CLEAN_EXIT, HEARTBEAT};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};
use std::time::{Duration, Instant};

/// 하트비트 간격(앱은 2초, 테스트는 짧게).
const BEAT: Duration = Duration::from_millis(100);
/// crash·clean·hang·close-hang 모드가 정상으로 도는 시간.
const BRIEF: Duration = Duration::from_millis(300);
/// 테스트가 실패해 아무도 닫지 않아도 이 시간 뒤에는 끝난다.
const MAX_LIFE: Duration = Duration::from_secs(120);

fn append(path: &Path, line: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

fn mark_clean_exit(dir: &Path, pid: u32) {
    let _ = atomic_write(&dir.join(CLEAN_EXIT), &format!("pid={pid}\n"));
}

/// 다른 앱을 먼저 띄워 두고 그 앱이 app.pid를 쓸 때까지 기다린다. 이 가짜 앱이 곧 끝나므로
/// 먼저 띄운 앱은 init이 거둔다(기다리지 않는다).
#[allow(clippy::zombie_processes)]
fn spawn_peer(dir: &Path) {
    let peer = Command::new(std::env::current_exe().expect("현재 exe"))
        .env("FAKE_MODE", "ok")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("먼저 띄울 앱");
    let end = Instant::now() + Duration::from_secs(5);
    while read_pid(&dir.join(APP_PID)) != Some(peer.id()) && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn main() {
    let dir = PathBuf::from(std::env::var("FAKE_DIR").expect("FAKE_DIR"));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let pid = std::process::id();
    append(&dir.join("launches"), &format!("{pid} {}", args.join(" ")));
    let has = |flag: &str| args.iter().any(|a| a == flag);
    let mode = match std::env::var("FAKE_MODE") {
        Ok(mode) => mode,
        Err(_) if !has("--supervised") => "front".to_string(),
        Err(_) => {
            let key = if has("--auto-relaunched") { "FAKE_NEXT" } else { "FAKE_FIRST" };
            std::env::var(key).unwrap_or_else(|_| "ok".to_string())
        }
    };
    let start = Instant::now();

    match mode.as_str() {
        // 앱의 시작 관문: 이미 떠 있으면 기존 창을 앞으로 가져오고 3으로 끝난다
        "front" => {
            append(&dir.join("fronts"), &pid.to_string());
            exit(3)
        }
        "dup" => exit(3),
        // 다른 앱을 먼저 띄워 두고 중복으로 끝난다(감시가 그 앱을 넘겨받아야 한다)
        "dup-after-peer" => {
            spawn_peer(&dir);
            exit(3)
        }
        // 앱이 아닌 다른 프로세스: 잠금도 신호도 없이 닫으라고 할 때까지 산다
        "bystander" => loop {
            if dir.join(format!("close-{pid}")).exists() || start.elapsed() > MAX_LIFE {
                exit(0)
            }
            std::thread::sleep(BEAT);
        },
        _ => {}
    }

    // 앱처럼 잠금을 쥐고 PID를 쓴다. 이미 떠 있으면 중복이다.
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join(APP_LOCK))
        .expect("app.lock");
    if lock.try_lock().is_err() {
        exit(3)
    }
    atomic_write(&dir.join(APP_PID), &format!("{pid}\n")).expect("app.pid");
    let silent_warnings = std::env::var("FAKE_SILENT_WARNINGS").ok().and_then(|v| v.parse().ok()).unwrap_or(0);

    let (mut ui, mut audio) = (0u64, 1000u64);
    let mut marked = false;
    loop {
        if dir.join(format!("crash-{pid}")).exists() {
            exit(1)
        }
        if dir.join(format!("close-{pid}")).exists() {
            mark_clean_exit(&dir, pid);
            exit(0)
        }
        if start.elapsed() > MAX_LIFE {
            exit(0)
        }
        let brief_over = start.elapsed() >= BRIEF;
        let beating = match mode.as_str() {
            "crash" if brief_over => exit(1),
            "clean" if brief_over => {
                mark_clean_exit(&dir, pid);
                exit(1)
            }
            // 운영자가 닫았는데(clean_exit) 닫는 중에 멈췄다
            "close-hang" if brief_over => {
                if !marked {
                    mark_clean_exit(&dir, pid);
                    marked = true;
                }
                false
            }
            "hang" if brief_over => false,
            "silent" => false,
            _ => true,
        };
        if beating {
            ui += 1;
            if mode != "audio-stall" || dir.join("audio-ok").exists() {
                audio += 1;
            }
            let beat = Heartbeat { pid, ui_ms: ui, audio_cb_ms: audio, engine_active: true, silent_warnings };
            let _ = atomic_write(&dir.join(HEARTBEAT), &beat.render());
        }
        std::thread::sleep(BEAT);
    }
}
```

- [ ] **Step 4: 통과를 확인한다**

Run: `cargo test --manifest-path supervisor/Cargo.toml --test rules --test supervise` → 모두 통과.
Run: `cargo clippy --manifest-path supervisor/Cargo.toml --all-targets -- -D warnings` → 경고 0.
빌드·장치 테스트 전에 Global Constraints의 두 가지(다른 Atmos 앱 없음, 디스크 3GB 이상)를 확인한다. 이 시험은 가짜 앱만 띄우므로 오디오 장치는 쓰지 않는다.

- [ ] **Step 5: 체크포인트** — `git status`·`git diff --stat`. 커밋은 사용자 요청 시.

---

### Task 4: 넘겨받기(`app.lock`이 잡혀 있을 때만), 중복 종료 코드 3, 두 번째 감시

**상태:** 구현 파일·시험 있음(코드는 작업 트리의 파일을 옮겼다). Main 보고: 감시 크레이트 테스트 21개 통과(프로세스 시험 12개 = Task 3의 8개 + Task 4의 4개), clippy 경고 0(macOS·Windows 대상 `x86_64-pc-windows-msvc` 모두). PID 재사용 시험은 변이 확인을 했다(`adopt_running_app`의 잠금 확인을 빼면 실패).

**Files:**
- Modify(통째로 바꾼다): `supervisor/src/process.rs`
- Modify: `supervisor/src/run.rs`, `supervisor/Cargo.toml`
- Test: `supervisor/tests/supervise.rs`(추가)

**Interfaces:**
- `process::Target::Adopted`(unix `kill(pid, 0)`/SIGKILL, Windows `OpenProcess`·`WaitForSingleObject`·`GetExitCodeProcess`·`TerminateProcess`·`CloseHandle`), `lock_held(path)`, `adopt_running_app(dir)`.
- `run.rs`: `hand_to_running_supervisor`, `next_target`, `DUPLICATE_EXIT_CODE`(3), `DUPLICATE_RETRY_LIMIT = 3`.

- [ ] **Step 1: 실패하는 테스트를 쓴다** — `supervisor/tests/supervise.rs`에 더한 부분(스펙 7절 6·7·9, PID 재사용 Review Focus 4)

```diff
--- a/supervisor/tests/supervise.rs (Task 3 시점)
+++ b/supervisor/tests/supervise.rs (현재)
@@ -2,19 +2,13 @@
 //! 판단 시간은 짧게 준다(멈춤·첫 하트비트·오디오 멈춤 2초, 확인 0.1초, 첫 대기 0.2초).
 //! 테스트마다 폴더가 따로라 함께 돌아도 된다.
 use atmos_supervisor::heartbeat::{read_pid, APP_LOCK, APP_PID};
-
 use std::path::{Path, PathBuf};
-
 use std::process::{Child, Command, ExitStatus, Stdio};
-
 use std::sync::atomic::{AtomicBool, Ordering};
-
 use std::sync::Arc;
-
 use std::time::{Duration, Instant};
 
 const SUPERVISOR: &str = env!("CARGO_BIN_EXE_atmos_supervisor");
-
 const FAKE_APP: &str = env!("CARGO_BIN_EXE_fake_app");
 
 fn fresh_dir(tag: &str) -> PathBuf {
@@ -122,6 +116,30 @@
     std::fs::write(dir.join(format!("{what}-{pid}")), "").unwrap();
 }
 
+/// 테스트가 직접 띄운 가짜 앱(감시보다 먼저 떠 있던 앱 등). 끝나면 바로 거둬 좀비로 남지 않게 한다 —
+/// 감시는 넘겨받은 앱이 살아 있는지 PID로 본다. 돌려주는 플래그는 그 앱이 끝났는지다.
+fn spawn_fake(dir: &Path, mode: &str, env: &[(&str, &str)]) -> (u32, Arc<AtomicBool>) {
+    let mut command = Command::new(FAKE_APP);
+    command
+        .env("FAKE_DIR", dir)
+        .env("FAKE_MODE", mode)
+        .stdin(Stdio::null())
+        .stdout(Stdio::null())
+        .stderr(Stdio::null());
+    for (key, value) in env {
+        command.env(key, value);
+    }
+    let mut child = command.spawn().unwrap();
+    let pid = child.id();
+    let exited = Arc::new(AtomicBool::new(false));
+    let flag = exited.clone();
+    std::thread::spawn(move || {
+        let _ = child.wait();
+        flag.store(true, Ordering::SeqCst);
+    });
+    (pid, exited)
+}
+
 #[test]
 fn 닫는다는_표시를_남기고_끝나면_종료_코드가_1이어도_다시_띄우지_않고_감시도_끝난다() {
     let dir = fresh_dir("clean");
@@ -205,6 +223,83 @@
 }
 
 #[test]
+fn 감시보다_먼저_떠_있던_앱을_넘겨받고_그_앱이_죽으면_다시_띄운다() {
+    let dir = fresh_dir("adopt");
+    let (pid, _exited) = spawn_fake(&dir, "ok", &[("FAKE_SILENT_WARNINGS", "2")]);
+    wait_until("가짜 앱이 app.pid를 쓰지 않았다", 5, || app_pid(&dir) == Some(pid));
+    let mut sup = supervisor(&dir, &[], &[("FAKE_NEXT", "ok")]);
+    wait_until("떠 있는 앱을 넘겨받지 않았다", 10, || log_text(&dir).contains(&format!("넘겨받았다(pid {pid})")));
+    wait_until("넘겨받은 앱의 무음 경고를 기록하지 않았다", 5, || log_text(&dir).contains("무음 경고"));
+    assert_eq!(launches(&dir).len(), 1, "떠 있는 앱이 있는데 새로 띄웠다: {}", log_text(&dir));
+
+    request(&dir, "crash", pid);
+    wait_until("넘겨받은 앱이 죽었는데 다시 띄우지 않았다", 10, || launches(&dir).len() == 2);
+    assert_eq!(launch_args(&dir)[1], "--supervised --auto-relaunched");
+    wait_until("다시 띄운 앱이 app.pid를 쓰지 않았다", 5, || app_pid(&dir).is_some_and(|p| p != pid));
+    request(&dir, "close", app_pid(&dir).unwrap());
+    sup.wait_exit(10, "앱을 닫았는데 감시가 끝나지 않았다");
+    let _ = std::fs::remove_dir_all(dir);
+}
+
+#[test]
+fn 중복_실행으로_끝나면_떠_있는_앱을_넘겨받는다() {
+    let dir = fresh_dir("dup");
+    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "dup-after-peer"), ("FAKE_NEXT", "ok")]);
+    wait_until("중복 실행 뒤 떠 있는 앱을 넘겨받지 않았다", 10, || log_text(&dir).contains("넘겨받았다"));
+    let peer = app_pid(&dir).unwrap();
+    assert!(log_text(&dir).contains(&format!("넘겨받았다(pid {peer})")), "{}", log_text(&dir));
+    assert!(log_text(&dir).contains("중복 실행"), "{}", log_text(&dir));
+    std::thread::sleep(Duration::from_secs(1));
+    assert_eq!(
+        launches(&dir).len(),
+        2,
+        "중복으로 끝난 앱과 그 앱이 먼저 띄운 앱만 있어야 한다: {:?}",
+        launches(&dir)
+    );
+    request(&dir, "close", peer);
+    sup.wait_exit(10, "앱을 닫았는데 감시가 끝나지 않았다");
+    let _ = std::fs::remove_dir_all(dir);
+}
+
+#[test]
+fn 감시가_이미_돌면_두_번째는_앱을_한_번_띄워_기존_창을_앞으로_가져오고_끝난다() {
+    let dir = fresh_dir("second");
+    let mut first = supervisor(&dir, &[], &[("FAKE_FIRST", "ok")]);
+    wait_until("첫 앱이 뜨지 않았다", 10, || app_pid(&dir).is_some());
+    let mut second = supervisor(&dir, &[], &[]);
+    let status = second.wait_exit(5, "두 번째 감시가 바로 끝나지 않았다");
+    assert!(status.success(), "{status:?}");
+    wait_until("두 번째 감시가 띄운 앱이 기존 창을 앞으로 가져오지 않았다", 5, || dir.join("fronts").exists());
+    assert!(first.running(), "첫 감시가 끝났다");
+    assert_eq!(
+        launch_args(&dir),
+        vec!["--supervised".to_string(), String::new()],
+        "두 번째 감시는 앱을 인자 없이 한 번만 띄운다"
+    );
+    request(&dir, "close", app_pid(&dir).unwrap());
+    first.wait_exit(10, "앱을 닫았는데 첫 감시가 끝나지 않았다");
+    let _ = std::fs::remove_dir_all(dir);
+}
+
+#[test]
+fn 죽은_앱의_pid를_다른_프로세스가_쓰고_있으면_넘겨받지_않는다() {
+    let dir = fresh_dir("reuse");
+    // 지난 app.pid의 PID를 앱이 아닌 프로세스가 물려받았다. app.lock은 아무도 쥐고 있지 않다.
+    let (bystander, exited) = spawn_fake(&dir, "bystander", &[]);
+    std::fs::write(dir.join(APP_PID), format!("{bystander}\n")).unwrap();
+    std::fs::write(dir.join(APP_LOCK), "").unwrap();
+    let mut sup = supervisor(&dir, &[], &[("FAKE_FIRST", "ok")]);
+    wait_until("새 앱을 띄우지 않았다", 10, || app_pid(&dir).is_some_and(|p| p != bystander));
+    assert!(!log_text(&dir).contains("넘겨받았다"), "앱이 아닌 프로세스를 넘겨받았다: {}", log_text(&dir));
+    std::thread::sleep(Duration::from_secs(3)); // 넘겨받았다면 첫 하트비트 기한(2초)이 지나 죽였을 시간
+    assert!(!exited.load(Ordering::SeqCst), "앱이 아닌 프로세스가 죽었다");
+    request(&dir, "close", app_pid(&dir).unwrap());
+    sup.wait_exit(10, "앱을 닫았는데 감시가 끝나지 않았다");
+    request(&dir, "close", bystander);
+    let _ = std::fs::remove_dir_all(dir);
+}
+
+#[test]
 fn 앱_경로를_상대_경로로_줘도_띄운다() {
     let dir = fresh_dir("relative");
     // target/에서 감시를 띄우고 앱을 "debug/fake_app"처럼 준다. 감시는 앱 폴더를 작업 폴더로 삼는다.
```

- [ ] **Step 3: 구현**

`supervisor/src/process.rs`(통째로 바뀐 현재 파일):
```rust
//! 감시 대상 앱 프로세스. 감시가 띄운 앱(자식)과 감시보다 먼저 떠 있던 앱(넘겨받음)을 같은 방법으로 다룬다.
use crate::heartbeat::{read_pid, APP_LOCK, APP_PID};
use std::fs::{OpenOptions, TryLockError};
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// 감시 대상 앱.
pub enum Target {
    /// 감시가 띄운 앱.
    Child(Child),
    /// 감시보다 먼저 떠 있던 앱.
    Adopted(Adopted),
}

impl Target {
    /// 앱을 띄운다. 감시는 창 없이 돌므로 표준 입출력은 쓰지 않는다.
    pub fn spawn(app: &Path, args: &[String]) -> std::io::Result<Target> {
        // 작업 폴더를 앱 폴더로 바꾸므로 상대 경로는 먼저 절대 경로로 만든다(유닉스는 바뀐 작업 폴더에서 찾는다).
        let app = std::path::absolute(app)?;
        let mut command = Command::new(&app);
        command.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        if let Some(dir) = app.parent().filter(|d| !d.as_os_str().is_empty()) {
            command.current_dir(dir);
        }
        Ok(Target::Child(command.spawn()?))
    }

    pub fn pid(&self) -> u32 {
        match self {
            Target::Child(child) => child.id(),
            Target::Adopted(adopted) => adopted.pid,
        }
    }

    /// 끝났으면 Some(종료 코드). 종료 코드를 알 수 없으면(신호로 끝남, macOS에서 넘겨받은 앱) Some(None).
    pub fn try_exit(&mut self) -> Option<Option<i32>> {
        match self {
            Target::Child(child) => match child.try_wait() {
                Ok(Some(status)) => Some(status.code()),
                Ok(None) => None,
                Err(_) => Some(None),
            },
            Target::Adopted(adopted) => adopted.try_exit(),
        }
    }

    /// 강제 종료하고 끝날 때까지 기다린다.
    pub fn kill(&mut self) {
        match self {
            Target::Child(child) => {
                let _ = child.kill();
                let _ = child.wait();
            }
            Target::Adopted(adopted) => adopted.kill(),
        }
    }
}

/// 다른 프로세스가 [path] 잠금을 쥐고 있나. 앱은 떠 있는 동안 app.lock을 쥔다(rust core::app_signals).
/// 확인하려고 잠깐 잡았다 놓는다. 바로 그 순간 시작하는 앱은 중복으로 보고 끝날 수 있지만(종료 코드 3)
/// 감시가 그 종료를 보고 대상을 다시 정한다.
pub fn lock_held(path: &Path) -> bool {
    let Ok(file) = OpenOptions::new().read(true).write(true).open(path) else {
        return false;
    };
    matches!(file.try_lock(), Err(TryLockError::WouldBlock))
}

/// 떠 있는 앱을 넘겨받는다. app.lock이 잡혀 있고 app.pid의 프로세스가 살아 있어야 한다. 잠금을 함께 보므로
/// 앱이 죽은 뒤 그 PID를 다른 프로세스가 물려받았어도 넘겨받지 않는다(엉뚱한 프로세스를 죽이지 않게).
pub fn adopt_running_app(dir: &Path) -> Option<Target> {
    if !lock_held(&dir.join(APP_LOCK)) {
        return None;
    }
    let pid = read_pid(&dir.join(APP_PID))?;
    Adopted::open(pid).map(Target::Adopted)
}

/// 넘겨받은 앱. macOS는 PID로만 살아 있는지 본다(스펙 8절: PID 재사용을 완전히 막지 못함 — 확인용이라 받아들인다).
#[cfg(unix)]
pub struct Adopted {
    pid: u32,
}

#[cfg(unix)]
impl Adopted {
    fn open(pid: u32) -> Option<Adopted> {
        pid_alive(pid).then_some(Adopted { pid })
    }

    fn try_exit(&mut self) -> Option<Option<i32>> {
        if pid_alive(self.pid) {
            None
        } else {
            Some(None)
        }
    }

    fn kill(&mut self) {
        use std::time::{Duration, Instant};
        unsafe {
            libc::kill(self.pid as libc::pid_t, libc::SIGKILL);
        }
        let end = Instant::now() + Duration::from_secs(5);
        while pid_alive(self.pid) && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// 신호 0은 보내지 않고 대상이 있는지만 본다. 권한이 없어도(EPERM) 프로세스는 있다. PID 0은 프로세스 그룹
/// 전체를 뜻하므로 받지 않는다.
#[cfg(unix)]
fn pid_alive(pid: u32) -> bool {
    if pid == 0 || pid > i32::MAX as u32 {
        return false;
    }
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// 넘겨받은 앱. 프로세스 핸들을 쥐고 있어서, 그 프로세스가 끝나 PID가 재사용돼도 핸들은 원래 프로세스를 가리킨다.
#[cfg(windows)]
pub struct Adopted {
    pid: u32,
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl Adopted {
    fn open(pid: u32) -> Option<Adopted> {
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
        };
        if pid == 0 {
            return None;
        }
        let access = PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE;
        let handle = unsafe { OpenProcess(access, 0, pid) };
        if handle.is_null() {
            return None;
        }
        let mut adopted = Adopted { pid, handle };
        // 이미 끝났으면 넘겨받지 않는다(drop이 핸들을 닫는다).
        adopted.try_exit().is_none().then_some(adopted)
    }

    fn try_exit(&mut self) -> Option<Option<i32>> {
        use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
        use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
        if unsafe { WaitForSingleObject(self.handle, 0) } != WAIT_OBJECT_0 {
            return None;
        }
        let mut code: u32 = 0;
        let ok = unsafe { GetExitCodeProcess(self.handle, &mut code) } != 0;
        Some(ok.then_some(code as i32))
    }

    fn kill(&mut self) {
        use windows_sys::Win32::System::Threading::{TerminateProcess, WaitForSingleObject};
        unsafe {
            TerminateProcess(self.handle, 1);
            WaitForSingleObject(self.handle, 5000);
        }
    }
}

#[cfg(windows)]
impl Drop for Adopted {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}
```

`supervisor/src/run.rs`(Task 3 시점과의 차이):
```diff
--- a/supervisor/src/run.rs (Task 3 시점)
+++ b/supervisor/src/run.rs (현재)
@@ -3,7 +3,7 @@
 use crate::backoff::Backoff;
 use crate::heartbeat::{read_heartbeat, read_pid, CLEAN_EXIT, HEARTBEAT, SUPERVISOR_LOCK};
 use crate::logfile::log;
-use crate::process::Target;
+use crate::process::{adopt_running_app, Target};
 use crate::watch::{Verdict, Watch};
 use std::fs::{File, OpenOptions};
 use std::path::{Path, PathBuf};
@@ -11,6 +11,10 @@
 
 /// 오디오 멈춤으로는 연속 이 횟수까지만 다시 띄운다(스펙 5.3).
 pub const AUDIO_STALL_LIMIT: u32 = 3;
+/// 앱이 중복 실행이라 끝날 때의 종료 코드(다른 앱이 이미 떠 있다).
+pub const DUPLICATE_EXIT_CODE: i32 = 3;
+/// 중복 종료가 이만큼 이어지면(넘겨받을 앱을 못 찾음, 권한 등) 바로 다시 보지 않고 재실행 대기를 둔다.
+const DUPLICATE_RETRY_LIMIT: u32 = 3;
 
 #[cfg(windows)]
 const DEFAULT_APP: &str = "atmos_mixer_pro.exe";
@@ -117,13 +121,15 @@
     audio_gave_up_logged: bool,
     /// 마지막으로 본 무음 경고 누적 횟수.
     silent_warnings: u32,
+    /// 중복 실행(종료 코드 3)으로 연달아 끝난 횟수.
+    duplicates: u32,
 }
 
 /// 감시를 돌린다. 운영자가 앱을 닫았거나 감시가 이미 돌고 있으면 돌아온다.
 pub fn run(cfg: &Config) {
     let _ = std::fs::create_dir_all(&cfg.dir);
     let Some(_lock) = acquire_lock(&cfg.dir.join(SUPERVISOR_LOCK)) else {
-        log(&cfg.dir, "감시가 이미 돌고 있다 — 끝낸다");
+        hand_to_running_supervisor(cfg);
         return;
     };
     log(
@@ -145,8 +151,23 @@
         audio_relaunches: 0,
         audio_gave_up_logged: false,
         silent_warnings: 0,
+        duplicates: 0,
     }
     .run_loop();
+}
+
+/// 감시가 이미 돈다(바로가기를 또 눌렀다). 앱을 인자 없이 한 번 띄우면 앱의 시작 관문이 기존 창을 앞으로
+/// 가져오고 끝난다. 앱이 내려가 있던 사이(재실행 대기 중)라면 앱이 바로 뜨고, 돌고 있는 감시가 넘겨받는다.
+/// 띄운 앱은 기다리지 않는다 — 감시가 곧 끝나므로 init이 거둔다.
+#[allow(clippy::zombie_processes)]
+fn hand_to_running_supervisor(cfg: &Config) {
+    match Target::spawn(&cfg.app, &[]) {
+        Ok(target) => log(
+            &cfg.dir,
+            &format!("감시가 이미 돌고 있다 — 앱을 한 번 띄워(pid {}) 기존 창을 앞으로 가져오고 끝낸다", target.pid()),
+        ),
+        Err(e) => log(&cfg.dir, &format!("감시가 이미 돌고 있다 — 앱을 띄우지 못했다: {e}")),
+    }
 }
 
 /// [path]를 열어 잠근다. 다른 프로세스가 쥐고 있으면 None.
@@ -169,6 +190,17 @@
                 Ending::Exited(code) if self.closed_by_operator(&target) => {
                     self.log(&format!("운영자가 앱을 닫았다(pid {}, 종료 코드 {code:?}) — 감시를 끝낸다", target.pid()));
                     return;
+                }
+                Ending::Exited(Some(DUPLICATE_EXIT_CODE)) => {
+                    // 다른 앱이 이미 떠 있다. 바로 대상을 다시 정한다(넘겨받기).
+                    self.duplicates += 1;
+                    if self.duplicates < DUPLICATE_RETRY_LIMIT {
+                        self.log("앱이 중복 실행으로 끝났다(종료 코드 3) — 떠 있는 앱을 넘겨받는다");
+                        continue;
+                    }
+                    self.log("앱이 중복 실행으로 끝나는데 넘겨받을 앱이 보이지 않는다 — 기다렸다 다시 본다");
+                    self.wait_before_relaunch(Duration::ZERO);
+                    continue;
                 }
                 Ending::Exited(code) => {
                     self.log(&format!(
@@ -192,13 +224,19 @@
                     self.log(&format!("{} — 앱을 강제 종료했다(pid {})", self.describe(verdict), target.pid()));
                 }
             }
+            self.duplicates = 0;
             self.relaunch = true;
             self.wait_before_relaunch(started.elapsed());
         }
     }
 
-    /// 다음 감시 대상: (대상, 넘겨받았나).
+    /// 다음 감시 대상: (대상, 넘겨받았나). 떠 있는 앱이 있으면 넘겨받고, 없으면 띄운다.
     fn next_target(&mut self) -> Option<(Target, bool)> {
+        if let Some(target) = adopt_running_app(&self.cfg.dir) {
+            self.log(&format!("떠 있는 앱을 넘겨받았다(pid {})", target.pid()));
+            self.duplicates = 0;
+            return Some((target, true));
+        }
         self.launch().map(|target| (target, false))
     }
 
```

`supervisor/Cargo.toml`(Task 1 시점과의 차이):
```diff
--- a/supervisor/Cargo.toml (Task 3 시점)
+++ b/supervisor/Cargo.toml (현재)
@@ -7,3 +7,10 @@
 [dependencies]
 # 감시 기록 시각을 앱 로그(chrono::Local)와 같은 현지 시각으로 남긴다.
 chrono = { version = "0.4.45", default-features = false, features = ["clock"] }
+
+# 넘겨받은 앱(감시가 띄우지 않은 프로세스)이 살아 있는지 보고 강제 종료한다.
+[target.'cfg(unix)'.dependencies]
+libc = "0.2"
+
+[target.'cfg(windows)'.dependencies]
+windows-sys = { version = "0.61", features = ["Win32_Foundation", "Win32_System_Threading"] }
```

`supervisor/src/bin/fake_app.rs`(Task 3 시점과의 차이):
(바뀌지 않았다)

`supervisor/src/main.rs`·`lib.rs`: 바뀌지 않았다.

- [ ] **Step 2: 실패를 확인한다**(구현 전)

  Run: `cargo test --manifest-path supervisor/Cargo.toml --test supervise`
  Expected: 넘겨받기·중복 관련 시험이 없는 기능이라 실패(또는 컴파일 실패).

- [ ] **Step 4: 통과를 확인한다**

  Run: `cargo test --manifest-path supervisor/Cargo.toml` → 모두 통과.
  Run: `cargo clippy --manifest-path supervisor/Cargo.toml --all-targets -- -D warnings` → 경고 0.
  Run: `cargo clippy --manifest-path supervisor/Cargo.toml --all-targets --target x86_64-pc-windows-msvc -- -D warnings` → 경고 0(Main이 확인함, 이 Mac에 타깃이 설치돼 있다). 앱 크레이트는 Windows 크로스 체크가 안 된다(HANDOFF "Windows 미검증").
  변이 확인: `adopt_running_app`의 `app.lock` 확인을 빼면 PID 재사용 시험이 실패한다.

- [ ] **Step 5: 체크포인트** — `git status`·`git diff --stat`로 이 작업의 파일만 바뀌었는지 본다. 커밋은 사용자 요청 시.

---

### Task 5: 첫 방 테마 시작(`api::show::api_theme_start`) + OSC `ThemeStart`를 단위 변형으로

**상태:** 구현 파일·시험 있음(코드는 작업 트리의 파일을 옮겼다). Main 보고: 앱 Rust 전체 `cargo test` 78개 바이너리 통과, 새 clippy 경고 없음(`listener.rs` 219·312행 `get(0)` 경고는 기존).

**Files:**
- Create: `rust/src/api/show.rs`(Task 8에서 `api_start_show_state`·`api_resume_show`가 같은 파일에 더해진다), `rust/tests/test_theme_start.rs`
- Modify: `rust/src/api/mod.rs`, `rust/src/osc/router.rs`, `rust/src/osc/listener.rs`
- Test: `rust/tests/test_osc_room_routing.rs`(6단계 `/theme/start` 추가)

**Interfaces:** `api::show::api_theme_start() -> Result<(), AtmosError>`: 전체 정지 → 설정의 첫 방을 활성으로 → 그 방 루프 재생(실패는 기록). OSC `ThemeStart`는 단위 변형이 되어 이 함수를 부른다.

- [ ] **Step 1: 실패하는 테스트를 쓴다**

`rust/tests/test_theme_start.rs`:
```rust
//! 첫 방 테마 시작(api::show::api_theme_start). 감시가 다시 띄운 앱이 이어 갈 수 없을 때와 OSC 테마 시작이
//! 이 함수를 쓴다. 전역 상태를 쓰므로 테스트는 하나다.
use rust_lib_atmos_mixer_pro::api::show::api_theme_start;
use rust_lib_atmos_mixer_pro::api::simple::{api_play_track, api_set_active_room};
use rust_lib_atmos_mixer_pro::audio::player::SoundData;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

fn tone_wav(path: &std::path::Path, seconds: f32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..(48_000.0 * seconds) as usize {
        let v = (i as f32 * 200.0 * std::f32::consts::TAU / 48_000.0).sin() * 3000.0;
        w.write_sample(v as i16).unwrap();
    }
    w.finalize().unwrap();
}

fn track(id: &str, path: &str, is_loop: bool) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: path.into(),
        volume: 1.0,
        is_loop,
        is_streaming: false,
        output_channel: 1,
        output_stereo: false,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
    }
}

fn room(id: &str, tracks: Vec<TrackConfig>) -> RoomConfig {
    RoomConfig {
        id: id.into(),
        name: id.into(),
        color_hex: "#ffffff".into(),
        volume: 1.0,
        volume_osc_address: String::new(),
        clear_osc_address: String::new(),
        tracks,
    }
}

fn playing() -> Vec<String> {
    let mut v: Vec<String> = GLOBAL_STATE.playing_track_ids.read().unwrap().values().cloned().collect();
    v.sort();
    v
}

fn drain() -> Vec<AudioCommand> {
    let mut v = Vec::new();
    while let Ok(c) = GLOBAL_STATE.command_receiver.try_recv() {
        v.push(c);
    }
    v
}

#[test]
fn 테마_시작은_전체_정지_뒤_첫_방을_활성으로_하고_그_방_루프만_튼다() {
    let dir = std::env::temp_dir().join(format!("atmos_theme_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["la", "lb"] {
        tone_wav(&dir.join(format!("{name}.wav")), 3.0);
    }
    let p = |n: &str| dir.join(format!("{n}.wav")).to_string_lossy().into_owned();
    GLOBAL_STATE.sound_cache.write().unwrap().insert(
        "/oa.wav".into(),
        Arc::new(SoundData { samples: vec![0.1; 48_000], channels: 1, sample_rate: 48_000 }),
    );
    GLOBAL_STATE.engine_sample_rate.store(48_000, Ordering::SeqCst);
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig {
        rooms: vec![
            room("a", vec![track("la", &p("la"), true), track("oa", "/oa.wav", false)]),
            room("b", vec![track("lb", &p("lb"), true)]),
        ],
        ..AppConfig::default()
    });
    GLOBAL_STATE.clear_playing_tracks();

    // 방 B BGM과 방 A 단발이 도는 중이다
    api_set_active_room(Some("b".into())).unwrap();
    api_play_track("b".into(), "lb".into()).unwrap();
    api_play_track("a".into(), "oa".into()).unwrap();
    drain();

    api_theme_start().unwrap();
    assert_eq!(*GLOBAL_STATE.active_room_id.read().unwrap(), Some("a".to_string()));
    assert_eq!(playing(), vec!["la".to_string()], "첫 방 루프만 재생 목록에 남아야 한다");
    let cmds = drain();
    let stop_all_at = cmds.iter().position(|c| matches!(c, AudioCommand::StopAll)).expect("전체 정지 명령이 없다");
    let plays: Vec<(usize, String)> = cmds
        .iter()
        .enumerate()
        .filter_map(|(i, c)| match c {
            AudioCommand::PlayTrack { instance, .. } => Some((i, instance.track_id_str.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(plays.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>(), vec!["la"]);
    assert!(plays[0].0 > stop_all_at, "재생 명령이 전체 정지보다 먼저 나갔다");

    // 방이 없으면 정지만 한다
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig::default());
    api_theme_start().unwrap();
    assert!(playing().is_empty());
    assert_eq!(*GLOBAL_STATE.active_room_id.read().unwrap(), None);

    drain();
    let _ = std::fs::remove_dir_all(&dir);
}
```

`rust/tests/test_osc_room_routing.rs`(변경분 — 이 단계는 리팩터링 전에도 통과해야 한다):
```diff
--- a/atmos_mixer_pro/rust/tests/test_osc_room_routing.rs
+++ b/atmos_mixer_pro/rust/tests/test_osc_room_routing.rs
@@ -1,6 +1,7 @@
 //! OSC 재생·방 비우기 경로. 실제 OSC 리스너에 UDP로 보낸다. 전역 상태를 쓰므로 테스트는 하나다.
 //! - OSC 재생은 대시보드와 같은 경로라 루프·스트리밍 트랙도 나온다.
 //! - 방 비우기(OSC·대시보드)는 그 방 트랙만 재생 목록에서 빼고, OSC로 다음 방에 넘어가면 그 방 BGM(루프)을 튼다.
+//! - OSC 테마 시작은 전체 정지 뒤 첫 방을 활성으로 하고 그 방 루프만 튼다(감시 재실행과 같은 함수).
 use rosc::{encoder, OscMessage, OscPacket, OscType};
 use rust_lib_atmos_mixer_pro::api::simple::{
     api_clear_room, api_play_track, api_set_active_room, api_start_osc_listener,
@@ -105,6 +106,7 @@ fn osc_재생은_루프와_스트리밍도_틀고_방_비우기는_그_방만_
     let config = AppConfig {
         osc_port: port,
         is_exhibition_mode: true,
+        theme_start_osc_address: "/theme/start".into(),
         rooms: vec![
             room(
                 "a",
@@ -165,6 +167,12 @@ fn osc_재생은_루프와_스트리밍도_틀고_방_비우기는_그_방만_
     send("/a/clear");
     wait_until("OSC로 방 A를 비웠는데 다음 방 B의 BGM이 자동으로 나오지 않았다", || has("lb"));
 
+    // 6) OSC 테마 시작: 전체 정지 뒤 첫 방(A)을 활성으로 하고 그 방 루프만 튼다
+    send("/theme/start");
+    wait_until("OSC 테마 시작 뒤 첫 방 A가 활성이고 그 방 루프만 돌아야 한다", || {
+        GLOBAL_STATE.active_room_id.read().unwrap().as_deref() == Some("a") && playing() == vec!["la".to_string()]
+    });
+
     drained_plays();
     let _ = std::fs::remove_dir_all(&dir);
 }
```

- [ ] **Step 3: 구현**

`rust/src/api/show.rs`의 이 작업 부분(파일 머리의 `use`는 아래와 같다. 나머지 두 함수는 Task 8):
```rust
use crate::api::error::AtmosError;
use crate::api::simple::{api_play_track, api_set_active_room, api_stop_all};
use crate::core::state::GLOBAL_STATE;

/// 첫 방 테마 시작: 전체 정지 → 설정의 첫 방을 활성으로 → 그 방 루프 재생. OSC 테마 시작과 같다.
pub fn api_theme_start() -> Result<(), AtmosError> {
    api_stop_all()?;
    let first_room = GLOBAL_STATE
        .config
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|config| config.rooms.first())
        .map(|room| {
            let loops: Vec<String> = room.tracks.iter().filter(|t| t.is_loop).map(|t| t.id.clone()).collect();
            (room.id.clone(), loops)
        });
    let Some((room_id, loop_ids)) = first_room else {
        return Ok(());
    };
    api_set_active_room(Some(room_id.clone()))?;
    for track_id in loop_ids {
        if let Err(e) = api_play_track(room_id.clone(), track_id.clone()) {
            GLOBAL_STATE.log(format!("테마 시작: {track_id} 재생 실패: {}", e.message));
        }
    }
    Ok(())
}
```

`rust/src/osc/router.rs`:
```diff
--- a/atmos_mixer_pro/rust/src/osc/router.rs
+++ b/atmos_mixer_pro/rust/src/osc/router.rs
@@ -9,7 +9,7 @@ pub enum OscAction {
     SetMasterVolume(String),
     PlayTrack(String, String),
     StopTrack(String, String),
-    ThemeStart(String, Vec<String>),
+    ThemeStart,
     SystemReset,
 }
 
@@ -30,13 +30,7 @@ pub fn get_osc_action(addr: &str, config: &AppConfig, config_version: u64) -> Op
     let mut new_map = HashMap::new();
     
     if !config.theme_start_osc_address.is_empty() {
-        let mut track_ids = vec![];
-        let mut first_room_id = String::new();
-        if let Some(first_room) = config.rooms.first() {
-            first_room_id = first_room.id.clone();
-            track_ids = first_room.tracks.iter().filter(|t| t.is_loop).map(|t| t.id.clone()).collect();
-        }
-        new_map.insert(config.theme_start_osc_address.clone(), OscAction::ThemeStart(first_room_id, track_ids));
+        new_map.insert(config.theme_start_osc_address.clone(), OscAction::ThemeStart);
     }
     
     if !config.system_reset_osc_address.is_empty() {
```

`rust/src/osc/listener.rs`:
```diff
--- a/atmos_mixer_pro/rust/src/osc/listener.rs
+++ b/atmos_mixer_pro/rust/src/osc/listener.rs
@@ -256,18 +256,9 @@ fn handle_packet(packet: OscPacket, debouncer: &OscDebouncer) {
                         crate::core::state::GLOBAL_STATE
                             .log("OSC Triggered: System Reset".to_string());
                     }
-                    OscAction::ThemeStart(first_room, track_ids) => {
-                        let _ = crate::api::simple::api_stop_all();
-                        if !first_room.is_empty() {
-                            let _ =
-                                crate::api::simple::api_set_active_room(Some(first_room.clone()));
-                            for track_id in track_ids {
-                                let _ = crate::api::simple::api_play_track(
-                                    first_room.clone(),
-                                    track_id,
-                                );
-                            }
-                        }
+                    OscAction::ThemeStart => {
+                        // 감시가 다시 띄운 앱이 이어 갈 수 없을 때와 같은 동작이다(api::show).
+                        let _ = crate::api::show::api_theme_start();
                     }
                     OscAction::ClearRoom(room_id) => {
                         if !check_gating(&room_id, config.is_exhibition_mode) {
```

`rust/src/api/mod.rs`(최종 파일. `lifecycle`은 Task 7):
```rust
pub mod error;
pub mod simple;
pub mod osc;
pub mod scene;
pub mod acoustics;
pub mod lifecycle;
pub mod show;
```

`rust/src/api/`를 바꿨으므로 FRB codegen이 필요하다. Task 5·7·8의 API 변경을 모아 Task 10에서 한 번 돌렸다.

- [ ] **Step 2: 실패를 확인한다**(구현 전)

  Run: `cargo test --test test_theme_start --test test_osc_room_routing`
  Expected: `api_theme_start`가 없어 컴파일 실패. 6단계는 변경 전에도 통과한다.

- [ ] **Step 4: 통과를 확인한다**

  Run: `cargo test --test test_theme_start --test test_osc_room_routing` → 모두 통과.
  Run(`rust/`에서): `cargo clippy` → 이 작업이 바꾼 줄에 새 경고 0(기존 경고는 그대로).
  빌드·장치 테스트 전에 Global Constraints의 두 가지(다른 Atmos 앱 없음, 디스크 3GB 이상)를 확인한다.

- [ ] **Step 5: 체크포인트** — `git status`·`git diff --stat`로 이 작업의 파일만 바뀌었는지 본다. 커밋은 사용자 요청 시.

---

### Task 6: 앱 로그 파일(감시 기록 내보내기, 회전 실패해도 줄을 버리지 않음)

**상태:** 구현 파일·시험 있음(코드는 작업 트리의 파일을 옮겼다). Main 보고: 앱 Rust 전체 `cargo test` 78개 바이너리 통과, 새 clippy 경고 없음(`listener.rs` 219·312행 `get(0)` 경고는 기존). 회전 규칙은 Sub 제안 반영(스펙 5.4).

**Files:**
- Modify: `rust/src/core/log_file.rs`
- Test: `rust/tests/test_log_rotation.rs`(시험 2개 추가)

**Interfaces:** `SUPERVISOR_LOG_FILE_NAME`. `export_to`가 `supervisor.log`(밀린 것 포함)도 복사한다. `append_line`·`rotate`는 지금 파일을 `<이름>.rotating`으로 먼저 옮겨 보고, 실패하면 아무것도 건드리지 않고 줄을 지금 파일에 쓴다.

- [ ] **Step 1: 실패하는 테스트를 쓴다** — `rust/tests/test_log_rotation.rs`(변경분)

```diff
--- a/atmos_mixer_pro/rust/tests/test_log_rotation.rs
+++ b/atmos_mixer_pro/rust/tests/test_log_rotation.rs
@@ -1,6 +1,6 @@
 //! 앱 로그 파일 회전. 크기를 넘으면 뒤로 밀고(.1, .2 …) 정한 개수만 남긴다. 내보내기는 밀린 파일까지 복사한다.
 //! Windows의 %TEMP%는 저절로 비워지지 않아, 회전이 없으면 무인 운영에서 로그가 끝없이 커진다.
-use rust_lib_atmos_mixer_pro::core::log_file::{append_line, export_to, LOG_FILE_NAME};
+use rust_lib_atmos_mixer_pro::core::log_file::{append_line, export_to, LOG_FILE_NAME, SUPERVISOR_LOG_FILE_NAME};
 use std::path::{Path, PathBuf};
 
 fn fresh_dir(tag: &str) -> PathBuf {
@@ -54,3 +54,41 @@ fn 크기를_넘으면_뒤로_밀고_정한_개수만_남기며_내보내기는_
         let _ = std::fs::remove_dir_all(d);
     }
 }
+
+#[test]
+fn 로그_내보내기는_감시_기록도_함께_복사한다() {
+    let dir = fresh_dir("export_sup_src");
+    std::fs::create_dir_all(&dir).unwrap();
+    append_line(&dir, "app", 1024, 2).unwrap();
+    std::fs::write(dir.join(SUPERVISOR_LOG_FILE_NAME), "sup\n").unwrap();
+    std::fs::write(dir.join(format!("{SUPERVISOR_LOG_FILE_NAME}.1")), "sup old\n").unwrap();
+    let dest = fresh_dir("export_sup_dest");
+    std::fs::create_dir_all(&dest).unwrap();
+    assert_eq!(export_to(&dir, &dest).unwrap(), 3, "앱 로그 + 감시 기록 2개");
+    assert_eq!(names(&dest), names(&dir));
+    for d in [dir, dest] {
+        let _ = std::fs::remove_dir_all(d);
+    }
+}
+
+#[test]
+fn 지금_로그를_옮기지_못하면_밀지_않고_그_줄은_지금_파일에_남긴다() {
+    let dir = fresh_dir("stuck");
+    std::fs::create_dir_all(&dir).unwrap();
+    std::fs::write(dir.join(format!("{LOG_FILE_NAME}.1")), "old\n").unwrap();
+    // 옮길 자리에 비어 있지 않은 폴더가 있어 지금 파일의 이름 바꾸기가 실패한다.
+    // Windows에서 다른 프로그램이 로그를 쥐고 있을 때와 같다.
+    std::fs::create_dir_all(dir.join(format!("{LOG_FILE_NAME}.rotating")).join("x")).unwrap();
+    for i in 0..10 {
+        append_line(&dir, &format!("line-{i:04}"), 30, 2).unwrap();
+    }
+    let current = std::fs::read_to_string(dir.join(LOG_FILE_NAME)).unwrap();
+    assert_eq!(current.lines().count(), 10, "밀지 못해도 줄을 버리면 안 된다: {current:?}");
+    assert_eq!(
+        std::fs::read_to_string(dir.join(format!("{LOG_FILE_NAME}.1"))).unwrap(),
+        "old\n",
+        "밀지 못했는데 오래된 로그가 바뀌었다"
+    );
+    assert!(!dir.join(format!("{LOG_FILE_NAME}.2")).exists(), "밀지 못했는데 오래된 로그를 옮겼다");
+    let _ = std::fs::remove_dir_all(dir);
+}
```

- [ ] **Step 3: 구현** — `rust/src/core/log_file.rs`(변경분)

```diff
--- a/atmos_mixer_pro/rust/src/core/log_file.rs
+++ b/atmos_mixer_pro/rust/src/core/log_file.rs
@@ -6,6 +6,8 @@ use std::path::{Path, PathBuf};
 use std::sync::Mutex;
 
 pub const LOG_FILE_NAME: &str = "atmos_mixer_pro.log";
+/// 감시 프로그램(supervisor/)이 같은 폴더에 남기는 기록. 내보낼 때 함께 복사한다.
+pub const SUPERVISOR_LOG_FILE_NAME: &str = "supervisor.log";
 /// 파일 하나의 최대 크기. 넘으면 다음 줄을 쓰기 전에 민다.
 pub const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;
 /// 밀린 파일을 몇 개까지 남기나. 지금 파일까지 최대 5개(약 50MB).
@@ -23,43 +25,51 @@ fn rotated(dir: &Path, n: usize) -> PathBuf {
     dir.join(format!("{LOG_FILE_NAME}.{n}"))
 }
 
-/// [dir]의 로그 파일에 한 줄을 덧붙인다. 지금 파일이 [max_bytes] 이상이면 먼저 민다.
+/// [dir]의 로그 파일에 한 줄을 덧붙인다. 지금 파일이 [max_bytes] 이상이면 먼저 민다. 밀기에 실패해도
+/// (Windows에서 다른 프로그램이 로그를 쥐고 있으면 이름 바꾸기가 실패한다) 그 줄은 지금 파일에 남긴다.
 pub fn append_line(dir: &Path, line: &str, max_bytes: u64, keep: usize) -> std::io::Result<()> {
     let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
     std::fs::create_dir_all(dir)?;
     let current = dir.join(LOG_FILE_NAME);
     if std::fs::metadata(&current).map(|m| m.len() >= max_bytes).unwrap_or(false) {
-        rotate(dir, keep)?;
+        let _ = rotate(dir, keep);
     }
     let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&current)?;
     writeln!(file, "{line}")
 }
 
-/// 가장 오래된 파일을 지우고 나머지를 한 칸씩 민다(지금 파일 → .1).
+/// 가장 오래된 파일을 지우고 나머지를 한 칸씩 민다(지금 파일 → .1). 지금 파일을 옆 이름으로 먼저 옮겨 보고,
+/// 그게 안 되면 아무것도 건드리지 않는다 — 예전에는 오래된 파일부터 밀다가 지금 파일에서 실패해 그 줄을 버렸고,
+/// 다음 줄마다 다시 밀면서 오래된 로그를 하나씩 지웠다.
 fn rotate(dir: &Path, keep: usize) -> std::io::Result<()> {
     let current = dir.join(LOG_FILE_NAME);
     if keep == 0 {
         return std::fs::remove_file(current);
     }
+    let rotating = dir.join(format!("{LOG_FILE_NAME}.rotating"));
+    std::fs::rename(&current, &rotating)?;
     let _ = std::fs::remove_file(rotated(dir, keep));
     for n in (1..keep).rev() {
         let from = rotated(dir, n);
         if from.exists() {
-            std::fs::rename(&from, rotated(dir, n + 1))?;
+            let _ = std::fs::rename(&from, rotated(dir, n + 1));
         }
     }
-    std::fs::rename(current, rotated(dir, 1))
+    std::fs::rename(rotating, rotated(dir, 1))
 }
 
-/// 로그 파일(지금 것과 밀린 것)을 [dest] 폴더로 복사하고 복사한 개수를 돌려준다.
+/// 로그 파일(앱 로그와 감시 기록, 지금 것과 밀린 것)을 [dest] 폴더로 복사하고 복사한 개수를 돌려준다.
 pub fn export_to(dir: &Path, dest: &Path) -> std::io::Result<usize> {
     let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
     let mut copied = 0;
-    let files = std::iter::once(dir.join(LOG_FILE_NAME)).chain((1..=KEEP_ROTATED).map(|n| rotated(dir, n)));
-    for path in files {
-        if let (true, Some(name)) = (path.exists(), path.file_name()) {
-            std::fs::copy(&path, dest.join(name))?;
-            copied += 1;
+    for name in [LOG_FILE_NAME, SUPERVISOR_LOG_FILE_NAME] {
+        let files = std::iter::once(dir.join(name))
+            .chain((1..=KEEP_ROTATED).map(|n| dir.join(format!("{name}.{n}"))));
+        for path in files {
+            if let (true, Some(file_name)) = (path.exists(), path.file_name()) {
+                std::fs::copy(&path, dest.join(file_name))?;
+                copied += 1;
+            }
         }
     }
     Ok(copied)
```

- [ ] **Step 2: 실패를 확인한다**(구현 전)

  Run: `cargo test --test test_log_rotation`
  Expected: 내보내기에 `supervisor.log`가 없어 실패, 회전 실패 시험은 줄이 버려져 실패.

- [ ] **Step 4: 통과를 확인한다**

  Run: `cargo test --test test_log_rotation` → 모두 통과.
  Run(`rust/`에서): `cargo clippy` → 이 작업이 바꾼 줄에 새 경고 0(기존 경고는 그대로).

- [ ] **Step 5: 체크포인트** — `git status`·`git diff --stat`로 이 작업의 파일만 바뀌었는지 본다. 커밋은 사용자 요청 시.

---

### Task 7: 신호와 시작 관문(`core::app_signals`, `api::lifecycle`)

**상태:** 구현 파일·시험 있음(코드는 작업 트리의 파일을 옮겼다). Main 보고: 앱 Rust 전체 `cargo test` 78개 바이너리 통과, 새 clippy 경고 없음(`listener.rs` 219·312행 `get(0)` 경고는 기존).

**Files:**
- Create: `rust/src/core/app_signals.rs`, `rust/src/api/lifecycle.rs`, `rust/tests/test_app_signals.rs`
- Modify: `rust/src/core/mod.rs`, `rust/src/api/mod.rs`(Task 5의 파일 참고), `rust/Cargo.toml`(windows 기능 `Win32_UI_WindowsAndMessaging`)

- [ ] **Step 1: 실패하는 테스트를 쓴다** — `rust/tests/test_app_signals.rs`

```rust
//! 앱이 감시 프로그램에 남기는 신호와 시작 관문(core::app_signals). 실제 로그 폴더 대신 임시 폴더를 쓴다.
use rust_lib_atmos_mixer_pro::api::lifecycle::StartupDecision;
use rust_lib_atmos_mixer_pro::core::app_signals::{
    current_heartbeat, mark_clean_exit, startup_gate, write_heartbeat, Gate, APP_LOCK, APP_PID, CLEAN_EXIT,
    HEARTBEAT, SILENT_WARNINGS, SUPERVISOR_LOCK,
};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("atmos_signals_test_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 다른 프로세스처럼 [path] 잠금을 쥔다(같은 프로세스라도 따로 연 파일끼리는 잠금이 부딪힌다).
fn hold_lock(path: &Path) -> File {
    let file = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(path).unwrap();
    file.try_lock().unwrap();
    file
}

/// 관문을 한 번 돌린 결과
struct GateRun {
    decision: StartupDecision,
    lock: Option<File>,
    launched: Vec<PathBuf>,
    fronted: Vec<u32>,
}

fn run_gate(dir: &Path, supervisor_exe: Option<&Path>, argv: &[&str]) -> GateRun {
    let launched = RefCell::new(Vec::new());
    let fronted = RefCell::new(Vec::new());
    let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    let (decision, lock) = {
        let launch = |exe: &Path| -> std::io::Result<()> {
            launched.borrow_mut().push(exe.to_path_buf());
            Ok(())
        };
        let front = |pid: u32| fronted.borrow_mut().push(pid);
        let gate = Gate { dir, supervisor_exe, pid: 4242, launch_supervisor: &launch, bring_to_front: &front };
        startup_gate(&args, &gate)
    };
    GateRun { decision, lock, launched: launched.into_inner(), fronted: fronted.into_inner() }
}

#[test]
fn 직접_실행했고_감시가_옆에_있고_돌지_않으면_감시로_넘기고_잠금을_놓는다() {
    let dir = fresh_dir("handoff");
    let exe = dir.join("atmos_supervisor");
    let run = run_gate(&dir, Some(&exe), &[]);
    assert_eq!(run.decision, StartupDecision::HandedToSupervisor);
    assert_eq!(run.launched, vec![exe]);
    assert!(run.lock.is_none());
    assert!(!dir.join(APP_PID).exists(), "넘기는 앱이 app.pid를 쓰면 감시가 끝나 가는 앱을 넘겨받는다");
    drop(hold_lock(&dir.join(APP_LOCK))); // 감시가 띄울 앱이 잠금을 잡을 수 있다
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 감시가_없거나_감시가_띄웠거나_감시가_돌면_진행하고_잠금과_pid를_쥔다() {
    // 감시 exe가 없다(개발 빌드·macOS)
    let dir = fresh_dir("proceed");
    std::fs::write(dir.join(CLEAN_EXIT), "pid=1\n").unwrap();
    let run = run_gate(&dir, None, &[]);
    assert_eq!(run.decision, StartupDecision::Proceed);
    assert!(run.lock.is_some(), "app.lock을 쥐어야 한다");
    assert!(run.launched.is_empty());
    assert_eq!(std::fs::read_to_string(dir.join(APP_PID)).unwrap().trim(), "4242");
    assert!(!dir.join(CLEAN_EXIT).exists(), "지난 clean_exit이 남아 있다");
    drop(run.lock);

    // 감시가 띄웠다(--supervised): 감시 exe가 있어도 넘기지 않는다
    let exe = dir.join("atmos_supervisor");
    let run = run_gate(&dir, Some(&exe), &["--supervised", "--auto-relaunched"]);
    assert_eq!(run.decision, StartupDecision::Proceed);
    assert!(run.launched.is_empty());
    drop(run.lock);

    // 감시가 이미 돈다(supervisor.lock이 잡혀 있다): 직접 실행이어도 넘기지 않는다(그 감시가 넘겨받는다)
    let supervisor = hold_lock(&dir.join(SUPERVISOR_LOCK));
    let run = run_gate(&dir, Some(&exe), &[]);
    assert_eq!(run.decision, StartupDecision::Proceed);
    assert!(run.launched.is_empty());
    drop(run.lock);
    drop(supervisor);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 앱이_이미_떠_있으면_그_창을_앞으로_가져오고_중복으로_끝난다() {
    let dir = fresh_dir("dup");
    let first = run_gate(&dir, None, &["--supervised"]);
    assert_eq!(first.decision, StartupDecision::Proceed);
    std::fs::write(dir.join(APP_PID), "777\n").unwrap(); // 떠 있는 앱의 PID

    let second = run_gate(&dir, None, &[]);
    assert_eq!(second.decision, StartupDecision::Duplicate);
    assert_eq!(second.fronted, vec![777]);
    assert!(second.launched.is_empty(), "감시 exe가 없으면 감시를 띄우지 않는다");
    assert_eq!(std::fs::read_to_string(dir.join(APP_PID)).unwrap().trim(), "777", "중복 실행이 app.pid를 덮어썼다");

    // 감시 없이 떠 있는 앱 + 감시 exe 있음: 감시를 띄워 그 앱을 넘겨받게 한다
    let exe = dir.join("atmos_supervisor");
    let third = run_gate(&dir, Some(&exe), &[]);
    assert_eq!(third.decision, StartupDecision::Duplicate);
    assert_eq!(third.launched, vec![exe]);
    drop(first.lock);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 하트비트에는_다섯_값이_들어가고_부를_때마다_ui_ms가_커진다() {
    GLOBAL_STATE.watchdog_last_callback.store(777, Ordering::Relaxed);
    SILENT_WARNINGS.store(2, Ordering::Relaxed);
    let a = current_heartbeat();
    let b = current_heartbeat();
    assert_eq!(a.pid, std::process::id());
    assert_eq!(a.audio_cb_ms, 777);
    assert!(!a.engine_active, "테스트에는 엔진이 없다");
    assert_eq!(a.silent_warnings, 2);
    assert!(b.ui_ms > a.ui_ms, "같은 밀리초에 불러도 ui_ms가 바뀌어야 감시가 응답으로 본다");

    let dir = fresh_dir("hb");
    write_heartbeat(&dir, &b).unwrap();
    let text = std::fs::read_to_string(dir.join(HEARTBEAT)).unwrap();
    let kv: HashMap<&str, &str> = text.lines().filter_map(|l| l.split_once('=')).collect();
    assert_eq!(kv.len(), 5, "감시가 읽는 키는 다섯 개다: {text}");
    assert_eq!(kv["pid"], b.pid.to_string());
    assert_eq!(kv["ui_ms"], b.ui_ms.to_string());
    assert_eq!(kv["audio_cb_ms"], "777");
    assert_eq!(kv["engine_active"], "0");
    assert_eq!(kv["silent_warnings"], "2");
    assert!(!dir.join(format!("{HEARTBEAT}.tmp")).exists(), "임시 파일이 남았다");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 정상_종료_표시는_닫는_앱의_pid다() {
    let dir = fresh_dir("clean");
    mark_clean_exit(&dir, 4242).unwrap();
    assert_eq!(std::fs::read_to_string(dir.join(CLEAN_EXIT)).unwrap(), "pid=4242\n");
    let _ = std::fs::remove_dir_all(dir);
}
```

- [ ] **Step 3: 구현**

`rust/src/core/app_signals.rs`:
```rust
//! 감시 프로그램(supervisor/)이 읽는 신호 파일과 시작 관문(스펙 4.2·4.3).
//! 파일 이름과 하트비트 형식은 감시 크레이트(supervisor/src/heartbeat.rs)와 같아야 한다.
//! 오디오 스레드는 건드리지 않는다 — 이미 있는 원자 값을 읽기만 한다(DSP 3법칙).
use crate::api::lifecycle::StartupDecision;
use crate::core::state::GLOBAL_STATE;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

pub const APP_LOCK: &str = "app.lock";
pub const APP_PID: &str = "app.pid";
pub const HEARTBEAT: &str = "heartbeat";
pub const CLEAN_EXIT: &str = "clean_exit";
pub const SUPERVISOR_LOCK: &str = "supervisor.lock";
/// 앱 옆에 이 감시 프로그램이 있으면 직접 실행된 앱을 감시로 넘긴다(배포는 Windows만, macOS 번들에는 없다).
#[cfg(target_os = "windows")]
pub const SUPERVISOR_EXE: &str = "atmos_supervisor.exe";
#[cfg(not(target_os = "windows"))]
pub const SUPERVISOR_EXE: &str = "atmos_supervisor";

/// 재생 중 무음 경고 누적 횟수. core::show_state가 올리고 하트비트가 감시에 알린다.
pub static SILENT_WARNINGS: AtomicU32 = AtomicU32::new(0);

/// 벽시계 밀리초.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 임시 파일에 쓰고 이름을 바꾼다. 읽는 쪽(감시, 다음 실행)이 반쯤 쓴 내용을 보지 않는다.
pub fn atomic_write(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

/// 감시가 읽는 하트비트 값.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeartbeatValues {
    pub pid: u32,
    /// UI가 하트비트를 부른 시각(밀리초). 부를 때마다 커진다.
    pub ui_ms: u64,
    /// 마지막 오디오 콜백 시각(GLOBAL_STATE.watchdog_last_callback, 밀리초).
    pub audio_cb_ms: u64,
    pub engine_active: bool,
    pub silent_warnings: u32,
}

impl HeartbeatValues {
    pub fn render(&self) -> String {
        format!(
            "pid={}\nui_ms={}\naudio_cb_ms={}\nengine_active={}\nsilent_warnings={}\n",
            self.pid,
            self.ui_ms,
            self.audio_cb_ms,
            u8::from(self.engine_active),
            self.silent_warnings
        )
    }
}

/// 지금 값으로 하트비트를 만든다. 같은 밀리초에 두 번 불러도 ui_ms가 커진다 — 감시는 값이 바뀌는지만 본다.
pub fn current_heartbeat() -> HeartbeatValues {
    static LAST_UI_MS: AtomicU64 = AtomicU64::new(0);
    let now = now_ms();
    let prev = LAST_UI_MS
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |prev| Some(now.max(prev + 1)))
        .unwrap_or_else(|prev| prev);
    HeartbeatValues {
        pid: std::process::id(),
        ui_ms: now.max(prev + 1),
        audio_cb_ms: GLOBAL_STATE.watchdog_last_callback.load(Ordering::Relaxed),
        engine_active: crate::api::simple::ENGINE_ACTIVE.load(Ordering::SeqCst),
        silent_warnings: SILENT_WARNINGS.load(Ordering::Relaxed),
    }
}

pub fn write_heartbeat(dir: &Path, values: &HeartbeatValues) -> std::io::Result<()> {
    atomic_write(&dir.join(HEARTBEAT), &values.render())
}

/// 운영자가 창을 닫는다는 표시. 감시는 이 PID의 앱이 끝나면 다시 띄우지 않는다.
pub fn mark_clean_exit(dir: &Path, pid: u32) -> std::io::Result<()> {
    atomic_write(&dir.join(CLEAN_EXIT), &format!("pid={pid}\n"))
}

/// 시작 관문이 쓰는 폴더와 바깥 동작. 테스트는 동작을 바꿔 끼운다.
pub struct Gate<'a> {
    /// 신호 파일 폴더(앱 로그 폴더).
    pub dir: &'a Path,
    /// 앱 옆의 감시 프로그램(있을 때만).
    pub supervisor_exe: Option<&'a Path>,
    /// 이 앱의 PID.
    pub pid: u32,
    pub launch_supervisor: &'a dyn Fn(&Path) -> std::io::Result<()>,
    pub bring_to_front: &'a dyn Fn(u32),
}

/// 시작 관문(스펙 4.3). `Proceed`면 app.lock 파일을 함께 돌려준다 — 앱이 끝날 때까지 쥐고 있어야 다른
/// 실행이 중복으로 끝난다. 잠금 파일을 열 수조차 없으면 잠금 없이 진행한다(중복 막기보다 공연이 먼저다).
pub fn startup_gate(args: &[String], gate: &Gate) -> (StartupDecision, Option<File>) {
    let supervised = args.iter().any(|a| a == "--supervised");
    // 감시 없이 직접 실행됐고, 감시 프로그램이 옆에 있는데 돌고 있지 않으면 감시로 넘긴다.
    let hand_over_to = gate
        .supervisor_exe
        .filter(|_| !supervised && !lock_held(&gate.dir.join(SUPERVISOR_LOCK)));
    let lock = match open_and_lock(&gate.dir.join(APP_LOCK)) {
        Ok(Some(file)) => Some(file),
        Ok(None) => {
            // 다른 앱이 떠 있다. 그 창을 앞으로 가져오고, 감시가 없으면 감시를 띄워 그 앱을 넘겨받게 한다.
            if let Some(pid) = read_pid(&gate.dir.join(APP_PID)) {
                (gate.bring_to_front)(pid);
            }
            if let Some(exe) = hand_over_to {
                let _ = (gate.launch_supervisor)(exe);
            }
            return (StartupDecision::Duplicate, None);
        }
        Err(_) => None,
    };
    if let Some(exe) = hand_over_to {
        // 잠금을 쥔 채 감시를 띄우고 돌아가며 놓는다. 감시가 띄우는 앱은 그보다 한참 늦게 잠금을 잡는다.
        match (gate.launch_supervisor)(exe) {
            Ok(()) => return (StartupDecision::HandedToSupervisor, None),
            Err(e) => GLOBAL_STATE.log(format!("감시 프로그램을 띄우지 못했다 — 감시 없이 진행한다: {e}")),
        }
    }
    if lock.is_some() {
        let _ = atomic_write(&gate.dir.join(APP_PID), &format!("{}\n", gate.pid));
    }
    let _ = std::fs::remove_file(gate.dir.join(CLEAN_EXIT));
    (StartupDecision::Proceed, lock)
}

/// [path]를 열어 잠근다. 다른 프로세스(또는 같은 프로세스의 다른 열기)가 쥐고 있으면 Ok(None).
fn open_and_lock(path: &Path) -> std::io::Result<Option<File>> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(e)) => Err(e),
    }
}

/// 누가 [path] 잠금을 쥐고 있나(잠깐 잡았다 놓는다).
fn lock_held(path: &Path) -> bool {
    matches!(open_and_lock(path), Ok(None))
}

fn read_pid(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}
```

`rust/src/api/lifecycle.rs`:
```rust
//! 앱 수명 신호: 시작 관문, 하트비트, 정상 종료 표시(docs/superpowers/specs/2026-10-05-crash-relaunch-supervisor-design.md
//! 4.2·4.3). 판단은 core::app_signals에 있다.
use crate::core::app_signals::{self, Gate};
use crate::core::log_file::log_dir;
use crate::core::state::GLOBAL_STATE;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

/// 시작 관문의 결정. Dart `main`이 창을 띄우기 전에 받아 그대로 한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupDecision {
    /// 계속 시작한다.
    Proceed,
    /// 감시 프로그램을 띄웠다. 앱은 종료 코드 0으로 끝낸다(감시가 앱을 다시 띄운다).
    HandedToSupervisor,
    /// 다른 앱이 이미 떠 있다(그 창을 앞으로 가져왔다). 종료 코드 3으로 끝낸다.
    Duplicate,
}

/// 앱이 끝날 때까지 쥐는 app.lock.
static APP_LOCK_FILE: Mutex<Option<File>> = Mutex::new(None);

/// 창을 띄우기 전에 한 번 부른다. [args]는 앱 실행 인자다.
pub fn api_startup_gate(args: Vec<String>) -> StartupDecision {
    let dir = log_dir();
    let supervisor = supervisor_next_to_app();
    let gate = Gate {
        dir: &dir,
        supervisor_exe: supervisor.as_deref(),
        pid: std::process::id(),
        launch_supervisor: &launch_supervisor,
        bring_to_front: &bring_window_to_front,
    };
    let (decision, lock) = app_signals::startup_gate(&args, &gate);
    if decision == StartupDecision::Proceed && lock.is_none() {
        GLOBAL_STATE.log("시작 관문: app.lock을 잡지 못했다 — 중복 실행을 막지 못한 채 진행한다".into());
    }
    *APP_LOCK_FILE.lock().unwrap_or_else(|e| e.into_inner()) = lock;
    GLOBAL_STATE.log(format!("시작 관문: {decision:?} (인자 {args:?})"));
    decision
}

/// Dart가 2초마다 부른다. 감시가 이 파일로 앱이 응답하는지 본다.
pub fn api_heartbeat() {
    let _ = app_signals::write_heartbeat(&log_dir(), &app_signals::current_heartbeat());
}

/// 운영자가 창을 닫을 때 엔진을 멈추기 전에 부른다. 감시는 이 앱을 다시 띄우지 않는다.
pub fn api_mark_clean_exit() {
    if let Err(e) = app_signals::mark_clean_exit(&log_dir(), std::process::id()) {
        GLOBAL_STATE.log(format!("정상 종료 표시를 남기지 못했다(감시가 다시 띄울 수 있다): {e}"));
    }
}

fn supervisor_next_to_app() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?.parent()?.join(app_signals::SUPERVISOR_EXE);
    exe.is_file().then_some(exe)
}

/// 감시 프로그램을 띄운다. 이 앱은 곧 끝나므로 기다리지 않는다.
#[allow(clippy::zombie_processes)]
fn launch_supervisor(exe: &Path) -> std::io::Result<()> {
    Command::new(exe)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}

/// 그 PID의 보이는 최상위 창을 앞으로 가져온다(최소화돼 있으면 되살린다).
#[cfg(target_os = "windows")]
fn bring_window_to_front(pid: u32) {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    struct Search {
        pid: u32,
        found: Option<HWND>,
    }

    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = &mut *(lparam.0 as *mut Search);
        let mut owner = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut owner as *mut u32));
        if owner == search.pid && IsWindowVisible(hwnd).as_bool() {
            search.found = Some(hwnd);
            return BOOL(0); // 찾았으니 그만 돈다
        }
        BOOL(1)
    }

    let mut search = Search { pid, found: None };
    unsafe {
        let _ = EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize));
        if let Some(hwnd) = search.found {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

/// macOS는 확인용이라 창을 앞으로 가져오지 않는다(현장은 Windows).
#[cfg(not(target_os = "windows"))]
fn bring_window_to_front(_pid: u32) {}
```

`rust/Cargo.toml`:
```diff
--- a/atmos_mixer_pro/rust/Cargo.toml
+++ b/atmos_mixer_pro/rust/Cargo.toml
@@ -48,7 +48,7 @@ coreaudio-sys = "0.2.15"
 
 [target.'cfg(target_os = "windows")'.dependencies]
 cpal = { version = "0.16.0", features = ["asio"] }
-windows = { version = "0.58.0", features = ["Win32_Media_Audio", "Win32_System_Com", "Win32_Devices_Properties", "Win32_Foundation", "Win32_UI_Shell_PropertiesSystem", "Win32_UI_Shell", "Win32_System_Memory", "Win32_System_Threading", "Win32_System_Power"] }
+windows = { version = "0.58.0", features = ["Win32_Media_Audio", "Win32_System_Com", "Win32_Devices_Properties", "Win32_Foundation", "Win32_UI_Shell_PropertiesSystem", "Win32_UI_Shell", "Win32_System_Memory", "Win32_System_Threading", "Win32_System_Power", "Win32_UI_WindowsAndMessaging"] }
 
 [profile.dev]
 opt-level = 3
```

`rust/src/core/mod.rs`(최종 파일. `show_state`는 Task 8):
```rust
pub mod app_signals;
pub mod keep_awake;
pub mod log_file;
pub mod restart_resume;
pub mod show_state;
pub mod state;
```

- [ ] **Step 2: 실패를 확인한다**(구현 전)

  Run: `cargo test --test test_app_signals`
  Expected: `app_signals`·`lifecycle`이 없어 컴파일 실패.

- [ ] **Step 4: 통과를 확인한다**

  Run: `cargo test --test test_app_signals` → 모두 통과.
  Run(`rust/`에서): `cargo clippy` → 이 작업이 바꾼 줄에 새 경고 0(기존 경고는 그대로).
  빌드·장치 테스트 전에 Global Constraints의 두 가지(다른 Atmos 앱 없음, 디스크 3GB 이상)를 확인한다.

- [ ] **Step 5: 체크포인트** — `git status`·`git diff --stat`로 이 작업의 파일만 바뀌었는지 본다. 커밋은 사용자 요청 시.

---

### Task 8: 공연 상태 저장과 이어 가기(`core::show_state`, `api::show`)

**상태:** 구현 파일·시험 있음(코드는 작업 트리의 파일을 옮겼다). Main 보고: 앱 Rust 전체 `cargo test` 78개 바이너리 통과, 새 clippy 경고 없음(`listener.rs` 219·312행 `get(0)` 경고는 기존).

**Files:**
- Create: `rust/src/core/show_state.rs`, `rust/tests/test_show_state_rules.rs`, `rust/tests/test_show_resume.rs`
- Modify: `rust/src/api/show.rs`(`api_start_show_state`, `api_resume_show`), `rust/src/core/mod.rs`(Task 7의 파일 참고)

- [ ] **Step 1: 실패하는 테스트를 쓴다**

`rust/tests/test_show_state_rules.rs`:
```rust
//! 공연 이어 가기 판단, 저장 형식, 무음 판단(core::show_state). 전역 상태를 쓰지 않는다.
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::show_state::{
    decide_resume, load, save, silence_condition, ResumePlan, SavedTrack, ShowState, SilenceWatch,
    MAX_RESUME_AGE_MS, SHOW_STATE_FILE,
};

const NOW: u64 = 1_759_650_000_000;

fn track(id: &str) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: format!("/{id}.wav"),
        volume: 1.0,
        is_loop: false,
        is_streaming: false,
        output_channel: 0,
        output_stereo: false,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
    }
}

fn room(id: &str, tracks: Vec<TrackConfig>) -> RoomConfig {
    RoomConfig {
        id: id.into(),
        name: id.into(),
        color_hex: "#ffffff".into(),
        volume: 1.0,
        volume_osc_address: String::new(),
        clear_osc_address: String::new(),
        tracks,
    }
}

fn config() -> AppConfig {
    AppConfig {
        rooms: vec![room("a", vec![track("la"), track("oa")]), room("b", vec![track("lb")])],
        ..AppConfig::default()
    }
}

fn saved(room: &str, track: &str, seconds: f64) -> SavedTrack {
    SavedTrack { room_id: room.into(), track_id: track.into(), seconds }
}

#[test]
fn 이어_가기는_10분_안에_저장했고_지금_설정에_남은_트랙이_있을_때만이다() {
    let cfg = config();
    let s = ShowState {
        saved_at_ms: NOW - 5_000,
        active_room_id: Some("a".into()),
        tracks: vec![saved("a", "la", 12.5), saved("a", "oa", 3.0)],
    };
    assert_eq!(
        decide_resume(Some(&s), NOW, Some(&cfg)),
        ResumePlan::Resume { active_room_id: Some("a".into()), tracks: s.tracks.clone() }
    );

    // 지금 설정에서 사라진 트랙(다른 방으로 옮겨진 것 포함)은 빼고 이어 간다
    let moved = ShowState {
        tracks: vec![saved("a", "la", 12.5), saved("a", "gone", 1.0), saved("x", "lb", 2.0)],
        ..s.clone()
    };
    assert_eq!(
        decide_resume(Some(&moved), NOW, Some(&cfg)),
        ResumePlan::Resume { active_room_id: Some("a".into()), tracks: vec![saved("a", "la", 12.5)] }
    );

    // 활성 방이 설정에서 사라졌으면 활성 방 없이 이어 간다
    let no_room = ShowState { active_room_id: Some("gone".into()), ..s.clone() };
    assert!(matches!(
        decide_resume(Some(&no_room), NOW, Some(&cfg)),
        ResumePlan::Resume { active_room_id: None, .. }
    ));

    // 딱 10분은 이어 간다
    let edge = ShowState { saved_at_ms: NOW - MAX_RESUME_AGE_MS, ..s.clone() };
    assert!(matches!(decide_resume(Some(&edge), NOW, Some(&cfg)), ResumePlan::Resume { .. }));

    // 첫 방 테마로 가는 경우
    let theme = |plan: ResumePlan| matches!(plan, ResumePlan::ThemeStart(_));
    assert!(theme(decide_resume(None, NOW, Some(&cfg))), "저장 파일 없음");
    let old = ShowState { saved_at_ms: NOW - MAX_RESUME_AGE_MS - 1, ..s.clone() };
    assert!(theme(decide_resume(Some(&old), NOW, Some(&cfg))), "10분 지남");
    let gone = ShowState { tracks: vec![saved("a", "gone", 1.0)], ..s.clone() };
    assert!(theme(decide_resume(Some(&gone), NOW, Some(&cfg))), "트랙이 설정에서 사라짐");
    let idle = ShowState { tracks: vec![], ..s.clone() };
    assert!(theme(decide_resume(Some(&idle), NOW, Some(&cfg))), "재생 중이던 게 없었음");
    assert!(theme(decide_resume(Some(&s), NOW, None)), "설정이 없음");
    let future = ShowState { saved_at_ms: NOW + 3_600_000, ..s.clone() };
    assert!(theme(decide_resume(Some(&future), NOW, Some(&cfg))), "저장 시각이 한 시간 뒤(시계가 뒤로 감)");
}

#[test]
fn 저장_파일은_고스란히_다시_읽히고_깨진_파일은_오류다() {
    let dir = std::env::temp_dir().join(format!("atmos_show_rules_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    assert!(load(&dir).unwrap().is_none(), "파일이 없으면 None");

    let s = ShowState {
        saved_at_ms: NOW,
        active_room_id: Some("a".into()),
        tracks: vec![saved("a", "la", 12.5), saved("b", "lb", 0.25)],
    };
    save(&dir, &s).unwrap();
    assert_eq!(load(&dir).unwrap(), Some(s));
    assert!(!dir.join(format!("{SHOW_STATE_FILE}.tmp")).exists(), "임시 파일이 남았다");

    std::fs::write(dir.join(SHOW_STATE_FILE), "{\"saved_at_ms\": 1, \"trac").unwrap();
    assert!(load(&dir).is_err(), "깨진 파일을 읽었다고 했다");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 무음은_60초가_지나면_경고하고_이어지면_10분마다_다시_경고한다() {
    let mut w = SilenceWatch::default();
    // 저장 스레드처럼 5초마다 본다
    let warned: Vec<u64> = (0..=150u64).map(|i| i * 5_000).filter(|&t| w.observe(t, true)).collect();
    assert_eq!(warned, vec![60_000, 660_000]);
    assert!(!w.observe(755_000, false), "소리가 나면 경고하지 않는다");
    assert!(!w.observe(760_000, true), "다시 처음부터 잰다");
    assert!(!w.observe(815_000, true));
    assert!(w.observe(820_000, true));
}

#[test]
fn 무음_조건은_루프_재생_중이고_all_mute가_꺼져_있고_모든_피크가_0일_때다() {
    assert!(silence_condition(true, false, [0.0, 0.0]));
    assert!(!silence_condition(false, false, [0.0, 0.0]), "루프가 안 돈다(단발 사이 정적은 정상)");
    assert!(!silence_condition(true, true, [0.0, 0.0]), "All Mute");
    assert!(!silence_condition(true, false, [0.0, 1e-6]), "아주 작은 소리라도 난다");
    assert!(silence_condition(true, false, []), "출력 채널이 없으면 나가는 소리도 없다");
}
```

`rust/tests/test_show_resume.rs`(Review Focus 1: 저장 스레드가 파일을 덮어쓴 뒤에도 앱 시작 때 읽어 둔 지난 위치로 이어 간다):
```rust
//! 공연 상태 저장과 이어 가기(core::show_state). 전역 상태를 쓰므로 테스트는 하나다.
use rust_lib_atmos_mixer_pro::api::simple::api_set_active_room;
use rust_lib_atmos_mixer_pro::audio::engine::ENGINE_GENERATION;
use rust_lib_atmos_mixer_pro::audio::playback_cursor::CURSOR_TABLE;
use rust_lib_atmos_mixer_pro::audio::player::SoundData;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::app_signals::now_ms;
use rust_lib_atmos_mixer_pro::core::restart_resume;
use rust_lib_atmos_mixer_pro::core::show_state::{self, capture, load, save, SavedTrack, ShowState};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

fn tone_wav(path: &std::path::Path, seconds: f32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..(48_000.0 * seconds) as usize {
        let v = (i as f32 * 200.0 * std::f32::consts::TAU / 48_000.0).sin() * 3000.0;
        w.write_sample(v as i16).unwrap();
    }
    w.finalize().unwrap();
}

fn track(id: &str, path: &str, is_loop: bool) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: path.into(),
        volume: 1.0,
        is_loop,
        is_streaming: false,
        output_channel: 1,
        output_stereo: false,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
    }
}

fn room(id: &str, tracks: Vec<TrackConfig>) -> RoomConfig {
    RoomConfig {
        id: id.into(),
        name: id.into(),
        color_hex: "#ffffff".into(),
        volume: 1.0,
        volume_osc_address: String::new(),
        clear_osc_address: String::new(),
        tracks,
    }
}

fn playing() -> Vec<String> {
    let mut v: Vec<String> = GLOBAL_STATE.playing_track_ids.read().unwrap().values().cloned().collect();
    v.sort();
    v
}

fn drain() -> Vec<AudioCommand> {
    let mut v = Vec::new();
    while let Ok(c) = GLOBAL_STATE.command_receiver.try_recv() {
        v.push(c);
    }
    v
}

/// 재생 명령들의 (트랙, 시작 위치 초), 트랙 이름 순
fn plays(cmds: &[AudioCommand]) -> Vec<(String, f64)> {
    let mut v: Vec<(String, f64)> = cmds
        .iter()
        .filter_map(|c| match c {
            AudioCommand::PlayTrack { instance, .. } => {
                Some((instance.track_id_str.clone(), instance.position_seconds()))
            }
            _ => None,
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

fn saved(room: &str, track: &str, seconds: f64) -> SavedTrack {
    SavedTrack { room_id: room.into(), track_id: track.into(), seconds }
}

#[test]
fn 공연_상태를_저장하고_시작할_때_읽어_둔_위치부터_이어_간다() {
    let dir = std::env::temp_dir().join(format!("atmos_show_resume_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["la", "lb"] {
        tone_wav(&dir.join(format!("{name}.wav")), 5.0);
    }
    let p = |n: &str| dir.join(format!("{n}.wav")).to_string_lossy().into_owned();
    GLOBAL_STATE.sound_cache.write().unwrap().insert(
        "/oa.wav".into(),
        Arc::new(SoundData { samples: vec![0.1; 96_000], channels: 1, sample_rate: 48_000 }),
    );
    GLOBAL_STATE.engine_sample_rate.store(48_000, Ordering::SeqCst);
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig {
        rooms: vec![
            room("a", vec![track("la", &p("la"), true), track("oa", "/oa.wav", false)]),
            room("b", vec![track("lb", &p("lb"), true)]),
        ],
        ..AppConfig::default()
    });
    GLOBAL_STATE.clear_playing_tracks();
    drain();

    // 1) 저장 내용: 위치를 아는 인스턴스만, 방 ID와 함께. 루프·단발 모두.
    GLOBAL_STATE.add_playing_track(9_001, "la".into());
    GLOBAL_STATE.add_playing_track(9_002, "oa".into());
    GLOBAL_STATE.add_playing_track(9_003, "lb".into()); // 위치가 아직 없다(막 큐에 들어감)
    CURSOR_TABLE.clear_all();
    CURSOR_TABLE.publish(0, 9_001, 2.5);
    CURSOR_TABLE.publish(1, 9_002, 1.25);
    api_set_active_room(Some("a".into())).unwrap();
    assert_eq!(
        capture(1_000),
        ShowState {
            saved_at_ms: 1_000,
            active_room_id: Some("a".into()),
            tracks: vec![saved("a", "la", 2.5), saved("a", "oa", 1.25)],
        }
    );

    // 엔진 재시작 중(재생 목록이 비고 재개를 기다림)에도 그 트랙을 저장한다
    restart_resume::take_snapshot(ENGINE_GENERATION.load(Ordering::SeqCst));
    assert!(playing().is_empty());
    assert_eq!(capture(2_000).tracks, vec![saved("a", "la", 2.5), saved("a", "oa", 1.25)]);
    restart_resume::cancel_all();
    CURSOR_TABLE.clear_all();
    assert!(capture(3_000).tracks.is_empty());

    // 2) 시작할 때 지난 상태를 읽어 둔다. 저장 스레드가 곧 파일을 지금(빈) 상태로 덮어써도
    //    이어 가기는 읽어 둔 상태를 쓴다.
    let previous = ShowState {
        saved_at_ms: now_ms() - 3_000,
        active_room_id: Some("b".into()),
        tracks: vec![saved("b", "lb", 2.5), saved("a", "oa", 1.5)],
    };
    save(&dir, &previous).unwrap();
    show_state::start(dir.clone(), Duration::from_millis(100));
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(
        load(&dir).unwrap().map(|s| s.tracks),
        Some(vec![]),
        "저장 스레드가 파일을 지금 상태로 덮어쓰지 않았다"
    );

    drain();
    show_state::resume(now_ms()).unwrap();
    assert_eq!(*GLOBAL_STATE.active_room_id.read().unwrap(), Some("b".to_string()));
    let p = plays(&drain());
    assert_eq!(p.len(), 2, "이어 튼 트랙: {p:?}");
    assert_eq!(p[0].0, "lb");
    assert!((p[0].1 - 2.5).abs() < 1e-6, "lb 시작 위치 {}", p[0].1);
    assert_eq!(p[1].0, "oa");
    assert!((p[1].1 - 1.5).abs() < 1e-6, "oa 시작 위치 {}", p[1].1);
    assert_eq!(playing(), vec!["lb".to_string(), "oa".to_string()]);

    // 3) 읽어 둔 상태는 한 번만 쓴다. 다시 부르면 첫 방 테마로 시작한다.
    show_state::resume(now_ms()).unwrap();
    assert_eq!(*GLOBAL_STATE.active_room_id.read().unwrap(), Some("a".to_string()));
    assert_eq!(playing(), vec!["la".to_string()]);

    drain();
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 3: 구현**

`rust/src/core/show_state.rs`(Task 9 부분 제외):
```rust
//! 공연 상태 저장과 이어 가기(docs/superpowers/specs/2026-10-05-crash-relaunch-supervisor-design.md 4.4),
//! 재생 중 무음 경고(4.5). 전용 스레드가 5초마다 재생 중인 트랙과 위치를 show_state.json에 저장하고 무음을 본다.
//! 앱이 시작할 때 지난 상태를 먼저 읽어 두므로, 저장 스레드가 파일을 덮어써도 이어 가기는 지난 상태를 쓴다.
//! 오디오 스레드가 적어 둔 원자 값(피크, All Mute)을 읽기만 한다(DSP 3법칙).
use crate::api::error::AtmosError;
use crate::audio::playback_cursor::CURSOR_TABLE;
use crate::common::config::AppConfig;
use crate::core::state::GLOBAL_STATE;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const SHOW_STATE_FILE: &str = "show_state.json";
/// 저장 간격. 이어 갈 때 위치가 최대 이만큼 앞이다.
pub const SAVE_INTERVAL: Duration = Duration::from_secs(5);
/// 저장한 지 이보다 오래면 이어 가지 않고 첫 방 테마로 시작한다(오래 꺼져 있던 뒤에는 처음부터가 안전하다).
pub const MAX_RESUME_AGE_MS: u64 = 10 * 60 * 1000;
/// 저장 시각이 지금보다 이만큼 넘게 앞서면(시계가 뒤로 감) 믿지 않는다.
const CLOCK_SKEW_MS: u64 = 60 * 1000;
/// 무음이 이만큼 이어지면 경고한다.
pub const SILENCE_WARN_AFTER_MS: u64 = 60 * 1000;
/// 무음이 이어지면 이 간격으로 다시 경고한다.
pub const SILENCE_REPEAT_MS: u64 = 10 * 60 * 1000;

/// 저장한 트랙 하나. 루프는 한 바퀴 안의 위치다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedTrack {
    pub room_id: String,
    pub track_id: String,
    pub seconds: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShowState {
    /// 저장 시각(벽시계 밀리초). 재부팅을 넘어 비교하므로 벽시계를 쓴다.
    pub saved_at_ms: u64,
    pub active_room_id: Option<String>,
    pub tracks: Vec<SavedTrack>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResumePlan {
    /// 저장된 위치부터 이어 간다(활성 방, 지금 설정에 남은 트랙).
    Resume { active_room_id: Option<String>, tracks: Vec<SavedTrack> },
    /// 이어 갈 수 없어 첫 방 테마로 시작한다(이유).
    ThemeStart(String),
}

/// 이어 갈지 정한다.
pub fn decide_resume(saved: Option<&ShowState>, now_ms: u64, config: Option<&AppConfig>) -> ResumePlan {
    let Some(saved) = saved else {
        return ResumePlan::ThemeStart("저장된 공연 상태가 없다".into());
    };
    if saved.saved_at_ms > now_ms.saturating_add(CLOCK_SKEW_MS) {
        return ResumePlan::ThemeStart("저장 시각이 지금보다 뒤다(시계가 바뀜)".into());
    }
    let age_ms = now_ms.saturating_sub(saved.saved_at_ms);
    if age_ms > MAX_RESUME_AGE_MS {
        return ResumePlan::ThemeStart(format!("저장한 지 {}초가 지났다(10분 넘음)", age_ms / 1000));
    }
    if saved.tracks.is_empty() {
        return ResumePlan::ThemeStart("재생 중이던 트랙이 없었다".into());
    }
    let tracks: Vec<SavedTrack> = saved
        .tracks
        .iter()
        .filter(|t| {
            config.is_some_and(|c| {
                c.rooms.iter().any(|r| r.id == t.room_id && r.tracks.iter().any(|x| x.id == t.track_id))
            })
        })
        .cloned()
        .collect();
    if tracks.is_empty() {
        return ResumePlan::ThemeStart("저장된 트랙이 지금 설정에 없다".into());
    }
    let active_room_id = saved
        .active_room_id
        .clone()
        .filter(|id| config.is_some_and(|c| c.rooms.iter().any(|r| &r.id == id)));
    ResumePlan::Resume { active_room_id, tracks }
}

/// 지금 공연 상태. 위치를 아는 인스턴스만 넣는다(막 큐에 들어갔거나 이미 끝난 것은 뺀다).
/// 엔진 재시작 복원을 기다리는 트랙도 넣는다 — 재시작 중에는 재생 목록이 잠깐 비어 있다.
pub fn capture(now_ms: u64) -> ShowState {
    let playing = GLOBAL_STATE.playing_track_ids.read().unwrap_or_else(|e| e.into_inner()).clone();
    let cursors: HashMap<u64, f64> = CURSOR_TABLE.snapshot().into_iter().collect();
    let active_room_id = GLOBAL_STATE.active_room_id.read().unwrap_or_else(|e| e.into_inner()).clone();
    let mut tracks: Vec<SavedTrack> = {
        let config = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner());
        playing
            .iter()
            .filter_map(|(instance_id, track_id)| {
                let seconds = *cursors.get(instance_id)?;
                let room = config.as_ref()?.rooms.iter().find(|r| r.tracks.iter().any(|t| &t.id == track_id))?;
                Some(SavedTrack { room_id: room.id.clone(), track_id: track_id.clone(), seconds })
            })
            .collect()
    };
    tracks.extend(crate::core::restart_resume::pending_entries().into_iter().map(|e| SavedTrack {
        room_id: e.room_id,
        track_id: e.track_id,
        seconds: e.seconds,
    }));
    tracks.sort_by(|a, b| a.track_id.cmp(&b.track_id).then(a.seconds.total_cmp(&b.seconds)));
    ShowState { saved_at_ms: now_ms, active_room_id, tracks }
}

pub fn save(dir: &Path, state: &ShowState) -> std::io::Result<()> {
    let json = serde_json::to_string(state).map_err(std::io::Error::other)?;
    crate::core::app_signals::atomic_write(&dir.join(SHOW_STATE_FILE), &json)
}

/// 저장 파일을 읽는다. 없으면 Ok(None), 깨졌으면 Err.
pub fn load(dir: &Path) -> Result<Option<ShowState>, serde_json::Error> {
    let Ok(text) = std::fs::read_to_string(dir.join(SHOW_STATE_FILE)) else {
        return Ok(None);
    };
    serde_json::from_str(&text).map(Some)
}

/// 앱이 시작할 때 읽어 둔 지난 공연 상태. 이어 가기가 한 번 꺼내 쓴다.
static PREVIOUS: Mutex<Option<ShowState>> = Mutex::new(None);

/// 지난 공연 상태를 읽어 두고 저장 스레드를 시작한다(앱 시작 때 한 번, 두 번째부터는 무시).
/// 첫 저장은 [interval] 뒤라 읽기가 먼저다.
pub fn start(dir: PathBuf, interval: Duration) {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        let previous = match load(&dir) {
            Ok(previous) => previous,
            Err(e) => {
                GLOBAL_STATE.log(format!("공연 상태 파일이 깨져 있어 쓰지 않는다: {e}"));
                None
            }
        };
        *PREVIOUS.lock().unwrap_or_else(|e| e.into_inner()) = previous;
        let spawned = std::thread::Builder::new()
            .name("show-state".into())
            .spawn(move || saver_loop(&dir, interval));
        if let Err(e) = spawned {
            GLOBAL_STATE.log(format!("공연 상태 저장 스레드를 시작하지 못했다: {e}"));
        }
    });
}

fn saver_loop(dir: &Path, interval: Duration) {
    let started = Instant::now();
    let mut silence = SilenceWatch::default();
    let mut failing = false;
    loop {
        std::thread::sleep(interval);
        match save(dir, &capture(crate::core::app_signals::now_ms())) {
            Ok(()) => failing = false,
            Err(e) => {
                if !failing {
                    GLOBAL_STATE.log(format!("공연 상태를 저장하지 못했다(다시 시도한다): {e}"));
                }
                failing = true;
            }
        }
        if let Some(count) = check_silence(&mut silence, started.elapsed().as_millis() as u64) {
            GLOBAL_STATE.log(format!(
                "무음 경고({count}회째): 루프를 재생 중이고 All Mute가 꺼져 있는데 출력이 1분 넘게 완전히 0이다. \
                 라우팅·장치·볼륨을 확인하세요"
            ));
        }
    }
}

// … Task 9의 무음 판단 코드(`SilenceWatch`, `silence_condition`, `check_silence`, `silent_now`)는 Task 9 블록에 있다. 위 `saver_loop`가 부른다.

/// 시작할 때 읽어 둔 지난 상태로 이어 간다. 이어 갈 수 없으면 첫 방 테마로 시작한다. 읽어 둔 상태는 한 번만 쓴다.
pub fn resume(now_ms: u64) -> Result<(), AtmosError> {
    let previous = PREVIOUS.lock().unwrap_or_else(|e| e.into_inner()).take();
    let plan = {
        let config = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner());
        decide_resume(previous.as_ref(), now_ms, config.as_ref())
    };
    match plan {
        ResumePlan::Resume { active_room_id, tracks } => {
            GLOBAL_STATE.log(format!(
                "공연 이어 가기: 활성 방 {active_room_id:?}, 트랙 {}개를 멈춘 위치부터 다시 튼다",
                tracks.len()
            ));
            GLOBAL_STATE.set_active_room(active_room_id);
            for t in tracks {
                if let Err(e) = crate::api::simple::play_track_from(t.room_id, t.track_id.clone(), t.seconds) {
                    GLOBAL_STATE.log(format!("공연 이어 가기: {} 재생 실패: {}", t.track_id, e.message));
                }
            }
            Ok(())
        }
        ResumePlan::ThemeStart(reason) => {
            GLOBAL_STATE.log(format!("공연 이어 가기 대신 첫 방 테마로 시작한다: {reason}"));
            crate::api::show::api_theme_start()
        }
    }
}
```

`rust/src/api/show.rs`에 더한 함수:
```rust
/// 지난 공연 상태를 읽어 두고 5초마다 저장을 시작한다. 앱 시작 때(시작 관문 다음) 한 번 부른다.
/// [dir]는 앱 지원 폴더(설정 파일과 같은 곳)다.
pub fn api_start_show_state(dir: String) {
    crate::core::show_state::start(std::path::PathBuf::from(dir), crate::core::show_state::SAVE_INTERVAL);
}

/// 감시가 다시 띄웠거나(--auto-relaunched) 로그인 자동 실행이면, 앱 시작 절차(설정 적용·엔진 준비) 뒤 한 번 부른다.
/// 시작할 때 읽어 둔 위치부터 이어 가고, 이어 갈 수 없으면 첫 방 테마로 시작한다.
pub fn api_resume_show() -> Result<(), AtmosError> {
    crate::core::show_state::resume(crate::core::app_signals::now_ms())
}
```

- [ ] **Step 2: 실패를 확인한다**(구현 전)

  Run: `cargo test --test test_show_state_rules --test test_show_resume`
  Expected: `show_state`가 없어 컴파일 실패.

- [ ] **Step 4: 통과를 확인한다**

  Run: `cargo test --test test_show_state_rules --test test_show_resume` → 모두 통과.
  Run(`rust/`에서): `cargo clippy` → 이 작업이 바꾼 줄에 새 경고 0(기존 경고는 그대로).
  빌드·장치 테스트 전에 Global Constraints의 두 가지(다른 Atmos 앱 없음, 디스크 3GB 이상)를 확인한다.

- [ ] **Step 5: 체크포인트** — `git status`·`git diff --stat`로 이 작업의 파일만 바뀌었는지 본다. 커밋은 사용자 요청 시.

---

### Task 9: 무음 경고

**상태:** 구현 파일·시험 있음(코드는 작업 트리의 파일을 옮겼다). Main 보고: 앱 Rust 전체 `cargo test` 78개 바이너리 통과, 새 clippy 경고 없음(`listener.rs` 219·312행 `get(0)` 경고는 기존).

**Files:**
- Modify: `rust/src/core/show_state.rs`(무음 판단, `saver_loop`가 부른다)
- Create: `rust/tests/test_silence_warning.rs`

- [ ] **Step 1: 실패하는 테스트를 쓴다** — `rust/tests/test_silence_warning.rs`

```rust
//! 재생 중 무음 경고(core::show_state). 전역 상태를 쓰므로 테스트는 하나다.
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::app_signals::SILENT_WARNINGS;
use rust_lib_atmos_mixer_pro::core::show_state::{check_silence, SilenceWatch};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;

fn track(id: &str, is_loop: bool) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: format!("/{id}.wav"),
        volume: 1.0,
        is_loop,
        is_streaming: false,
        output_channel: 0,
        output_stereo: false,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
    }
}

fn set_peak(channel: usize, value: f32) {
    GLOBAL_STATE.vu_levels[channel].store(value.to_bits(), Ordering::Relaxed);
}

#[test]
fn 루프_재생_중_출력이_1분_넘게_0이면_경고_횟수를_올린다() {
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig {
        rooms: vec![RoomConfig {
            id: "a".into(),
            name: "a".into(),
            color_hex: "#ffffff".into(),
            volume: 1.0,
            volume_osc_address: String::new(),
            clear_osc_address: String::new(),
            tracks: vec![track("la", true), track("oa", false)],
        }],
        ..AppConfig::default()
    });
    GLOBAL_STATE.clear_playing_tracks();
    GLOBAL_STATE.active_device_channels.store(2, Ordering::Relaxed);
    set_peak(0, 0.0);
    set_peak(1, 0.0);
    GLOBAL_STATE.master_mute.store(false, Ordering::Relaxed);
    let before = SILENT_WARNINGS.load(Ordering::Relaxed);
    let mut w = SilenceWatch::default();

    // 단발만 돌면 경고하지 않는다(단발 사이 정적은 정상)
    GLOBAL_STATE.add_playing_track(1, "oa".into());
    assert_eq!(check_silence(&mut w, 0), None);
    assert_eq!(check_silence(&mut w, 120_000), None);

    // 루프가 도는데 출력이 0
    GLOBAL_STATE.add_playing_track(2, "la".into());
    assert_eq!(check_silence(&mut w, 125_000), None, "막 무음이 시작됐다");
    assert_eq!(check_silence(&mut w, 185_000), Some(before + 1), "60초 무음");
    assert_eq!(SILENT_WARNINGS.load(Ordering::Relaxed), before + 1, "하트비트가 알릴 횟수");
    assert_eq!(check_silence(&mut w, 190_000), None, "10분 안에는 다시 경고하지 않는다");

    // All Mute면 경고하지 않고, 풀린 뒤 다시 잰다
    GLOBAL_STATE.master_mute.store(true, Ordering::Relaxed);
    assert_eq!(check_silence(&mut w, 300_000), None);
    GLOBAL_STATE.master_mute.store(false, Ordering::Relaxed);
    assert_eq!(check_silence(&mut w, 305_000), None);
    assert_eq!(check_silence(&mut w, 365_000), Some(before + 2));

    // 소리가 나면 처음부터 다시 잰다
    set_peak(1, 0.02);
    assert_eq!(check_silence(&mut w, 370_000), None);
    set_peak(1, 0.0);
    assert_eq!(check_silence(&mut w, 375_000), None);
    assert_eq!(check_silence(&mut w, 435_000), Some(before + 3));

    GLOBAL_STATE.clear_playing_tracks();
    GLOBAL_STATE.active_device_channels.store(0, Ordering::Relaxed);
}
```

- [ ] **Step 3: 구현** — `rust/src/core/show_state.rs`의 이 작업 부분(상수 `SILENCE_WARN_AFTER_MS`, `SILENCE_REPEAT_MS`는 파일 머리에 있다. 재실행은 하지 않고 앱 로그에 경고만 남긴다)

```rust
/// 재생 중 무음 판단. 무음 조건이 60초 이어지면 경고하고, 계속 이어지면 10분마다 다시 경고한다.
/// 조건이 풀리면(소리가 남, All Mute, 루프 정지) 처음부터 다시 잰다.
#[derive(Debug, Default)]
pub struct SilenceWatch {
    silent_since_ms: Option<u64>,
    last_warned_ms: Option<u64>,
}

impl SilenceWatch {
    /// [now_ms]는 단조 시계(밀리초)다. 이번에 경고해야 하면 true.
    pub fn observe(&mut self, now_ms: u64, silent: bool) -> bool {
        if !silent {
            *self = SilenceWatch::default();
            return false;
        }
        let since = *self.silent_since_ms.get_or_insert(now_ms);
        if now_ms.saturating_sub(since) < SILENCE_WARN_AFTER_MS {
            return false;
        }
        if self.last_warned_ms.is_some_and(|warned| now_ms.saturating_sub(warned) < SILENCE_REPEAT_MS) {
            return false;
        }
        self.last_warned_ms = Some(now_ms);
        true
    }
}

/// 무음 경고 조건: 루프 트랙 재생 중, All Mute 꺼짐, 모든 출력 채널 피크가 정확히 0.
pub fn silence_condition(loop_playing: bool, master_mute: bool, peaks: impl IntoIterator<Item = f32>) -> bool {
    loop_playing && !master_mute && peaks.into_iter().all(|peak| peak == 0.0)
}

/// 저장 스레드가 저장할 때마다 부른다. 경고해야 하면 누적 횟수(하트비트가 감시에 알림)를 올리고 그 값을 돌려준다.
pub fn check_silence(watch: &mut SilenceWatch, now_ms: u64) -> Option<u32> {
    if !watch.observe(now_ms, silent_now()) {
        return None;
    }
    Some(crate::core::app_signals::SILENT_WARNINGS.fetch_add(1, Ordering::Relaxed) + 1)
}

fn silent_now() -> bool {
    let playing: Vec<String> =
        GLOBAL_STATE.playing_track_ids.read().unwrap_or_else(|e| e.into_inner()).values().cloned().collect();
    let loop_playing = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner()).as_ref().is_some_and(|c| {
        c.rooms.iter().flat_map(|r| r.tracks.iter()).any(|t| t.is_loop && playing.contains(&t.id))
    });
    let channels = (GLOBAL_STATE.active_device_channels.load(Ordering::Relaxed) as usize)
        .min(GLOBAL_STATE.vu_levels.len());
    silence_condition(
        loop_playing,
        GLOBAL_STATE.master_mute.load(Ordering::Relaxed),
        GLOBAL_STATE.vu_levels[..channels].iter().map(|v| f32::from_bits(v.load(Ordering::Relaxed))),
    )
}
```

- [ ] **Step 2: 실패를 확인한다**(구현 전)

  Run: `cargo test --test test_silence_warning`
  Expected: 무음 판단이 없어 컴파일 실패.

- [ ] **Step 4: 통과를 확인한다**

  Run: `cargo test --test test_silence_warning` → 모두 통과.
  Run(`rust/`에서): `cargo clippy` → 이 작업이 바꾼 줄에 새 경고 0(기존 경고는 그대로).

- [ ] **Step 5: 체크포인트** — `git status`·`git diff --stat`로 이 작업의 파일만 바뀌었는지 본다. 커밋은 사용자 요청 시.

---

### Task 10: Dart(관문, 하트비트, `clean_exit`, 이어 가기, 환경설정 스위치)

**상태:** 구현 파일·시험 있음(코드는 작업 트리의 파일을 옮겼다). Main 보고: `flutter test` 158개 통과. `flutter analyze` 5건은 전부 HEAD에 있던 기존 경고(`dashboard_screen.dart` 미사용 import 3건, 루트 `backup_panel.dart` 1건, cargokit 1건). FRB codegen을 다시 돌렸다.

**Files:**
- Create: `lib/core/state/launch_mode.dart`, `test/launch_mode_test.dart`
- Modify: `lib/main.dart`, `lib/features/splash/screens/audio_init_splash_screen.dart`(`_resumeShowIfLaunchedForIt`), `lib/features/settings/widgets/preferences_modal.dart`(스위치), `integration_test/app_flow_test.dart`(`app.main(const [])`)
- 생성 파일(codegen 결과라 옮기지 않는다): 새 `lib/src/rust/api/lifecycle.dart`, `lib/src/rust/api/show.dart`, 바뀐 `frb_generated.dart`·`frb_generated.io.dart`·`frb_generated.web.dart`·`frb_generated.rs`(Rust 쪽), `simple.dart`는 주석 한 줄. codegen 뒤 `frb_generated.dart`의 `ioDirectory`가 `rust/target/release/`로 돌아가도 `main.dart`가 macOS 로더를 직접 지정하므로 영향이 없다(커밋 c7d2fd2).

**하트비트 시작 위치:** `AtmosMixerProApp.initState`의 2초 `Timer.periodic`이다. 앞 호출이 끝나지 않았으면 건너뛰어 Rust가 막히면 하트비트가 끊긴다. 3D 웹뷰 로딩과 무관하다.

- [ ] **Step 1: 실패하는 테스트를 쓴다** — `test/launch_mode_test.dart`

```dart
import 'package:atmos_mixer_pro/core/state/launch_mode.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  group('공연 이어 가기 결정', () {
    test('감시가 충돌·멈춤 뒤 다시 띄웠으면 환경설정과 상관없이 이어 간다', () {
      expect(shouldResumeShow(args: ['--supervised', '--auto-relaunched'], resumeOnLogon: false), isTrue);
    });

    test('로그인 자동 실행은 환경설정을 따른다', () {
      expect(shouldResumeShow(args: ['--supervised', '--logon'], resumeOnLogon: true), isTrue);
      expect(shouldResumeShow(args: ['--supervised', '--logon'], resumeOnLogon: false), isFalse);
    });

    test('사람이 켰으면 대기한다', () {
      expect(shouldResumeShow(args: const [], resumeOnLogon: true), isFalse);
      expect(shouldResumeShow(args: ['--supervised'], resumeOnLogon: true), isFalse);
    });
  });

  group('환경설정 "로그인할 때 공연 자동 시작"', () {
    test('기본은 켜짐이고 끈 값은 남는다', () async {
      SharedPreferences.setMockInitialValues({});
      expect(await loadAutoResumeOnLogon(), isTrue);
      await saveAutoResumeOnLogon(false);
      expect(await loadAutoResumeOnLogon(), isFalse);
      await saveAutoResumeOnLogon(true);
      expect(await loadAutoResumeOnLogon(), isTrue);
    });
  });
}
```

- [ ] **Step 3: 구현**

`lib/core/state/launch_mode.dart`:
```dart
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// 앱 실행 인자. main이 받은 값으로 덮어쓴다(감시 프로그램이 --supervised 등을 넘긴다).
final launchArgsProvider = Provider<List<String>>((ref) => const []);

/// 환경설정 "로그인할 때 공연 자동 시작" 키. 이 컴퓨터의 운영 설정이라 프로젝트 파일에 넣지 않는다.
const kAutoResumeOnLogonKey = 'auto_resume_on_logon';

/// 앱 시작 절차 뒤 공연을 이어 갈지 정한다.
/// - 감시 프로그램이 충돌·멈춤 뒤 다시 띄웠다(--auto-relaunched): 항상 이어 간다.
/// - 로그인 자동 실행(--logon): 환경설정이 켜져 있을 때만 이어 간다.
/// - 사람이 바로가기로 켰다: 지금처럼 대기한다.
bool shouldResumeShow({required List<String> args, required bool resumeOnLogon}) {
  if (args.contains('--auto-relaunched')) return true;
  if (args.contains('--logon')) return resumeOnLogon;
  return false;
}

/// "로그인할 때 공연 자동 시작"(기본 켜짐). 점검하러 재부팅할 때 끈다.
Future<bool> loadAutoResumeOnLogon() async {
  final prefs = await SharedPreferences.getInstance();
  return prefs.getBool(kAutoResumeOnLogonKey) ?? true;
}

Future<void> saveAutoResumeOnLogon(bool value) async {
  final prefs = await SharedPreferences.getInstance();
  await prefs.setBool(kAutoResumeOnLogonKey, value);
}

/// 공연 상태 파일(show_state.json)을 두는 앱 지원 폴더. 설정 파일과 같은 곳이다.
Future<String> showStateDir() async => (await getApplicationSupportDirectory()).path;
```

`lib/main.dart`(변경분):
```diff
--- a/atmos_mixer_pro/lib/main.dart
+++ b/atmos_mixer_pro/lib/main.dart
@@ -1,11 +1,15 @@
+import 'dart:async';
 import 'dart:io';
 
 import 'package:flutter/material.dart';
 import 'package:atmos_mixer_pro/core/state/global_state.dart';
 import 'package:atmos_mixer_pro/core/state/engine_resync.dart';
+import 'package:atmos_mixer_pro/core/state/launch_mode.dart';
 import 'package:flutter_riverpod/flutter_riverpod.dart';
 import 'package:window_manager/window_manager.dart';
 import 'package:atmos_mixer_pro/src/rust/frb_generated.dart';
+import 'package:atmos_mixer_pro/src/rust/api/lifecycle.dart';
+import 'package:atmos_mixer_pro/src/rust/api/show.dart';
 import 'package:atmos_mixer_pro/src/rust/api/simple.dart';
 import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
     show ExternalLibrary;
@@ -13,7 +17,7 @@ import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
 import 'package:atmos_mixer_pro/features/splash/screens/audio_init_splash_screen.dart';
 import 'package:atmos_mixer_pro/features/exhibition/state/acoustic_sync_provider.dart';
 
-Future<void> main() async {
+Future<void> main(List<String> args) async {
   WidgetsFlutterBinding.ensureInitialized();
 
   // Initialize rust bridge
@@ -32,6 +36,19 @@ Future<void> main() async {
         : null,
   );
 
+  // 시작 관문(rust api::lifecycle). 창을 띄우기 전에 감시 프로그램으로 넘기거나, 이미 떠 있으면 끝낸다.
+  switch (await apiStartupGate(args: args)) {
+    case StartupDecision.handedToSupervisor:
+      exit(0);
+    case StartupDecision.duplicate:
+      exit(3);
+    case StartupDecision.proceed:
+      break;
+  }
+  // 지난 공연 상태를 읽어 두고 5초마다 저장을 시작한다(rust api::show). 이어 가기(스플래시)는
+  // 여기서 읽어 둔 상태를 쓰므로, 저장이 파일을 덮어써도 멈춘 위치를 잃지 않는다.
+  await apiStartShowState(dir: await showStateDir());
+
   // Initialize window_manager for frameless kiosk mode
   await windowManager.ensureInitialized();
 
@@ -50,7 +67,10 @@ Future<void> main() async {
     await windowManager.focus();
   });
 
-  runApp(const ProviderScope(child: AtmosMixerProApp()));
+  runApp(ProviderScope(
+    overrides: [launchArgsProvider.overrideWithValue(args)],
+    child: const AtmosMixerProApp(),
+  ));
 }
 
 class AtmosMixerProApp extends ConsumerStatefulWidget {
@@ -62,20 +82,45 @@ class AtmosMixerProApp extends ConsumerStatefulWidget {
 
 class _AtmosMixerProAppState extends ConsumerState<AtmosMixerProApp>
     with WindowListener {
+  Timer? _heartbeat;
+  bool _heartbeatInFlight = false;
+
   @override
   void initState() {
     super.initState();
     windowManager.addListener(this);
+    // 감시 프로그램이 이 앱이 응답하는지 본다(rust api::lifecycle). 3D 로딩과 상관없이 바로 시작한다.
+    _beat();
+    _heartbeat = Timer.periodic(const Duration(seconds: 2), (_) => _beat());
+  }
+
+  /// 앞 호출이 아직 안 끝났으면 건너뛴다. Rust가 막혀 있으면 하트비트가 끊겨야 감시가 알아챈다.
+  Future<void> _beat() async {
+    if (_heartbeatInFlight) return;
+    _heartbeatInFlight = true;
+    try {
+      await apiHeartbeat();
+    } catch (_) {
+      // 이번 하트비트를 못 썼다. 다음 주기에 다시 쓴다.
+    } finally {
+      _heartbeatInFlight = false;
+    }
   }
 
   @override
   void dispose() {
+    _heartbeat?.cancel();
     windowManager.removeListener(this);
     super.dispose();
   }
 
   @override
   void onWindowClose() async {
+    // 운영자가 닫는다. 감시 프로그램이 다시 띄우지 않도록 엔진을 멈추기 전에 표시한다(rust api::lifecycle).
+    await Future.any([
+      apiMarkCleanExit(),
+      Future.delayed(const Duration(milliseconds: 500)),
+    ]);
     // Explicitly release ASIO hardware locks and cleanly stop audio engine
     // Adding timeout guard to prevent ghost processes
     await Future.any([
```

`lib/features/splash/screens/audio_init_splash_screen.dart`(변경분):
```diff
--- a/atmos_mixer_pro/lib/features/splash/screens/audio_init_splash_screen.dart
+++ b/atmos_mixer_pro/lib/features/splash/screens/audio_init_splash_screen.dart
@@ -2,6 +2,8 @@ import 'package:flutter/material.dart';
 import 'package:flutter_riverpod/flutter_riverpod.dart';
 import 'package:atmos_mixer_pro/features/dashboard/screens/dashboard_screen.dart';
 import 'package:atmos_mixer_pro/core/state/global_state.dart';
+import 'package:atmos_mixer_pro/core/state/launch_mode.dart';
+import 'package:atmos_mixer_pro/src/rust/api/show.dart' as show_api;
 import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
 import 'package:atmos_mixer_pro/features/settings/widgets/tuning_modal.dart';
 import 'package:atmos_mixer_pro/features/exhibition/state/three_js_engine_provider.dart';
@@ -57,6 +59,8 @@ class _AudioInitSplashScreenState extends ConsumerState<AudioInitSplashScreen> {
       // Apply tuning settings after engine starts
       ref.read(tuningStateProvider.notifier).applyAllToBackend();
 
+      await _resumeShowIfLaunchedForIt();
+
       _navigateToDashboard();
     } catch (e) {
       if (mounted) {
@@ -70,6 +74,21 @@ class _AudioInitSplashScreenState extends ConsumerState<AudioInitSplashScreen> {
     }
   }
 
+  /// 감시 프로그램이 다시 띄웠거나 로그인 자동 실행이면 공연을 이어 간다(rust api::show). 멈춘 위치부터
+  /// 다시 틀고, 이어 갈 수 없으면 첫 방 테마로 시작한다. 사람이 켰으면 대기한다.
+  Future<void> _resumeShowIfLaunchedForIt() async {
+    final resume = shouldResumeShow(
+      args: ref.read(launchArgsProvider),
+      resumeOnLogon: await loadAutoResumeOnLogon(),
+    );
+    if (!resume) return;
+    try {
+      await show_api.apiResumeShow();
+    } catch (e) {
+      // 이어 가지 못해도 앱은 계속 쓴다(이유는 Rust가 앱 로그에 남긴다).
+    }
+  }
+
   void _navigateToDashboard() {
     if (!mounted) return;
     Navigator.of(context).pushReplacement(
```

`lib/features/settings/widgets/preferences_modal.dart`(변경분):
```diff
--- a/atmos_mixer_pro/lib/features/settings/widgets/preferences_modal.dart
+++ b/atmos_mixer_pro/lib/features/settings/widgets/preferences_modal.dart
@@ -1,5 +1,7 @@
+import 'dart:async';
 import 'dart:io';
 import 'package:atmos_mixer_pro/core/state/engine_resync.dart';
+import 'package:atmos_mixer_pro/core/state/launch_mode.dart';
 import 'package:flutter/material.dart';
 import 'package:flutter_riverpod/flutter_riverpod.dart';
 import 'package:atmos_mixer_pro/core/theme/colors.dart';
@@ -26,6 +28,8 @@ class _PreferencesModalState extends ConsumerState<PreferencesModal>
   String _selectedDriverType = 'WASAPI';
   bool _isDeviceManuallyChanged = false;
   bool _isScanning = false;
+  // 이 컴퓨터의 운영 설정이라 config.json이 아니라 환경설정(SharedPreferences)에 둔다.
+  bool _autoResumeOnLogon = true;
   final Map<String, int> _trackChannels = {};
 
   String _getDriverType(String? deviceName) {
@@ -67,6 +71,9 @@ class _PreferencesModalState extends ConsumerState<PreferencesModal>
   void initState() {
     super.initState();
     _tabController = TabController(length: 2, vsync: this);
+    loadAutoResumeOnLogon().then((value) {
+      if (mounted) setState(() => _autoResumeOnLogon = value);
+    });
     // clone config for editing
     final currentConfig = ref.read(configProvider);
     _tempConfig = currentConfig != null
@@ -378,6 +385,7 @@ oscWhitelist: _tempConfig.oscWhitelist,
       globalTrajectory: _tempConfig.globalTrajectory,
       roomZones: _tempConfig.roomZones,
     );
+    unawaited(saveAutoResumeOnLogon(_autoResumeOnLogon));
     ref.read(configProvider.notifier).saveConfig(finalConfig);
     Navigator.of(context).pop();
   }
@@ -1171,6 +1179,38 @@ oscWhitelist: _tempConfig.oscWhitelist,
 
 
 
+        const SizedBox(height: 16),
+        Row(
+          children: [
+            const SizedBox(
+              width: 120,
+              child: Text(
+                '로그인할 때\n공연 자동 시작',
+                style: TextStyle(
+                  color: Colors.white70,
+                  fontWeight: FontWeight.bold,
+                  fontSize: 14,
+                ),
+              ),
+            ),
+            Switch(
+              value: _autoResumeOnLogon,
+              activeTrackColor: AppColors.primaryNeon.withValues(alpha: 0.5),
+              activeThumbColor: AppColors.primaryNeon,
+              onChanged: (val) => setState(() => _autoResumeOnLogon = val),
+            ),
+            const SizedBox(width: 8),
+            Text(
+              _autoResumeOnLogon ? 'On' : 'Off',
+              style: TextStyle(
+                color: _autoResumeOnLogon
+                    ? AppColors.primaryNeon
+                    : Colors.white54,
+                fontWeight: FontWeight.bold,
+              ),
+            ),
+          ],
+        ),
         const SizedBox(height: 24),
         const Text(
           '출력 채널 구성 (Channel Config)',
```

`integration_test/app_flow_test.dart`(변경분):
```diff
--- a/atmos_mixer_pro/integration_test/app_flow_test.dart
+++ b/atmos_mixer_pro/integration_test/app_flow_test.dart
@@ -42,7 +42,7 @@ void main() {
     final cwd = Directory.current;
     Directory.current = fixture.root;
     addTearDown(() => Directory.current = cwd);
-    await app.main();
+    await app.main(const []);
     await pumpUntil(tester, () => find.byType(DashboardScreen).evaluate().isNotEmpty,
         timeout: const Duration(seconds: 30), reason: '0단계: 30초 안에 대시보드가 뜨지 않았다');
     final container = ProviderScope.containerOf(tester.element(find.byType(app.AtmosMixerProApp)));
```

- [ ] **Step 4: 통과를 확인한다**

  Run(`atmos_mixer_pro/`에서): `flutter analyze` → 새 경고 0(기존 5건은 그대로). `flutter test` → 통과.
  빌드·장치 테스트 전에 Global Constraints의 두 가지(다른 Atmos 앱 없음, 디스크 3GB 이상)를 확인한다.

- [ ] **Step 5: 체크포인트** — `git status`·`git diff --stat`. 커밋은 사용자 요청 시.

---

### Task 11: installer와 CI

**상태:** 구현 파일·시험 있음(코드는 작업 트리의 파일을 옮겼다). 정적 확인만 했다. 이 Mac에서는 Windows 빌드를 검증할 수 없다. 첫 PR 빌드와 Windows 현장 점검표가 검증이다.

**Files:**
- Modify: `windows/installer.iss`, `../.github/workflows/build_release.yml`

**동작(Main 보고):** 바로가기·Run 키·설치 직후 실행이 감시 exe를 띄운다(Run 키에는 `--logon`). `[UninstallRun]`과 `PrepareToInstall`은 감시를 먼저, 그다음 앱을 끝낸다. CI: `pull_request`, `concurrency`, 감시 테스트(두 OS), Windows Release 폴더에 감시 exe 복사.

- [ ] **Step 1: 구현** — `windows/installer.iss`(변경분)

```diff
--- a/atmos_mixer_pro/windows/installer.iss
+++ b/atmos_mixer_pro/windows/installer.iss
@@ -1,6 +1,9 @@
 #define MyAppName "Atmos Mixer Pro"
 #define MyAppPublisher "Atmos"
 #define MyAppExeName "atmos_mixer_pro.exe"
+; The supervisor relaunches the app after a crash or a hang. Shortcuts, the logon Run key and the
+; post-install launch start the supervisor, which then starts the app.
+#define SupervisorExeName "atmos_supervisor.exe"
 ; The version is read from the built exe. Flutter writes pubspec.yaml's version into the exe's
 ; ProductVersion, so the installer always matches the app. The path uses the same base as [Files].
 #define MyAppExePath AddBackslash(SourcePath) + "build\windows\x64\runner\Release\" + MyAppExeName
@@ -31,14 +34,37 @@ Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{
 
 [Files]
 Source: "build\windows\x64\runner\Release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
+; The supervisor is copied next to the app by the build (CI copies it into the Release folder).
 Source: "build\windows\x64\runner\Release\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
 
 [Icons]
-Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
-Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon
+Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#SupervisorExeName}"; IconFilename: "{app}\{#MyAppExeName}"
+Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#SupervisorExeName}"; IconFilename: "{app}\{#MyAppExeName}"; Tasks: desktopicon
 
 [Registry]
-Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "AtmosMixerPro"; ValueData: """{app}\{#MyAppExeName}"""; Flags: uninsdeletevalue
+Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "AtmosMixerPro"; ValueData: """{app}\{#SupervisorExeName}"" --logon"; Flags: uninsdeletevalue
 
 [Run]
-Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
+Filename: "{app}\{#SupervisorExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
+
+[UninstallRun]
+; Stop the supervisor first. If the app is stopped first, the supervisor treats it as a crash and starts it again.
+Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM {#SupervisorExeName}"; Flags: runhidden; RunOnceId: "StopSupervisor"
+Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM {#MyAppExeName}"; Flags: runhidden; RunOnceId: "StopApp"
+
+[Code]
+// Installing over a running show (an upgrade): stop the supervisor before the app, for the same
+// reason as at uninstall. Otherwise the supervisor starts the app again and its files stay locked.
+procedure StopAtmos();
+var
+  ResultCode: Integer;
+begin
+  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#SupervisorExeName}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
+  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#MyAppExeName}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
+end;
+
+function PrepareToInstall(var NeedsRestart: Boolean): String;
+begin
+  StopAtmos();
+  Result := '';
+end;
```

`.github/workflows/build_release.yml`(변경분):
```diff
--- a/.github/workflows/build_release.yml
+++ b/.github/workflows/build_release.yml
@@ -11,8 +11,16 @@ on:
   push:
     tags:
       - 'v*'
+  # PR마다 Windows·macOS 빌드를 돌린다. 릴리스는 태그에서만 한다.
+  pull_request:
+    branches: [main]
   workflow_dispatch:
 
+# 같은 PR에 커밋이 또 올라오면 앞선 빌드를 취소한다.
+concurrency:
+  group: ${{ github.workflow }}-${{ github.ref }}
+  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
+
 jobs:
   build:
     strategy:
@@ -42,7 +50,13 @@ jobs:
       - name: Cache Rust dependencies
         uses: Swatinem/rust-cache@v2
         with:
-          workspaces: atmos_mixer_pro/rust
+          workspaces: |
+            atmos_mixer_pro/rust
+            atmos_mixer_pro/supervisor
+
+      # 감시 프로그램(충돌·멈춤 뒤 앱을 다시 띄움). Windows에서도 돌려 넘겨받기(프로세스 핸들) 경로를 확인한다.
+      - name: Supervisor tests
+        run: cargo test --manifest-path atmos_mixer_pro/supervisor/Cargo.toml
         
       - name: Set up Flutter
         uses: subosito/flutter-action@v2
@@ -96,6 +110,14 @@ jobs:
       - name: Build
         run: ${{ matrix.build_cmd }}
         working-directory: atmos_mixer_pro
+
+      # 감시 프로그램을 앱 옆에 둔다. installer의 바로가기·로그인 자동 실행이 이것을 띄운다.
+      - name: Add supervisor to Release (Windows)
+        if: matrix.os == 'windows-latest'
+        shell: pwsh
+        run: |
+          cargo build --release --manifest-path atmos_mixer_pro/supervisor/Cargo.toml
+          Copy-Item atmos_mixer_pro/supervisor/target/release/atmos_supervisor.exe atmos_mixer_pro/build/windows/x64/runner/Release/
         
       - name: Archive Release (Windows)
         if: matrix.os == 'windows-latest'
```

- [ ] **Step 2: 정적 확인** — 공연 중 새 버전 설치(Review Focus 5): `PrepareToInstall`이 감시를 먼저, 앱을 그다음에 끝내는지 스크립트를 읽어 확인한다. 그렇지 않으면 감시가 앱을 다시 띄워 파일이 잠긴다. 현장 점검표 2절의 "공연 중 새 버전 설치" 항목이 실제 확인이다.

- [ ] **Step 3: CI 확인** — PR을 올려 Windows·macOS 빌드가 자동으로 도는지, Windows 감시 빌드·복사 단계가 통과하는지 본다(새 PR 트리거의 첫 Windows 빌드는 `cpal` 0.16+ASIO, `rusqlite` bundled, `windows` 크레이트 새 기능이 Windows에서 컴파일되는 첫 확인이기도 하다).

- [ ] **Step 4: 체크포인트** — `git status`·`git diff --stat`. 커밋은 사용자 요청 시. 방화벽 규칙과 WebView2 런타임은 "사용자 결정이 필요한 것"이라 여기서 구현하지 않는다.

---

### Task 12: 끝까지 확인·인계

**상태:** Main의 실제 앱 확인(릴리스 빌드, 소리 없음) 12개 통과 보고. 남은 확인은 사용자가 옆에 있어야 하는 소리 시나리오(아래 스크립트). 점검표·HANDOFF는 완료(Sub).

**Files:**
- Create(Main): `tool/supervisor_e2e_macos.sh`(사람이 옆에서 돌리는 대화형 스크립트)
- Create(Sub, 완료): `docs/WINDOWS_FIELD_CHECKLIST.md`
- Modify(Sub, 완료): `docs/HANDOFF.md`

- [ ] **Step 1: macOS 끝까지 확인 스크립트** (Main)

```bash
#!/usr/bin/env bash
# 감시 프로그램 끝까지 확인(macOS, 사람이 옆에서 돌린다). 실제 앱을 감시 프로그램으로 띄워
# 강제 종료·일시 정지·두 번째 감시·앱 직접 두 번째 실행·넘겨받기·창 닫기를 차례로 본다.
# 판단 시간은 짧게 준다(멈춤 10초, 첫 하트비트 60초, 오디오 멈춤 30초). 실제 config로 소리가 난다.
#
# 사용법(atmos_mixer_pro 폴더에서): tool/supervisor_e2e_macos.sh [앱 실행 파일]
#   기본 앱: build/macos/Build/Products/Release/atmos_mixer_pro.app/Contents/MacOS/atmos_mixer_pro
#   먼저 `flutter build macos --release`로 앱을 빌드한다.
set -u
cd "$(dirname "$0")/.." || exit 1

APP="${1:-build/macos/Build/Products/Release/atmos_mixer_pro.app/Contents/MacOS/atmos_mixer_pro}"
LOG_DIR="${TMPDIR:-/tmp}"
LOG_DIR="${LOG_DIR%/}/atmos_mixer_pro_logs"
SUP=supervisor/target/release/atmos_supervisor
SUP_ARGS=(--app "$APP" --hang-secs 10 --first-heartbeat-secs 60 --audio-stall-secs 30)
APP_LOG="$LOG_DIR/atmos_mixer_pro.log"
SUP_LOG="$LOG_DIR/supervisor.log"
PASS=0
FAIL=0

ok() { echo "  통과: $1"; PASS=$((PASS + 1)); }
bad() { echo "  실패: $1"; FAIL=$((FAIL + 1)); }
alive() { kill -0 "$1" 2>/dev/null; }
gone() { ! alive "$1"; }
app_pid() { tr -d '[:space:]' <"$LOG_DIR/app.pid" 2>/dev/null; }
args_of() { ps -o args= -p "$1" 2>/dev/null; }
# 명령줄이 앱 실행 파일로 시작하는 프로세스만 센다(감시의 명령줄에도 --app 경로가 들어 있다).
app_count() { pgrep -f '^[^ ]*Contents/MacOS/atmos_mixer_pro( |$)' | wc -l | tr -d ' '; }
size_of() { stat -f%z "$1" 2>/dev/null || echo 0; }
# [secs]초 안에 명령이 성공할 때까지 1초마다 다시 본다.
wait_for() {
  local secs=$1
  shift
  local end=$((SECONDS + secs))
  until "$@"; do
    [ "$SECONDS" -ge "$end" ] && return 1
    sleep 1
  done
}
app_running() { local p; p=$(app_pid); [ -n "$p" ] && alive "$p"; }
heartbeat_from() { grep -q "^pid=$1\$" "$LOG_DIR/heartbeat" 2>/dev/null; }
new_app_since() { local p; p=$(app_pid); [ -n "$p" ] && [ "$p" != "$1" ] && alive "$p"; }
# [file]의 [offset] 바이트 뒤에 [text]가 나왔나
logged_since() { tail -c +"$(($2 + 1))" "$1" 2>/dev/null | grep -q "$3"; }

cleanup() {
  [ -n "${SUP_PID:-}" ] && alive "$SUP_PID" && kill "$SUP_PID" 2>/dev/null
  local p
  p=$(app_pid)
  [ -n "$p" ] && alive "$p" && kill "$p" 2>/dev/null
}
trap cleanup EXIT

[ -x "$APP" ] || { echo "앱이 없다: $APP — 먼저 flutter build macos --release"; exit 1; }
if [ "$(app_count)" != "0" ]; then
  echo "다른 Atmos 앱이 떠 있다. 닫고 다시 실행한다."
  exit 1
fi
cargo build --release --manifest-path supervisor/Cargo.toml || exit 1
mkdir -p "$LOG_DIR"

echo "== 0. 감시가 앱을 띄운다"
SUP_OFF=$(size_of "$SUP_LOG")
"$SUP" "${SUP_ARGS[@]}" &
SUP_PID=$!
if wait_for 60 app_running && wait_for 90 heartbeat_from "$(app_pid)"; then
  ok "앱이 떴고 하트비트가 온다(pid $(app_pid), 인자: $(args_of "$(app_pid)"))"
else
  bad "앱이 뜨지 않았거나 하트비트가 오지 않았다"
  exit 1
fi
echo "   대시보드에서 루프 트랙 하나를 재생한다(소리가 나는지 듣는다). 준비되면 Enter."
read -r _
sleep 6 # 공연 상태가 한 번 이상 저장되게(5초 간격)

echo "== 1. 앱 강제 종료(kill -9) → --auto-relaunched로 다시 뜨고 멈춘 위치부터 이어 간다"
P=$(app_pid)
APP_OFF=$(size_of "$APP_LOG")
kill -9 "$P"
if wait_for 30 new_app_since "$P"; then
  NEW=$(app_pid)
  case "$(args_of "$NEW")" in
    *--auto-relaunched*) ok "다시 떴다(pid $NEW, 인자: $(args_of "$NEW"))" ;;
    *) bad "다시 뜬 앱의 인자에 --auto-relaunched가 없다: $(args_of "$NEW")" ;;
  esac
  if wait_for 90 logged_since "$APP_LOG" "$APP_OFF" "공연 이어 가기"; then
    ok "앱 로그: $(tail -c +"$((APP_OFF + 1))" "$APP_LOG" | grep "공연 이어 가기" | tail -1)"
  else
    bad "앱 로그에 이어 가기 결과가 없다"
  fi
else
  bad "강제 종료한 앱이 30초 안에 다시 뜨지 않았다"
fi
echo "   소리가 멈춘 위치 근처부터 다시 나는지 듣는다. 확인했으면 Enter."
read -r _

echo "== 2. 앱 일시 정지(kill -STOP) → 멈춤 판단(10초) 뒤 강제 종료되고 다시 뜬다"
wait_for 90 heartbeat_from "$(app_pid)"
P=$(app_pid)
kill -STOP "$P"
if wait_for 40 new_app_since "$P"; then
  ok "멈춘 앱 대신 새 앱이 떴다(pid $(app_pid))"
  logged_since "$SUP_LOG" "$SUP_OFF" "UI 멈춤" && ok "감시 기록에 UI 멈춤" || bad "감시 기록에 UI 멈춤이 없다"
else
  bad "멈춘 앱을 40초 안에 다시 띄우지 않았다"
  kill -CONT "$P" 2>/dev/null
fi
wait_for 90 heartbeat_from "$(app_pid)"

echo "== 3. 감시를 하나 더 실행하면 바로 끝나고, 앱은 하나만 남는다"
"$SUP" "${SUP_ARGS[@]}"
RC=$?
[ "$RC" = "0" ] && alive "$SUP_PID" && ok "두 번째 감시가 끝났고 첫 감시는 돈다" || bad "두 번째 감시 종료 코드 $RC"
sleep 10
[ "$(app_count)" = "1" ] && ok "앱은 하나다" || bad "앱이 $(app_count)개다"

echo "== 4. 앱을 직접 하나 더 실행하면 종료 코드 3으로 바로 끝난다"
"$APP" >/dev/null 2>&1 &
DUP=$!
if wait_for 30 gone "$DUP"; then
  wait "$DUP"
  RC=$?
  [ "$RC" = "3" ] && ok "중복 실행이 종료 코드 3으로 끝났다" || bad "중복 실행 종료 코드 $RC"
else
  bad "직접 실행한 두 번째 앱이 30초 안에 끝나지 않았다"
  kill "$DUP" 2>/dev/null
fi

echo "== 5. 감시만 끝낸 뒤 감시를 다시 띄우면 떠 있는 앱을 넘겨받는다"
P=$(app_pid)
kill "$SUP_PID"
wait "$SUP_PID" 2>/dev/null
alive "$P" && ok "감시가 끝나도 앱은 돈다" || bad "감시를 끝냈더니 앱도 끝났다"
SUP_OFF=$(size_of "$SUP_LOG")
"$SUP" "${SUP_ARGS[@]}" &
SUP_PID=$!
if wait_for 20 logged_since "$SUP_LOG" "$SUP_OFF" "넘겨받았다(pid $P)"; then
  ok "떠 있는 앱을 넘겨받았다(pid $P)"
else
  bad "떠 있는 앱을 넘겨받지 않았다"
fi
sleep 3
[ "$(app_count)" = "1" ] && ok "새로 띄우지 않았다" || bad "앱이 $(app_count)개다"

echo "== 6. 창을 닫으면 다시 뜨지 않고 감시도 끝난다"
echo "   앱 창의 닫기 버튼(빨간 원)을 누른다(⌘Q가 아니라 창 닫기). 60초 기다린다."
P=$(app_pid)
if wait_for 60 gone "$P"; then
  ok "앱이 닫혔다"
  wait_for 15 gone "$SUP_PID" && ok "감시도 끝났다" || bad "앱을 닫았는데 감시가 끝나지 않았다"
  sleep 5
  [ "$(app_count)" = "0" ] && ok "다시 뜨지 않았다" || bad "닫은 앱이 다시 떴다"
else
  bad "60초 안에 창이 닫히지 않았다"
fi

echo
echo "결과: 통과 $PASS, 실패 $FAIL"
echo "감시 기록: $SUP_LOG"
echo "앱 로그: $APP_LOG"
[ "$FAIL" = "0" ]
```

  - 소리 없는 실제 앱 확인(Main 보고, 릴리스 빌드) 12개 통과: 시작 관문 Proceed, 하트비트 `ui_ms` 변화, 엔진 켜짐과 오디오 콜백 변화, `show_state.json` 저장, 직접 두 번째 실행은 종료 코드 3, 두 번째 감시는 바로 끝나고 앱은 하나, 감시를 끝내도 앱은 계속 돎, 감시를 다시 띄우면 넘겨받기, 넘겨받은 앱이 10초 뒤에도 그대로, 정리 후 프로세스 0.
  - `integration_test/app_flow_test.dart` 0~6단계 통과(Main 보고, 2분 35초, 재시작 이벤트 뒤 재개 139ms).
  - 소리가 나는 시나리오(`kill -9` 뒤 이어 가기, `kill -STOP`, 창 닫기)는 사용자가 옆에 있어야 한다. 위 스크립트로 돌린다.
  - 실행 전: 다른 Atmos 앱이 없어야 하고 디스크 여유가 3GB 이상이어야 한다. 실제 장치를 쓰므로 사용자가 앱을 띄워 둔 상태면 먼저 묻는다.

- [ ] **Step 2: Windows 현장 점검표** (Sub, 완료) — `docs/WINDOWS_FIELD_CHECKLIST.md`. 스펙 7절의 목록과 Windows 동일 동작 점검 항목(3D 방 뷰어, ASIO, 방화벽, 로그, 메뉴 등)을 Windows PC 앞에서 순서대로 따라 하는 형태로 담았다. 공연 중 새 버전 설치(업그레이드) 항목을 2절에 둔다: 감시가 먼저 꺼지는지, 앱이 다시 뜨지 않고 설치가 끝나는지.

- [ ] **Step 3: HANDOFF 갱신** (Sub, 완료) — "남은 일" 3번의 진행 상태, 3D 뷰어 Windows 구현(사용자 결정: 필수), `v1.1.3` 태그 질문, Windows `cargo test` 가드, Windows 로더.

- [ ] **Step 4: 체크포인트** — `git status`·`git diff --stat`. 커밋은 사용자 요청 시. 커밋에서 뺄 것: `.DS_Store`, `.claude/agents/*`, `rust/.ua/`, `rust/data/`.
