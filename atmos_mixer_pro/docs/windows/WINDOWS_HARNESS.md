# Windows 개발 PC 하네스

- 작성 2026-10-07(macOS의 Sub 세션). 기준: `main` 1907e73 이후(PR #12 — Windows에서 exe 옆 Rust dll을 직접 연다).
- 대상: Windows PC에서 개발 모드로 일하는 Claude Code 세션(아래 "Windows Claude")과 그 옆의 사용자. 사람이 먼저 읽는 짧은 안내와 시작 프롬프트는 [`WINDOWS_QUICKSTART.md`](WINDOWS_QUICKSTART.md)에 있다.
- 목적: 현장(전시 운영) PC는 Windows다. macOS에서 만든 기능이 Windows에서도 똑같이 동작하는지 이 PC에서 빌드·자동 테스트·현장 점검으로 확인하고, Windows에만 필요한 작업(3D 방 뷰어 등)을 한다.
- 이 문서와 `tool/windows/*.ps1`은 macOS에서 코드를 읽고 쓴 것이다. **Windows에서 실행해 본 적이 없다.** 명령이 틀리면 고쳐 쓰고, 고친 내용은 W8로 이 문서와 스크립트에 되돌린다.
- 표기: "HANDOFF n"은 `docs/HANDOFF.md` "남은 일" n번, "HANDOFF ⚠️ n"은 같은 문서 "⚠️ Windows 미검증" n번이다. "점검표 n절"은 `docs/WINDOWS_FIELD_CHECKLIST.md`의 n절이다.

## 0. 지금 할 일

1. 이 문서를 끝까지 읽는다. 저장소 루트 `CLAUDE.md`, `atmos_mixer_pro/docs/HANDOFF.md`, `atmos_mixer_pro/docs/WINDOWS_FIELD_CHECKLIST.md`(아래 "점검표")도 읽는다.
2. `D:\dev\harness\PROGRESS.md`가 있으면 거기 적힌 "다음 할 일"부터, 없으면 P0부터 한다.
3. 단계는 5절의 순서대로 한다. 게이트를 통과해야 다음으로 간다. 게이트가 실패하면 원인을 찾아 고치고, 사용자 조치가 필요하면 정확히 요청하고 기다린다.
4. 게이트마다 PROGRESS.md를 갱신하고 사용자에게 한국어로 짧게 보고한다(9.6 형식).
5. 사용자가 할 일(UAC 승인, 라이선스 동의, 시스템 설정, 장치 연결, 소리 확인, 재부팅·로그아웃)은 어디서 무엇을 누르는지까지 적어 요청한다.

## 1. 구조

### 1.1 전체 흐름

```
macOS                          GitHub (Apejean/Atmos)                 Windows 개발 PC
Main 세션: macOS 코드·측정  ──PR──▶ main ◀──PR (win/*)──  Windows Claude: 빌드·테스트·점검·Windows 작업
Sub 세션: 병합·문서·검토   ◀── PR 본문·댓글, HANDOFF.md ──▶  사용자: UAC·장치·청음·결정
                               Actions: PR마다 Windows·macOS 빌드 → zip 아티팩트(P5)
```

Windows Claude는 Mac 세션에 직접 말할 수 없다. 오가는 길은 GitHub(PR·댓글·문서)와 사용자뿐이다.

### 1.2 층

| 층 | 위치 | 하는 일 | 저장소에 |
|---|---|---|---|
| 규칙 | `%USERPROFILE%\.claude\CLAUDE.md` | 이 PC 전용 규칙(9.1). 이 PC의 모든 세션에 자동으로 실린다 | 아니오 |
| 규칙 | `D:\dev\Atmos\CLAUDE.md` | 저장소 공통 규칙(DSP 3법칙, 코딩 규칙) | 예 |
| 권한 | `D:\dev\Atmos\.claude\settings.local.json` | 허용·확인·금지 명령, 저장소 밖 하네스 폴더 접근(9.2) | 아니오 |
| 절차 | `docs/windows/WINDOWS_HARNESS.md`(이 문서), `WINDOWS_QUICKSTART.md` | 단계·게이트·명령 | 예 |
| 실행 | `tool/windows/*.ps1` | 환경·점검·실행 기록·아티팩트·백업·로그·자원 기록(10절) | 예 |
| 상태 | `D:\dev\harness\PROGRESS.md` | 단계·게이트 상태, 다음 할 일, 재개 지점. 세션 인계의 기준(9.3) | 아니오 |
| 결과 | `D:\dev\harness\RESULTS.md` | 점검표 항목별 결과와 증거(9.4) | 아니오(요약은 결과 PR로) |
| 증거 | `D:\dev\logs\<날짜>\` | run.ps1의 명령 출력, collect-logs.ps1 묶음, monitor CSV | 아니오 |
| 협업 | GitHub PR(`win/*`), HANDOFF.md, 점검표 | 결과 공유, Mac 세션 요청 | 예 |

### 1.3 폴더

```
C:\Users\<사용자>\.claude\CLAUDE.md          이 PC 규칙(9.1)
D:\dev\
  Atmos\                                     저장소. P3 이후 Claude는 여기서 연다
    CLAUDE.md
    .claude\settings.local.json              권한(9.2, 커밋하지 않음)
    atmos_mixer_pro\                         Flutter 앱
      rust\  supervisor\  windows\  integration_test\  test\
      tool\windows\*.ps1                     하네스 스크립트(10절)
      docs\windows\                          이 문서, QUICKSTART
      build\windows\x64\runner\Debug\ Release\
  harness\  PROGRESS.md  RESULTS.md  monitor.stop
  logs\<yyyy-MM-dd>\<게이트>_<HHmmss>.log    증거
  artifacts\<run id>\app\  manifest.txt      CI zip을 푼 곳
  backups\<yyyyMMdd-HHmmss>-<라벨>\           앱 데이터 백업
  downloads\                                 설치 파일
  flutter\  pub-cache\  rust\rustup\  rust\cargo\  LLVM\  sdk\asiosdk\  InnoSetup6\  Git\
D:\VS\2022\Community  D:\VS\Shared  D:\VS\Cache   Visual Studio
```

## 2. 역할과 협업

| 누구 | 하는 일 | 하지 않는 일 |
|---|---|---|
| Windows Claude | 개발 환경 구축, 빌드·자동 테스트, CI zip 검사, 점검표 진행·기록, Windows 전용 작업(5절 P7), 결과 PR | `main`에 직접 푸시, 사용자 지시 없는 병합, macOS 동작을 바꾸는 변경, 시스템·보안 설정 변경 |
| 사용자 | UAC 승인, 라이선스 동의, 시스템 설정(개발자 모드·Defender·방화벽·전원), gh 로그인, 장치 연결·청음, 재부팅·로그인 시험, 결정(11절) | |
| Mac Main 세션 | macOS 기능 개발·측정, macOS 회귀 확인 | |
| Mac Sub 세션 | PR 병합, 문서·인계 정리, 검토 | |

- Mac 세션에 부탁할 일은 PR 본문 "Mac 세션 확인 요청" 절이나 PR 댓글로 남기고, 사용자에게 "Mac에 전달해 주세요"라고 알린다.
- 큰 작업(W1 등)은 시작할 때 Draft PR을 먼저 열어 작업 중임을 알린다. 시작 전에 `gh pr list --repo Apejean/Atmos --state all --search "<주제>"`와 `git branch -r`로 같은 작업이 있는지 본다. 있으면 그 브랜치를 받아 Windows 검증만 한다.
- Mac에서 새 PR이 병합되면 `git pull --ff-only` 뒤 P4의 해당 게이트를 다시 돌려 Windows 회귀를 확인한다.

## 3. 규칙

### 3.1 말과 보고
- 사용자에게 보이는 답·보고·질문은 한국어. 코드·명령·커밋 메시지는 영어.
- 채널 번호는 사용자에게 CH1부터 말한다(내부 channel은 0부터, CH = 내부 + 1).
- 확인 등급을 구분해 말한다: ① CI 빌드 통과 ② 이 PC 빌드·자동 테스트 통과 ③ 이 PC 실행 확인(소리 없음) ④ 장치로 확인(소리) ⑤ 사람이 들어 확인. macOS에서 확인한 것을 Windows 보증으로 말하지 않는다.
- 실패는 출력 그대로(로그 경로 포함) 보고한다. 건너뛴 것은 건너뛰었다고, 추정은 추정이라고 말한다.

### 3.2 확정된 결정 — 다시 제안하지 않는다
- 크로스오버 **80Hz·LR24(24dB/oct)** 유지. 서브 저음이 메인 자리에서 들리는 것은 의도된 상태다. 120/150Hz, 48dB/oct는 제안하지 않는다.
- 현장 PC는 Windows다. 지금 되는 기능은 Windows에서도 같아야 하고, **3D 방 뷰어는 반드시 Windows에 들어간다.** 메뉴 형태·종료 경로·절전 방지와 로그 경로 구현 방식 같은 OS 고유 차이는 허용한다.
- 프로젝트는 오디오·도면을 `.atmos`와 같은 폴더(하위 폴더 가능)에 담아 옮긴다. 열 때 못 찾으면 그 폴더에서 같은 이름으로 다시 연결한다.
- 운용 흐름: ① macOS에서 바이노럴로 방·스피커를 설계하고 자동 계산 값(EQ·게인·딜레이·위상·리버브·초기반사)을 얻는다 → ② 프로젝트를 현장 Windows PC로 옮긴다 → ③ 오디오 인터페이스를 다시 스캔해 물리 채널(ADAT·Dante 포함)을 스피커에 연결하고 바이노럴을 끈 채 귀로 조정한다 → ④ 무인 운영(장시간 연속, 엔진 재시작 때 멈춘 위치부터 자동 재개, 다른 사람은 "테마 시작"만 누른다).
- 초기반사 "방 시뮬레이션"은 바이노럴을 켰을 때만 쓴다(현장에서는 실제 방이 반사를 만든다). 서브우퍼는 스피커 속성으로 방마다 하나이고, 베이스 매니지먼트는 채널 DSP 앞에서 가른다. 헤드폰 미리듣기는 스피커→청취 지점 전파(지연·거리 감쇠)를 흉내 낸다.
- ASIO 채널 이름 작업은 사용자가 보류했다. 시작하지 않는다.

### 3.3 코드
- 저장소 `CLAUDE.md`를 따른다: 오디오 스레드 3법칙(할당 금지·블로킹 금지·파라미터 보간), Rust 주석·문서 한국어, 운영 오디오 코드 `unwrap()` 금지, 외과적 변경, 단순함 우선, 검증 가능한 목표(실패하는 테스트 → 고침 → 통과).
- Windows 전용 코드는 `#[cfg(target_os = "windows")]`(Rust), `Platform.isWindows`(Dart)로 나눠 macOS 동작을 바꾸지 않는다. 바뀌게 되면 PR에 그 사실과 Mac 세션 확인 요청을 쓴다.
- 새 패키지는 macOS 빌드에 주는 영향(플러그인 등록 여부)을 PR에 적는다.
- `flutter analyze` 새 이슈 0, `cargo clippy` 새 경고 0. 기존 것(macOS에서 잰 현재 `main`: analyze 5건, Rust lib clippy 26건)은 건드리지 않고 목록만 남긴다.
- `rust/src/api/`를 바꾸면 `flutter_rust_bridge_codegen generate`(2.12.0)로 바인딩을 다시 만든다.
- `CLAUDE.md`, `.claude/agents/`, `.agents/`는 사용자가 요청할 때만 고친다.

### 3.4 안전
- 빌드·장치 시험 전에 `preflight.ps1`. Atmos가 실행 중이면 사용자에게 먼저 묻는다(사용자의 청음·장시간 확인일 수 있다).
- 소리가 나는 시험(재생 점검, `app_flow_test`의 −30dBFS 톤) 전에는 어떤 장치로 소리가 나는지 알리고 음량을 확인받는다.
- `%APPDATA%\com.example\atmos_mixer_pro`(설정·환경설정·공연 상태)와 `%TEMP%\atmos_mixer_pro_logs`는 사용자의 실제 데이터다. 바꾸는 시험 전에 `backup-appdata.ps1`. 되돌리기·삭제는 사용자 확인 뒤에 한다.
- 지우는 것은 이 세션이 만든 것만이다. `rust\target`, `build`, `pub-cache`, 사용자 캐시는 묻고 지운다.
- 시스템·보안 설정(개발자 모드, Defender 예외, 방화벽 규칙, LongPaths, 전원 계획, Windows Update, UAC 수준)은 사용자가 한다. Windows Claude는 어디서 무엇을 누르는지만 안내한다. 스크립트 실행 정책도 바꾸지 않는다(`-ExecutionPolicy Bypass`로 그 실행만).
- 라이선스·약관 동의(Visual Studio Community, Steinberg ASIO SDK)는 사용자에게 확인받는다.
- 감시 프로그램은 죽은 앱을 다시 띄운다. 앱을 끌 때는 `atmos_supervisor`를 먼저, `atmos_mixer_pro`를 그다음에 끈다(installer와 같은 순서). 대기 중인 앱이 충돌해 다시 뜨면 첫 방 테마로 시작해 소리가 날 수 있다.
- 재부팅·로그아웃이 필요한 시험 전에는 PROGRESS.md "재개 지점"을 먼저 적는다(세션이 끊긴다).
- 설치 파일을 시험하면 로그인 자동 실행(HKCU Run, 환경설정 "로그인할 때 공연 자동 시작" 기본 켜짐)이 남아 이 PC가 로그인할 때마다 공연을 시작한다. 시험이 끝나면 남길지 사용자에게 묻는다.

### 3.5 Git·GitHub
- 세션 시작 때 `git -C D:\dev\Atmos fetch --prune`, `git status`, `gh pr list --repo Apejean/Atmos`로 바뀐 것을 본다.
- `main`에 직접 커밋·푸시하지 않는다. 작업은 최신 `origin/main`에서 만든 `win/<주제>` 브랜치에서 한다.
- 커밋은 사용자가 요청했거나 이 문서의 단계가 정한 때만 한다. 메시지는 영어 Conventional Commits(`fix(windows): ...`, `test: ...`, `docs: ...`)에 Claude Code가 정한 공동 작성자 줄을 붙인다.
- PR은 `main` 대상, 본문은 한국어(9.5). 병합은 사용자가 지시할 때만, 머지 커밋으로 한다.
- 문서 변경은 그 기능 브랜치에 같이 넣는다. 충돌은 양쪽 내용을 살린다. 이미 `main`에 병합된 문서는 `origin/main` 쪽을 기준으로 한다.
- 커밋하지 않는 것: `D:\dev\logs`·`artifacts`·`backups`의 내용, `windows\Output\`, `windows\build\`, `.claude\settings.local.json`, 빌드 산출물, 측정용 임시 코드.

### 3.6 도구 쓰는 법
- 서브에이전트·워크플로(여러 에이전트)를 쓰지 않는다. 혼자 순서대로 한다(사용자 결정, 토큰 비용).
- 셸: PowerShell 도구가 있으면 그것을 쓴다. Git Bash뿐이면 Windows 명령·스크립트는 `MSYS_NO_PATHCONV=1 powershell -NoProfile -ExecutionPolicy Bypass -File <스크립트> ...`로 부르고, 경로는 `D:/dev/...`처럼 `/`로 쓰거나 작은따옴표로 감싼다. Git Bash는 따옴표 밖의 `\`를 지우고, `/v`·`/MIR` 같은 스위치를 경로로 바꾼다.
- 증거가 필요한 명령은 `run.ps1`로 돌린다(10절). 출력 전체가 `D:\dev\logs\<날짜>\<게이트>_<시각>.log`에 남는다.
- 8분 넘게 걸릴 수 있는 명령(첫 `cargo test`, 첫 `flutter build`, 통합 테스트)은 백그라운드로 돌리고 로그 파일로 확인한다(도구 한 번의 제한 시간은 10분).
- 앱을 오래 띄워 두는 시험(감시·장시간·로그인)은 사용자가 바로가기나 탐색기로 띄운다. Claude 셸의 자식으로 띄운 프로세스는 셸이 정리될 때 같이 꺼질 수 있다.
- `.mcp.json`의 agentmemory MCP는 Node.js가 있어야 뜬다. 이 PC의 기억은 Mac과 공유되지 않으므로 결정과 인계는 PROGRESS.md와 GitHub에 남긴다. MCP 승인 창은 사용자 판단으로 거절해도 된다.

## 4. 앱이 쓰는 Windows 경로

| 무엇 | 경로 | 메모 |
|---|---|---|
| 앱 데이터 | `%APPDATA%\com.example\atmos_mixer_pro\` | `config.json`, `shared_preferences.json`, 공연 상태. `windows/runner/Runner.rc`의 CompanyName `com.example`·ProductName `atmos_mixer_pro`에서 정해진다. 개발 빌드·CI zip·설치본이 **같은 폴더**를 쓴다 |
| 앱 로그·신호 파일 | `%TEMP%\atmos_mixer_pro_logs\` | `atmos_mixer_pro.log`(밀린 `.1`~`.4`), `supervisor.log`, `app.lock`, `app.pid`, `heartbeat`, `clean_exit`, `supervisor.lock` |
| 설치 위치 | `C:\Program Files\Atmos Mixer Pro\` | `installer.iss`의 `{autopf}` |
| 바로가기 | 시작 메뉴·바탕화면 "Atmos Mixer Pro" | `atmos_supervisor.exe`를 띄운다 |
| 로그인 자동 실행 | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`의 값 `AtmosMixerPro` | `"<설치 폴더>\atmos_supervisor.exe" --logon` |
| Debug 빌드 | `atmos_mixer_pro\build\windows\x64\runner\Debug\` | 감시 exe가 없어 앱이 혼자 뜬다 |
| Release 빌드 | `atmos_mixer_pro\build\windows\x64\runner\Release\` | CI처럼 감시 exe를 넣으면, 앱 exe를 직접 띄워도 감시로 넘어간다 |
| 설치 파일 | `atmos_mixer_pro\windows\Output\AtmosMixerPro_Setup.exe` | Inno Setup 기본 출력 위치 |
| 바탕화면(로그 내보내기) | `[Environment]::GetFolderPath('Desktop')` | OneDrive로 옮겨졌을 수 있다 |

## 5. 단계와 게이트

순서: P0 → P1 → P2 → P3(재시작) → P4 → P5 → P6-A → P7의 W2·W3 → P4 통합 테스트 다시 → P7의 W1(3D) → P6-E → P6-C → P6-D → P8. 결과 PR(P8)은 P6 묶음이 끝날 때마다 올린다.

표기: `run <게이트> [-Cwd <폴더>] "<명령>"`은 다음을 뜻한다(Cwd 기본값 `D:\dev\Atmos\atmos_mixer_pro`).

```
powershell -NoProfile -ExecutionPolicy Bypass -File D:/dev/Atmos/atmos_mixer_pro/tool/windows/run.ps1 -Gate <게이트> -Cwd <폴더> -Run "<명령>"
```

`<이름>.ps1 <인자>`는 `tool/windows/` 아래 스크립트를 같은 방식(`-File`)으로 부른다는 뜻이다. 대화형 PowerShell 블록은 먼저 `. D:\dev\Atmos\atmos_mixer_pro\tool\windows\env.ps1`을 점 소싱한다.

### P0 사용자 사전 준비 — Windows Claude는 확인만 (G0)

사용자가 QUICKSTART 1절에서 하는 일: 개발자 모드 켜기, Git for Windows(`D:\dev\Git`, `CLAUDE_CODE_GIT_BASH_PATH`), GitHub CLI 설치와 `gh auth login`·`gh auth setup-git`, Claude 설치·로그인, `D:\dev`에서 Claude 열기.

```powershell
git --version
gh --version
gh auth status
(Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock').AllowDevelopmentWithoutDevLicense   # 1
Get-PSDrive C, D | Select-Object Name, @{ n = 'FreeGB'; e = { [math]::Round($_.Free / 1GB, 1) } }                     # 권장 C 15GB, D 60GB 이상
```

G0: 위가 모두 정상. 빠진 것이 있으면 QUICKSTART 1절의 해당 줄을 사용자에게 다시 안내한다.

### P1 저장소 받기 (G1)

```powershell
New-Item -ItemType Directory -Force D:\dev | Out-Null
gh repo clone Apejean/Atmos D:\dev\Atmos                                  # 이미 있으면: git -C D:\dev\Atmos pull --ff-only
git -C D:\dev\Atmos merge-base --is-ancestor 1907e73 HEAD; $LASTEXITCODE  # 0
git config --global core.longpaths true
```

G1: 클론 완료, `main`에 1907e73 포함. 여기서부터 `tool/windows/` 스크립트를 쓸 수 있다.

### P2 개발 도구 설치 (G2)

도구는 D:에 둔다(C: 여유가 적다). **Visual Studio를 Rust보다 먼저** 설치한다(rustup-init은 VS가 없으면 C:에 VS 설치를 권한다). 대략 크기: VS C++ 8GB(D:)와 2~3GB(C:, Windows SDK 등), Flutter 3GB, Rust 1.5GB, LLVM 2GB, 빌드 산출물 10~20GB.

| 순서 | 내용 | 사용자 |
|---|---|---|
| 2-1 | 환경변수·폴더: `setup-env.ps1`(RUSTUP_HOME, CARGO_HOME, PUB_CACHE, LIBCLANG_PATH, CPAL_ASIO_DIR, PATH) | — |
| 2-2 | Visual Studio 2022 Community + "C++를 사용한 데스크톱 개발" | 라이선스 확인, UAC |
| 2-3 | Rust(MSVC) + clippy·rustfmt | — |
| 2-4 | LLVM 15.0.7(ASIO bindgen이 쓰는 libclang) | UAC |
| 2-5 | Steinberg ASIO SDK 2.3.4 | 라이선스 확인 |
| 2-6 | Flutter stable | — |
| 2-7 | WebView2 런타임 확인(W1에 필요) | 없으면 설치 확인 |
| 2-8 | Inno Setup 6 | UAC |
| 2-9 | (선택) Node.js LTS — agentmemory MCP용: `winget install --id OpenJS.NodeJS.LTS -e --source winget` | UAC |
| 2-10 | (선택) `cargo install flutter_rust_bridge_codegen --version 2.12.0 --locked` — `rust/src/api`를 바꿀 때만 | — |

2-2 Visual Studio. Flutter Windows 빌드는 Visual Studio 2022의 "C++를 사용한 데스크톱 개발" 워크로드와 그 기본 구성요소가 필요하다(Build Tools가 아니라 Visual Studio로 맞춘다). 설치 전에 사용자에게 Community 라이선스(조직 규모·매출 조건)로 이 PC에서 써도 되는지 확인받는다.

```powershell
winget install --id Microsoft.VisualStudio.2022.Community -e --source winget --accept-package-agreements --accept-source-agreements --override "--wait --passive --norestart --installPath D:\VS\2022\Community --path cache=D:\VS\Cache --path shared=D:\VS\Shared --add Microsoft.VisualStudio.Workload.NativeDesktop --includeRecommended"
```

- `--path cache=`·`shared=`는 이 PC에 VS를 처음 설치할 때만 적용된다.
- 이미 VS가 있으면 `doctor.ps1`의 Visual Studio 행을 보고 워크로드만 더한다: `& "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\setup.exe" modify --installPath "<설치 경로>" --add Microsoft.VisualStudio.Workload.NativeDesktop --includeRecommended --passive --norestart`

2-3 Rust(CI도 stable. `rust/.cargo/config.toml`이 Windows에서 `+crt-static`을 건다):

```powershell
Invoke-WebRequest https://win.rustup.rs/x86_64 -OutFile D:\dev\downloads\rustup-init.exe
D:\dev\downloads\rustup-init.exe -y --default-host x86_64-pc-windows-msvc --default-toolchain stable --profile minimal
. D:\dev\Atmos\atmos_mixer_pro\tool\windows\env.ps1
rustup component add clippy rustfmt
```

2-4 LLVM 15.0.7(CI는 `KyleMayes/install-llvm-action` 15.0):

```powershell
Invoke-WebRequest https://github.com/llvm/llvm-project/releases/download/llvmorg-15.0.7/LLVM-15.0.7-win64.exe -OutFile D:\dev\downloads\LLVM-15.0.7-win64.exe
Start-Process -FilePath D:\dev\downloads\LLVM-15.0.7-win64.exe -ArgumentList '/S', '/D=D:\dev\LLVM' -Verb RunAs -Wait
Test-Path D:\dev\LLVM\bin\libclang.dll
```

2-5 ASIO SDK(CI와 같은 파일. 내려받기 전에 사용자에게 Steinberg ASIO SDK 라이선스 동의를 확인받는다):

```powershell
Invoke-WebRequest https://download.steinberg.net/sdk_downloads/ASIO-SDK_2.3.4_2025-10-15.zip -OutFile D:\dev\downloads\asiosdk.zip
Expand-Archive D:\dev\downloads\asiosdk.zip -DestinationPath D:\dev\sdk\_asio -Force
$inner = Get-ChildItem D:\dev\sdk\_asio -Directory | Select-Object -First 1
Move-Item $inner.FullName D:\dev\sdk\asiosdk
Get-ChildItem D:\dev\sdk\asiosdk -Recurse -Filter asio.h | Select-Object -First 1 FullName
```

CI처럼 압축 안 첫 폴더를 `CPAL_ASIO_DIR`로 쓴다. 구조가 다르면 `common\asio.h`가 있는 SDK 최상위를 가리키게 맞춘다.

2-6 Flutter:

```powershell
git clone https://github.com/flutter/flutter.git -b stable D:\dev\flutter
. D:\dev\Atmos\atmos_mixer_pro\tool\windows\env.ps1
flutter --version            # 첫 실행에 Dart SDK를 내려받는다
flutter config --no-analytics
flutter doctor -v
```

- `pubspec.yaml`은 Dart 3.12.1 이상이 필요하다. CI는 그때의 최신 stable을 쓴다. CI와 버전이 달라 문제가 생기면 CI 로그에서 버전을 찾아 맞춘다: `gh run view <run id> --repo Apejean/Atmos --log | Select-String 'Flutter \d+\.\d+\.\d+' | Select-Object -First 1` → `git -C D:\dev\flutter checkout <버전>`.
- `flutter doctor`의 Android·Chrome 실패는 무시한다. Windows Version과 Visual Studio 항목만 통과하면 된다.

2-7 WebView2 런타임: Windows 11에는 보통 있다. `doctor.ps1`이 WARN이면 W1을 시작할 때 사용자 확인 뒤 `winget install --id Microsoft.EdgeWebView2Runtime -e --source winget`.

2-8 Inno Setup 6:

```powershell
winget install --id JRSoftware.InnoSetup -e --source winget --override "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- /DIR=D:\dev\InnoSetup6"
```

G2: `run G2_doctor "powershell -NoProfile -ExecutionPolicy Bypass -File D:\dev\Atmos\atmos_mixer_pro\tool\windows\doctor.ps1"` → `FAIL=0`(WARN 허용: WebView2·Inno·Node), `run G2_flutter_doctor "flutter doctor -v"` → Windows Version·Visual Studio 통과.

### P3 하네스 설치와 재시작 (G3)

1. `D:\dev\harness\PROGRESS.md`(9.3)와 `RESULTS.md`(9.4)를 만든다. RESULTS.md에는 점검표의 `[ ]` 항목을 "절-순번" ID(예: 2절 세 번째 항목 = `2-3`)로 옮기고 P6의 X 항목을 더한다.
2. 이 PC 규칙 `%USERPROFILE%\.claude\CLAUDE.md`(9.1)를 만든다. 파일이 이미 있으면 덮어쓰지 않고 사용자에게 보여 준 뒤 끝에 덧붙인다.
3. 권한 파일 `D:\dev\Atmos\.claude\settings.local.json`(9.2)를 만들고 `.claude/settings.local.json` 한 줄을 `D:\dev\Atmos\.git\info\exclude`에 더한다.
4. 사용자에게 요청: Claude를 트레이까지 **완전히 종료**하고 다시 열어 작업 폴더를 `D:\dev\Atmos`로 연 뒤 "PROGRESS 보고 이어서 진행해"라고 보낸다(새 환경변수와 규칙은 새로 띄운 프로그램에만 보인다).

G3(새 세션): 아래가 모두 `D:\dev` 아래를 가리키고, 새 세션이 9.1 규칙을 보고 있다고 한 줄로 보고한다.

```powershell
$env:CARGO_HOME; $env:PUB_CACHE; $env:LIBCLANG_PATH; (Get-Command flutter).Source; (Get-Command cargo).Source
```

### P4 빌드와 자동 테스트 (G4)

매 게이트 전에 `preflight.ps1`. 첫 실행은 오래 걸린다(`cargo test` 20~40분, `flutter build` 10~20분). `앱`은 `D:\dev\Atmos\atmos_mixer_pro`.

| 게이트 | Cwd | `-Run` | 통과 기준·기록 |
|---|---|---|---|
| G4-01_pub_get | 앱 | `flutter pub get` | exit 0 |
| G4-02_supervisor_test | 앱 | `cargo test --manifest-path supervisor/Cargo.toml` | 통과(CI와 같은 단계) |
| G4-03_cargo_check | 앱\rust | `cargo check` | exit 0(CI와 같은 단계) |
| G4-04_clippy | 앱\rust | `cargo clippy --all-targets` | 경고 목록. Windows에서만 나는 새 경고는 W6 후보 |
| G4-05_test_compile | 앱\rust | `cargo test --no-run` | exit 0이면 HANDOFF 12 확인 완료 |
| G4-06_cargo_test | 앱\rust | `cargo test -- --nocapture` | 실패 목록. macOS는 전부 통과하므로 실패는 Windows 차이(W6) |
| G4-07_analyze | 앱 | `flutter analyze` | 새 이슈 0(현재 `main` 기존 5건, PR #11 병합 뒤 0) |
| G4-08_flutter_test | 앱 | `flutter test` | macOS 163개 통과가 기준. 실패 목록(W6) |
| G4-09_build_debug | 앱 | `flutter build windows --debug` | `build\windows\x64\runner\Debug\atmos_mixer_pro.exe` |
| G4-10 | — | 아래 PowerShell(개발 실행·로더) | 실린 dll이 exe 옆 것 |
| G4-11_build_release | 앱 | `flutter build windows --release` | exit 0(CI와 같은 단계) |
| G4-12_supervisor_release | 앱 | `cargo build --release --manifest-path supervisor/Cargo.toml && copy /Y supervisor\target\release\atmos_supervisor.exe build\windows\x64\runner\Release\` | Release 폴더에 감시 exe(CI와 같음) |
| G4-13_deps | — | `deps.ps1 -Dir D:\dev\Atmos\atmos_mixer_pro\build\windows\x64\runner\Release` | VC++ 런타임 의존 기록(X1) |
| G4-14_it_relink | 앱 | `flutter test integration_test/project_media_relink_test.dart -d windows` | 통과, 아니면 실패 지점 기록 |
| G4-15_it_app_flow | 앱 | `flutter test integration_test/app_flow_test.dart -d windows` | W2 전: 0단계 `pgrep` 실패가 정상. W2 뒤: 0·1단계 통과, 2단계는 W1 전까지 실패. W1 뒤: 0~6단계 |

- G4-06·G4-08에서 Windows에서만 실패하는 테스트는 원인(경로 구분자, 줄바꿈, 장치, 시간, 권한)을 적는다.
- G4-14·G4-15는 기본 출력 장치로 소리가 난다(3.4). `rust\target\release`에 dll이 없어야 한다(preflight의 WARN).
- Debug 빌드의 Rust 엔진은 최적화가 덜 돼 끊길 수 있다. 소리 품질은 Release로 판단한다.

G4-10 개발 실행과 로더 확인(점검표 10절 로더, HANDOFF 13):

```powershell
. D:\dev\Atmos\atmos_mixer_pro\tool\windows\env.ps1
Set-Location $Atmos.App
New-Item -ItemType Directory -Force rust\target\release | Out-Null
Copy-Item build\windows\x64\runner\Debug\rust_lib_atmos_mixer_pro.dll rust\target\release\   # 옛 dll 자리에 미끼를 둔다(이 세션이 만든 파일)
Start-Process -FilePath build\windows\x64\runner\Debug\atmos_mixer_pro.exe -WorkingDirectory $Atmos.App
Start-Sleep -Seconds 20
(Get-Process -Name atmos_mixer_pro).Modules | Where-Object ModuleName -eq 'rust_lib_atmos_mixer_pro.dll' | Select-Object FileName
Get-Content (Join-Path $Atmos.AppLogs 'atmos_mixer_pro.log') -Tail 40
```

- 통과: FileName이 `...\build\windows\x64\runner\Debug\rust_lib_atmos_mixer_pro.dll`(미끼가 아님), 로그에 패닉·`ASIO Load Error`가 없음, 사용자가 화면에서 스플래시 → 대시보드를 확인(3D 화면은 스피너가 정상).
- 끝: `Stop-Process -Name atmos_mixer_pro`, 미끼 삭제 `Remove-Item rust\target\release\rust_lib_atmos_mixer_pro.dll`. RESULTS의 `10-3`에 적는다.

### P5 CI zip 검사 (Z)

CI("Build and Release")는 `main` 대상 PR마다 Windows·macOS를 빌드해 `atmos_mixer_pro_windows.zip`(Release 폴더 + 감시 exe)을 아티팩트로 올린다. `main`에 직접 들어간 커밋에서는 돌지 않으므로, `main`과 같은 내용은 마지막으로 병합된 PR의 실행에서 받는다(1907e73 = PR #12). 새로 필요하면 사용자가 GitHub Actions → Build and Release → Run workflow(`main`)로 돌린다. 아티팩트는 보관 기간(기본 90일)이 지나면 사라진다.

| 게이트 | 내용 | 통과 기준 |
|---|---|---|
| Z1 | `fetch-artifact.ps1 -Pr 12`(또는 `-RunId`, `-Branch`) | `D:\dev\artifacts\<run>\app`, `manifest.txt`에 MISS 없음 |
| Z2 | manifest의 exe 버전 | ProductVersion = pubspec 버전(지금 1.1.2), CompanyName `com.example` |
| Z3 | `deps.ps1 -Dir <app>` | VC++ 런타임 의존 목록(X1) |
| Z4 | 감시로 실행하고 정상 종료(아래) | 감시와 앱(`--supervised`)이 뜨고 heartbeat가 갱신, 창을 닫으면 둘 다 끝나고 다시 뜨지 않음 |
| Z5 | 설치 파일 만들기 | W3 전: 우회 / W3 뒤: `/DReleaseDir` |
| Z6 | (선택) 깨끗한 PC | VC++ 런타임이 없는 환경에서의 증상 |

Z4:

```powershell
. D:\dev\Atmos\atmos_mixer_pro\tool\windows\env.ps1
$app = 'D:\dev\artifacts\<run>\app'
powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $Atmos.Tools 'backup-appdata.ps1') -Label before-Z4
Start-Process -FilePath (Join-Path $app 'atmos_supervisor.exe') -WorkingDirectory $app
Start-Sleep -Seconds 20
Get-CimInstance Win32_Process -Filter "Name='atmos_mixer_pro.exe' OR Name='atmos_supervisor.exe'" | Select-Object Name, ProcessId, ParentProcessId, CommandLine | Format-List
(Get-Item (Join-Path $Atmos.AppLogs 'heartbeat')).LastWriteTime
Get-Content (Join-Path $Atmos.AppLogs 'supervisor.log') -Tail 30
```

- 방화벽 허용 창이 뜨면(이 exe 경로로 처음 실행) 무엇이 떴는지 기록하고 사용자에게 "취소"를 누르게 한다. 방화벽 점검(점검표 5절)은 설치본으로 한다.
- 사용자가 창을 X로 닫는다 → 몇 초 뒤 두 프로세스가 없고 `clean_exit`가 있으며 다시 뜨지 않는다(점검표 2절 "창 정상 닫기"와 같은 확인). 그 뒤 `collect-logs.ps1 -Label Z4 -Hours 1`.

Z5 설치 파일(HANDOFF ⚠️ 8):

- W3 전(코드 변경 없는 우회):
  ```powershell
  robocopy D:\dev\artifacts\<run>\app D:\dev\Atmos\atmos_mixer_pro\windows\build\windows\x64\runner\Release /MIR   # 종료 코드 0~7은 성공
  D:\dev\InnoSetup6\ISCC.exe D:\dev\Atmos\atmos_mixer_pro\windows\installer.iss
  ```
  → `atmos_mixer_pro\windows\Output\AtmosMixerPro_Setup.exe`. `windows\build`·`windows\Output`은 .gitignore에 있다.
- W3 뒤: `D:\dev\InnoSetup6\ISCC.exe /DReleaseDir=D:\dev\artifacts\<run>\app D:\dev\Atmos\atmos_mixer_pro\windows\installer.iss`
- 설치 자체의 점검은 P6-A(점검표 0절 "설치")에서 한다.

Z6 깨끗한 PC(선택): 개발 PC에는 Visual Studio가 VC++ 런타임을 깔아 두어 "런타임 없는 현장 PC"를 흉내 낼 수 없다. Windows 11 Pro 이상이면 사용자가 "Windows 샌드박스" 기능을 켜고(재부팅) 그 안에서 zip 폴더의 감시 exe를 실행해 본다. 결과로 W4(런타임 포함 방식)를 제안한다.

### P6 현장 점검표 진행

점검표의 각 항목을 RESULTS.md에 OK / 실패(증상) / 해당 없음 / 보류로 적고 등급(3.1)과 증거 경로를 붙인다.

| 묶음 | 점검표 | 준비 | 사람이 할 일 |
|---|---|---|---|
| A 장치 없이 | 0절(설치·WebView2·기동), 5절 방화벽 창, 6절 로그, 8절 메뉴·화면, 7절 현재 상태(W1 전), 2절 중 장치와 무관한 항목 | Z5 설치 파일, 이 PC 기본 출력(소리 작게) | 설치 승인, 방화벽 창 선택, 메뉴 조작, 리소스 모니터 조작 |
| B 자동 테스트 | 10절 | P4 | — |
| E 프로젝트 이식 | 9절 | macOS에서 저장한 `.atmos` + 오디오·도면 폴더(USB 등) | 파일 옮기기, 대화상자 버튼 누르기 |
| C 장치·청음 | 3절, 2절 ASIO 뽑기, 5절 OSC 수신, 7절(W1 뒤), 11절, X8 | 현장 오디오 인터페이스와 ASIO 드라이버, 스피커·헤드폰, 외부 OSC 장비 | 장치 연결·뽑기, 청음, 외부 OSC 보내기 |
| D 장시간·로그인·업그레이드 | 4절, 1절, 2절 업그레이드, X7 | 설치본, `monitor.ps1` | 로그아웃·로그인, 절전·깨우기, 관리자 `powercfg /requests` |

HANDOFF "⚠️ Windows 미검증" 1~14를 확인하는 곳:

| HANDOFF | 내용 | 확인하는 곳 |
|---|---|---|
| 1 | cpal 0.16(ASIO/WASAPI) | G4-03·G4-11 컴파일(CI 통과), 3절 동작 |
| 2 | 엔진 세대 가드 | 3절 재생과 정지(정지 뒤 소리가 남지 않음), ASIO 재시작 복원 |
| 3 | 통합 테스트의 기본 장치 | G4-15(WASAPI 기본 장치) |
| 4 | app_flow_test macOS 전용·SpeakerBridge | W2, W1, G4-15 |
| 5 | 재시작 복원(ASIO/WASAPI) | 3절 ASIO 재시작 복원 |
| 6 | 절전 방지 | 4절 절전 방지 |
| 7 | 로그 내보내기 바탕화면 | 6절 로그 내보내기 |
| 8 | installer 버전·경로 | Z5, W3, 0절 설치 |
| 9 | ASIO 재시작 순서·장치 목록 검사 | 3절 ASIO 재시작 복원, 장치 목록 검사의 안전 |
| 10 | 작업 집합·우선순위(관리자) | 3절 관리자 권한과 메모리 |
| 11 | 방화벽 | 5절 |
| 12 | 메뉴 동일성 | 8절 |
| 13 | 종료 경로(엔진 정지 1.5초) | 2절 창 정상 닫기 |
| 14 | 프로젝트 미디어 다시 연결 | 9절 |

묶음별 요령:

- **A**
  - 설치: `AtmosMixerPro_Setup.exe` 실행(UAC) → 설정 > 앱의 버전이 exe ProductVersion과 같음, `C:\Program Files\Atmos Mixer Pro\`, 바로가기, `Get-ItemProperty HKCU:\Software\Microsoft\Windows\CurrentVersion\Run | Select-Object AtmosMixerPro`.
  - 방화벽 창: 설치본을 바로가기로 처음 띄울 때 창이 뜨는지, 어떤 선택지가 있는지 사용자가 말해 준 대로 적는다. 외부에서 OSC를 보내는 확인은 C에서.
  - 앱 기동: `Select-String -Path "$env:TEMP\atmos_mixer_pro_logs\atmos_mixer_pro.log" -Pattern 'panic|ASIO Load Error'`가 비어 있다.
  - 로그 회전: 감시와 앱을 끈 뒤 로그를 백업하고 `atmos_mixer_pro.log`에 10MB 넘게 덧붙인다(예: `[IO.File]::AppendAllText($log, (('#' * 1023) + [Environment]::NewLine) * 10500)`). 앱을 띄워 `.1`로 밀리는지 본다. 메모장으로 연 채로도 한다.
  - 로그 내보내기: 사용자가 메뉴 "Export Log" → `[Environment]::GetFolderPath('Desktop')` 폴더에 생기고 `supervisor.log`가 들어 있는지.
  - 감시(장치 무관 항목): 작업 관리자 강제 종료는 `Stop-Process -Name atmos_mixer_pro -Force`와 같다 → 몇 초 뒤 `--auto-relaunched`로 다시 뜨는지(`Get-CimInstance Win32_Process` CommandLine)와 공연 위치. 일시 중단은 사용자가 리소스 모니터(`resmon`) → CPU → 프로세스 우클릭 → "프로세스 일시 중단". 항목마다 `collect-logs.ps1 -Label 2-<번호>`.
  - 3D(W1 전): 스피너만 돌고 앱은 죽지 않으며 하트비트가 정상인지 적는다.
- **E**: `D:\AtmosProjects\<한글 이름 폴더>\`처럼 한글·공백이 든 경로에도 둔다. 열기 전에 `backup-appdata.ps1 -Label before-E`.
- **C**: 현장 장비의 ASIO 드라이버는 사용자가 설치한다. ASIO는 한 번에 한 프로그램만 연다(다른 DAW·앱을 끈다). ASIO 리셋은 드라이버 제어판에서 버퍼 크기를 바꾸거나 장치를 뽑았다 꽂는다. 청음 결과는 사용자의 말을 그대로 적는다(등급 ⑤).
- **D**
  - 12시간: 사용자가 바로가기로 띄워 공연을 시작 → `Start-Process powershell -WindowStyle Minimized -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','D:\dev\Atmos\atmos_mixer_pro\tool\windows\monitor.ps1','-Hours','12'` → 끝나면 CSV로 작업 집합 증가(MB/분), 감시 재실행 횟수, 로그 크기를 정리한다. 메모리는 macOS 릴리스 앱의 재생 중 분당 0.09~0.3MB(12시간 약 65~200MB)와 비교하고, 12시간에 수백 MB 넘게 늘면 사용자에게 알리고 Flutter DevTools(프로필 모드)로 화면 그리기 쪽을 보자고 제안한다(HANDOFF 6). 사용자가 다음에 "장시간 결과 확인해"라고 하면 이어서 한다.
  - 절전 방지: 사용자가 관리자 명령 프롬프트에서 `powercfg /requests` → SYSTEM에 앱. 절전 → 깨우기 뒤 앱이 살아 있다.
  - 로그인 자동 실행: 로그아웃 전에 PROGRESS "재개 지점"을 적는다. 다시 로그인해 Claude를 열고 "이어서"를 받으면 `supervisor.log`·앱 로그로 확인한다.
  - 업그레이드: 공연 중 새 설치 파일 실행 → 감시 → 앱 순서로 끝나는지(`supervisor.log`), 파일 잠금 오류가 없는지.
  - 끝나면 이 PC에 설치본과 로그인 자동 실행을 남길지 사용자에게 묻는다(3.4).

점검표에 없는 추가 확인(X). RESULTS.md에 같이 적고, 첫 결과 PR에서 점검표에 넣자고 제안한다.

| ID | 확인 |
|---|---|
| X1 | VC++ 런타임: exe·dll 의존(`deps.ps1`)과 런타임이 없는 PC에서의 증상(Z6). 앱 Rust dll에 `rust/.cargo/config.toml`의 `+crt-static`이 실제로 걸렸는지(cargo 설정은 cargo를 부른 폴더 기준으로 찾으므로 cargokit 빌드에 걸리는지 확인 필요), 감시 exe에는 그런 설정이 없다 |
| X2 | 사용자 이름·경로에 한글이 있을 때(`%TEMP%`, `%APPDATA%`) 로그·설정·로그 내보내기 |
| X3 | 디스플레이 배율 100/125/150%와 모니터 두 대에서 화면·3D 표시 |
| X4 | 한글 방·트랙 이름이 깨지지 않는다 |
| X5 | 파일 끌어다 놓기(`desktop_drop`)와 파일 선택 창(`file_picker`) |
| X6 | 서명 안 된 설치 파일의 SmartScreen 경고(현장 설치 절차에 영향, 코드 서명은 사용자 결정) |
| X7 | 무인 운영 설정 현황: Windows Update 자동 재시작, 전원 계획(`powercfg /getactivescheme`), USB 선택적 절전, 화면 보호기. 기록만 하고 바꾸는 것은 사용자 |
| X8 | 현장이 Dante면 Dante Virtual Soundcard(ASIO)로 3절 반복 |

### P7 Windows 개발 작업 (W)

| W | 작업 | 수용 기준 |
|---|---|---|
| W1 | 3D 방 뷰어 Windows 구현(필수) | 6절 |
| W2 | 통합 테스트 Windows 이식 | `app_flow_test -d windows` 0·1단계 통과(2단계는 W1 전까지 실패 기록), macOS 동작 불변 |
| W3 | installer 경로 수정(HANDOFF ⚠️ 8) | 저장소 빌드와 CI 폴더 둘 다로 설치 파일 생성 → 설치 → 실행 → 제거 |
| W4 | 런타임·배포(사용자 결정 뒤) | 결정대로 |
| W5 | Windows CI에 `cargo test --no-run`(사용자 결정 뒤) | CI 통과 |
| W6 | Windows에서만 실패하는 `cargo test`·`flutter test`·clippy 경고 | 원인별 PR, macOS 불변 |
| W7 | ASIO 채널 이름 | 보류. 사용자가 풀 때까지 시작하지 않는다(HANDOFF 9) |
| W8 | 이 하네스 고치기 | 실제로 틀린 명령·스크립트를 `win/harness-fixes` PR로 |

- **W2**: `integration_test/support/probes.dart:166`의 `expectNoOtherAppInstance()`가 `pgrep -f atmos_mixer_pro.app/...`를 부른다. Windows에서는 `tasklist /FI "IMAGENAME eq atmos_mixer_pro.exe" /FO CSV /NH`로 같은 이름의 프로세스를 세되 테스트 자신(`dart:io`의 `pid`)은 빼고, `atmos_supervisor.exe`도 없어야 한다. `integration_test/project_media_relink_test.dart:112`의 `externalLibrary: Platform.isMacOS ? ... : null`은 `bundledRustLibrary()`(`lib/core/utils/rust_library.dart`)로 맞춘다. 두 파일 머리 주석의 실행 안내에 `-d windows`를 더한다. PR에 macOS에서 `app_flow_test` 0~6단계를 다시 돌려 달라는 Mac 세션 요청을 쓴다.
- **W3**: `windows/installer.iss`의 `MyAppExePath = AddBackslash(SourcePath) + "build\windows\x64\runner\Release\" + ...`와 `[Files]`의 `Source: "build\windows\x64\runner\Release\..."`는 스크립트 폴더(`atmos_mixer_pro\windows\`) 기준이라 `atmos_mixer_pro\windows\build\...`를 찾는다 → 버전을 못 읽어 `#error Could not read the version`. 최소 수정: 맨 위에 `#ifndef ReleaseDir` / `#define ReleaseDir AddBackslash(SourcePath) + "..\build\windows\x64\runner\Release"` / `#endif`를 두고 `MyAppExePath`와 `[Files]`의 두 `Source`를 `{#ReleaseDir}\...`로 바꾼다. CI zip으로 만들 때는 `ISCC /DReleaseDir=<폴더> windows\installer.iss`.
- **W4**: VC++ 런타임(X1·Z6 결과로 installer에 vc_redist 포함 또는 DLL 동봉), WebView2 런타임(오프라인 현장이면 Evergreen Standalone 설치 파일 포함), 방화벽 규칙(설치 때 넣을지). 결정 전에는 구현하지 않고 근거만 정리해 제안한다.
- **W5**: G4-05 결과(이 PC에서 `cargo test --no-run`이 되는지)를 근거로 `.github/workflows/build_release.yml` Windows 작업에 `cargo test --no-run`(atmos_mixer_pro/rust) 단계를 넣자고 제안한다. CI 시간이 늘어난다는 점도 적는다.

### P8 보고·마무리

- 게이트마다: PROGRESS.md 갱신, 사용자 보고(9.6).
- 결과 PR(`win/results-<yyyyMMdd>`, P6 묶음마다): 점검표 "결과 기록" 표에 행을 더하고(날짜, PC/Windows 버전, 오디오 장치·드라이버, 앱 버전·빌드(커밋·run id), 점검 절, 결과, 비고) 각 항목 `[ ]`에 결과를 적는다. HANDOFF "⚠️ Windows 미검증" 해당 항목에 결과 한 줄, 새로 찾은 문제는 "남은 일"에 더한다. 원본 로그는 커밋하지 않고 핵심 줄만 PR 본문에 옮긴다.
- 세션을 마칠 때: PROGRESS.md "다음 할 일"과 "사용자를 기다리는 것"을 최신으로 둔다.

## 6. W1 — 3D 방 뷰어 Windows 구현

### 6.1 지금 구조(macOS, `webview_flutter` 4.14.1)
- `lib/features/exhibition/state/three_js_engine_provider.dart`
  - `initialize()`: `HttpServer.bind(InternetAddress.loopbackIPv4, 0)`로 Flutter 에셋(`assets/3d_simulator/` 등, three.js 포함 전부 로컬, 약 5.7MB)을 `http://127.0.0.1:<port>/`에서 서빙한다.
  - `_initWebViewController()`: `WebViewController` + JavaScript 무제한 + 배경색 + 콘솔 메시지를 `debugPrint` + JS 채널 `SpeakerBridge` + `onPageFinished` 0.5초 뒤 `isEngineReadyNotifier = true` + `loadRequest(serverUrl)`.
  - `SpeakerBridge` 메시지(JSON 문자열): `{"type":"SPEAKER_SELECTED","speakerId":...}`, `{"type":"SPEAKER_DRAGGING"|"SPEAKER_MOVED","speakerId":...,"x":...,"y":...}`.
  - `executeJavaScript(js)`: 준비된 뒤 `runJavaScript`.
- `lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart:178` — `WebViewWidget(controller: engine.controller!)`. 준비 전에는 스피너.
- `assets/3d_simulator/studio_engine.html` — JS→Dart `window.SpeakerBridge.postMessage(JSON)` 세 곳(663·760·777줄). Dart→JS 함수 `window.updateScene`, `window.setCameraView`, `window.updateEarLevel`(그 밖에 `setMannequinScale`, `toggleTopView`).
- Windows: `webview_flutter`에 Windows 구현이 없어 `initialize()`의 try/catch가 오류를 로그만 남기고 스피너만 돈다. 인스펙터의 스피커 선택(`_selectedInspectorSpeakerId`)이 3D 탭에 의존해 채널↔스피커 연동이 막힌다.

### 6.2 방침(HANDOFF 남은 일 10 권장)
- macOS는 `webview_flutter`를 그대로 둔다. Windows만 WebView2 기반 구현을 `Platform.isWindows` 분기로 붙인다. 손대는 곳은 위 두 Dart 파일과 `pubspec.yaml`로 한정하고, HTML은 바꾸지 않는다 — 문서 시작 시점에 `window.SpeakerBridge` 호환 shim을 주입한다.
- 후보 1(먼저): `webview_windows`(WebView2, Windows 전용 플러그인이라 macOS 빌드 영향이 작다). 흐름: `WebviewController` → `initialize()` → 배경색 → 문서 생성 시 스크립트 주입(shim) → 웹 메시지 스트림에서 문자열·JSON 둘 다 받아 기존 처리로 → 탐색 완료 0.5초 뒤 준비 → `loadUrl(serverUrl)` → `executeScript(js)`. 위젯은 `Webview(controller)`. API 이름은 패키지 문서로 확인한다.
- 후보 2(1이 기준 미달일 때): `flutter_inappwebview` 6.x(Windows 지원). `InAppWebView` + `initialUserScripts`(문서 시작 시점 shim) + `addJavaScriptHandler('SpeakerBridge')` + `onLoadStop` + `evaluateJavascript`. macOS에도 플러그인이 등록되므로 macOS 빌드·동작 확인을 Mac 세션에 요청한다.
- shim:
  ```js
  // webview_windows
  window.SpeakerBridge = { postMessage: function (m) { window.chrome.webview.postMessage(m); } };
  // flutter_inappwebview
  window.SpeakerBridge = { postMessage: function (m) { window.flutter_inappwebview.callHandler('SpeakerBridge', m); } };
  ```
- WebView2 런타임이 없으면 앱이 죽지 않고 3D 자리에 안내 문구를 보인다(런타임 버전 조회가 비면 안내).

### 6.3 절차
1. 중복 확인(2절) → `win/3d-viewer-webview2` 브랜치 → Draft PR.
2. 실험: 후보 1로 두 파일만 바꿔 Debug 빌드 → 측정(측정용 코드는 커밋 전에 지운다)
   - 3D가 보일 때까지 걸린 시간.
   - 프레임 속도: shim과 함께 아래를 주입하고 Dart에서 `FPS` 메시지를 로그로 남긴다.
     ```js
     (function () { var n = 0, t = performance.now(); function f(now) { n++; if (now - t >= 1000) { window.SpeakerBridge.postMessage(JSON.stringify({ type: 'FPS', fps: n })); n = 0; t = now; } requestAnimationFrame(f); } requestAnimationFrame(f); })();
     ```
   - 드래그 왕복: JS 메시지에 `Date.now()`를 실어 Dart 수신 시각과 비교.
   - 카메라 전환, 귀 높이.
   - Release 빌드에서 3D를 움직이는 10분 동안 오디오 끊김·엔진 재시작(앱 로그), CPU·GPU(작업 관리자).
3. 기준(6.4)을 넘으면 정리해 테스트·문서와 함께 Ready로 바꾼다. 못 넘으면 후보 2로 같은 측정을 하고, 두 결과를 표로 비교해 사용자에게 제안한다(패키지 결정은 사용자).

### 6.4 수용 기준(점검표 7절 + α)
- 스피커 배치 화면에서 3D 방이 10초 안에 보이고 스피너가 사라진다(`isEngineReady`).
- 매끄럽다: 체감 끊김 없음, 측정 30fps 이상(목표 60fps).
- 탭 → `SPEAKER_SELECTED` → 인스펙터가 열린다.
- 드래그 → `SPEAKER_DRAGGING`·`SPEAKER_MOVED` → 위치·FX가 따라가고 헤드폰 미리듣기·자동 EQ가 macOS와 같은 방식으로 갱신된다.
- 카메라 시점 전환·귀 높이(`updateEarLevel`)가 동작한다.
- 인터넷을 끊어도 동작한다(에셋은 전부 로컬 127.0.0.1).
- Release 빌드와 CI zip에서도 동작한다(에셋 경로).
- WebView2 런타임이 없으면 앱이 죽지 않고 안내한다.
- 3D가 실패해도 감시 하트비트는 정상이다(점검표 7절 마지막 항목).
- 3D 조작 중 오디오 드롭아웃·엔진 재시작 0(Release, 10분).
- `flutter analyze` 새 이슈 0, `flutter test` 통과, `app_flow_test -d windows` 2단계 통과(W2 뒤).
- macOS 동작 불변: CI macOS 빌드 + PR 본문 "Mac 세션 확인 요청"(macOS에서 `app_flow_test` 0~6단계).

## 7. 알려진 문제와 주의

| # | 내용 | 대응 |
|---|---|---|
| K1 | 3D 방 뷰어 Windows 구현 없음 → 스피너만 돈다 | 지금은 정상. W1 |
| K2 | `app_flow_test` 0단계가 `pgrep`(probes.dart:166)이라 Windows에서 바로 실패 | W2 |
| K3 | `project_media_relink_test`는 Windows에서 기본 로더 → `rust\target\release` dll이 있으면 옛 Rust | 그 dll을 두지 않는다(preflight). W2 |
| K4 | `installer.iss` 경로(HANDOFF ⚠️ 8) → ISCC `#error Could not read the version` | Z5 우회, W3 |
| K5 | VC++ 런타임 의존 미확인 | X1, Z6, W4 |
| K6 | Debug 빌드의 Rust 엔진은 느려 끊길 수 있다 | 소리 품질은 Release로 |
| K7 | 감시 프로그램이 꺼진 앱을 다시 띄운다. 대기 중 충돌 뒤 다시 뜨면 첫 방 테마로 시작(소리) | 감시 먼저 끈다(3.4) |
| K8 | 설치하면 로그인할 때마다 공연이 시작된다(HKCU Run, 기본 켜짐) | 시험 뒤 사용자에게 묻는다 |
| K9 | 방화벽 허용 창은 exe 경로마다 따로 뜬다(Debug, CI 폴더, 설치 폴더) | "첫 실행" 점검은 설치본으로 |
| K10 | 앱 데이터 폴더가 `com.example`(Runner.rc 기본값)이고 개발 빌드·CI zip·설치본이 같이 쓴다 | 시험 전 백업. 이름 변경은 현장 데이터 이전이 필요해 사용자 결정 |
| K11 | ASIO는 한 번에 한 프로그램만 연다 | 다른 DAW·앱·다른 Atmos를 끈다 |
| K12 | CI는 앱의 `cargo test`·`flutter test`를 돌리지 않는다 | Windows 테스트 결과의 근거는 이 PC뿐이다 |
| K13 | Claude 셸의 자식으로 띄운 앱은 셸이 정리될 때 같이 꺼질 수 있다 | 장시간·감시·로그인 시험은 사용자가 띄운다 |
| K14 | Git for Windows 기본 `core.autocrlf=true`(CI와 같다) | 바꾸지 않는다. 텍스트 픽스처를 바이트로 비교하는 테스트가 실패하면 이것부터 의심 |
| K15 | CI의 pdfx CMake 패치는 지금 할 일이 없다(pdfx 의존성 없음) | CMake 버전 오류가 나면 8절 |
| K16 | `v1.1.3` 태그는 `main`의 조상이 아니다(HANDOFF 11) | 현장 설치본 버전을 볼 때 주의 |

## 8. 문제 해결

| 증상 | 원인·조치 |
|---|---|
| `flutter doctor`: Visual Studio 없음·구성요소 부족 | Visual Studio Installer → 수정 → "C++를 사용한 데스크톱 개발"과 권장 구성요소(MSVC v143, Windows SDK, C++ CMake tools) |
| `Building with plugins requires symlink support` | 개발자 모드(사용자, QUICKSTART 1절) |
| bindgen `Unable to find libclang` | `LIBCLANG_PATH=D:\dev\LLVM\bin`, `libclang.dll` 존재, 새 셸 또는 env.ps1 |
| asio-sys 빌드: ASIO SDK·`asio.h`를 못 찾음 | `CPAL_ASIO_DIR`이 `common\asio.h`가 있는 SDK 최상위인지 |
| `linker 'link.exe' not found` | VS C++ 도구, rustup 툴체인이 `-msvc`인지 |
| rusqlite(bundled)·cc 빌드 오류 | VS C++ 도구와 Windows SDK |
| cargokit `build_tool` 오류 | `flutter pub get` 다시, 로그 위치 확인. `flutter clean`은 사용자 확인 뒤 |
| `Compatibility with CMake < 3.5 has been removed` | 그 플러그인 CMakeLists의 `cmake_minimum_required`를 CI의 pdfx 패치처럼 3.5로(`windows/flutter/ephemeral/.plugin_symlinks/<플러그인>/windows/`), 또는 환경변수 `CMAKE_POLICY_VERSION_MINIMUM=3.5` |
| `LNK1104: cannot open file ...exe/dll` | 앱이 실행 중(파일 잠김). preflight |
| `Content hash ... different` | 옛 dll이 실렸다. 앱이면 로더(G4-10), 테스트면 `rust\target\release` dll |
| 앱이 뜨자마자 꺼짐, `VCRUNTIME140_1.dll was not found` | VC++ 런타임(X1, `collect-logs.ps1`의 이벤트) |
| 경로가 너무 김 | `git config --global core.longpaths true`, 그래도 나면 사용자가 LongPathsEnabled(관리자) |
| 빌드가 매우 느림 | Defender 실시간 검사. `D:\dev` 예외는 사용자 판단 |
| `.ps1` 실행이 막힘 | `powershell -NoProfile -ExecutionPolicy Bypass -File ...`로 부른다(정책은 바꾸지 않는다) |
| 출력의 한글이 깨짐 | env.ps1의 UTF-8 설정, run.ps1의 `chcp 65001` |
| `gh run download` 404 | `gh auth status`, 아티팩트 보관 기간 만료 → 사용자가 CI를 다시 돌린다 |
| `flutter test ... -d windows`에서 장치 없음 | `flutter devices`에 Windows가 있는지, `flutter config --enable-windows-desktop` |
| 환경변수가 옛 값 | Claude 앱을 트레이까지 종료하고 다시 열기, 또는 env.ps1 점 소싱 |
| rustup-init이 Visual Studio 설치를 묻는다 | VS를 먼저 설치(P2 순서) |
| Git Bash에서 `/MIR`·`/v` 같은 스위치가 경로로 바뀜 | `MSYS_NO_PATHCONV=1`을 앞에 붙이거나 PowerShell로 |

## 9. 템플릿

### 9.1 이 PC 규칙 — `%USERPROFILE%\.claude\CLAUDE.md`

```markdown
# Windows 개발 PC 규칙 (Atmos Mixer Pro)

이 PC는 Atmos Mixer Pro의 Windows 개발·검증 PC다. 저장소 D:\dev\Atmos, 하네스 문서 D:\dev\Atmos\atmos_mixer_pro\docs\windows\WINDOWS_HARNESS.md, 진행 상태 D:\dev\harness\PROGRESS.md.

## 세션 시작
1. D:\dev\harness\PROGRESS.md를 읽고 "다음 할 일"부터 잇는다. 하네스 문서는 필요한 절을 다시 읽는다.
2. git -C D:\dev\Atmos fetch --prune, git status, gh pr list --repo Apejean/Atmos로 바뀐 것을 본다.
3. 빌드·장치 시험 전에는 tool\windows\preflight.ps1. 실행 중인 Atmos가 있으면 사용자에게 먼저 묻는다.
4. 이번 세션 계획을 한국어 3~5줄로 말하고 시작한다.

## 말과 기록
- 사용자에게 보이는 답·보고·질문은 한국어. 코드·명령·커밋 메시지는 영어.
- 채널 번호는 CH1부터 말한다(내부 channel은 0부터, CH = 내부 + 1).
- 명령은 tool\windows\run.ps1로 돌려 D:\dev\logs에 증거를 남기고 PROGRESS.md·RESULTS.md에 경로를 적는다.
- "CI 빌드 통과 / 이 PC 빌드·테스트 통과 / 이 PC 실행 확인 / 장치로 확인 / 사람이 들어 확인"을 구분한다. macOS에서 확인한 것을 Windows 보증으로 말하지 않는다. 실패는 출력 그대로, 건너뛴 것은 건너뛰었다고 말한다.

## 확정된 결정(다시 제안하지 말 것)
- 크로스오버 80Hz·LR24 유지. 서브 저음이 메인 자리에서 들리는 것은 의도된 상태다.
- 현장 PC는 Windows. 지금 되는 기능은 Windows에서도 같아야 하고 3D 방 뷰어는 반드시 들어간다. OS 고유 차이(메뉴 형태·종료 경로·절전 방지·로그 경로 구현)는 허용한다.
- 프로젝트는 오디오·도면을 .atmos와 같은 폴더(하위 폴더 가능)에 담아 옮긴다.
- 운용 흐름: macOS에서 설계 → 현장 Windows PC로 옮김 → 인터페이스 다시 스캔·채널 연결·귀로 조정 → 무인 운영.
- 초기반사 방 시뮬레이션은 바이노럴 전용, 서브우퍼는 방마다 하나(스피커 속성), 베이스 매니지먼트는 채널 DSP 앞.
- ASIO 채널 이름 작업은 보류(시작하지 않는다).

## 일하는 방식
- 서브에이전트·워크플로(여러 에이전트)를 쓰지 않는다. 혼자 순서대로 한다.
- 저장소 CLAUDE.md의 DSP 3법칙, Rust 주석 한국어, 운영 오디오 코드 unwrap 금지, 외과적 변경을 지킨다.
- 오래 걸리는 명령(첫 cargo test, flutter build, 통합 테스트)은 백그라운드로 돌리고 로그로 확인한다.
- Git Bash에서 Windows 명령·스크립트를 부를 때는 MSYS_NO_PATHCONV=1을 앞에 붙이고 경로는 D:/dev/...로 쓴다.

## Git·GitHub
- main에 직접 커밋·푸시하지 않는다. 브랜치는 origin/main에서 win/<주제>.
- 커밋은 사용자가 요청했거나 하네스 단계가 정한 때만. 병합은 사용자가 지시할 때만(머지 커밋).
- PR 본문은 한국어(요약/변경/검증(Windows 증거)/macOS 영향/Mac 세션 확인 요청/남은 일·결정).
- 큰 작업은 Draft PR을 먼저 연다. 문서 변경은 그 기능 브랜치에 같이 넣는다. 충돌은 양쪽을 살리고, 이미 병합된 문서는 origin/main 쪽을 기준으로 한다.
- Mac 세션(Main·Sub)에는 직접 연락할 수 없다. 요청은 PR 본문·댓글에 쓰고 사용자에게 "Mac에 전달해 주세요"라고 알린다.
- 커밋하지 않는 것: D:\dev\logs·artifacts·backups, windows\Output, windows\build, .claude\settings.local.json, 빌드 산출물.
- 저장소 CLAUDE.md, .claude\agents, .agents는 사용자가 요청할 때만 고친다.

## 안전
- %APPDATA%\com.example\atmos_mixer_pro와 %TEMP%\atmos_mixer_pro_logs는 사용자의 실제 데이터다. 바꾸는 시험 전에 backup-appdata.ps1, 되돌리기·삭제는 사용자 확인 뒤.
- 지우는 것은 이 세션이 만든 것만. rust\target, build, pub-cache, 사용자 캐시는 묻고 지운다.
- 시스템·보안 설정(개발자 모드, Defender 예외, 방화벽 규칙, LongPaths, 전원 계획, Windows Update, UAC)은 사용자가 직접 한다. 어디서 무엇을 누르는지만 안내한다.
- 라이선스·약관 동의(Visual Studio, ASIO SDK)는 사용자에게 확인받는다.
- 소리가 나는 시험 전에 어떤 장치로 나는지 알리고 음량을 확인받는다.
- 앱을 끌 때는 atmos_supervisor를 먼저, atmos_mixer_pro를 그다음에.
- 재부팅·로그아웃이 필요한 시험 전에는 PROGRESS.md "재개 지점"을 먼저 적는다.
```

### 9.2 권한 — `D:\dev\Atmos\.claude\settings.local.json`

```json
{
  "permissions": {
    "allow": [
      "Bash(git status:*)",
      "Bash(git diff:*)",
      "Bash(git log:*)",
      "Bash(git show:*)",
      "Bash(git fetch:*)",
      "Bash(git branch:*)",
      "Bash(git rev-parse:*)",
      "Bash(git merge-base:*)",
      "Bash(gh pr list:*)",
      "Bash(gh pr view:*)",
      "Bash(gh pr checks:*)",
      "Bash(gh pr diff:*)",
      "Bash(gh run list:*)",
      "Bash(gh run view:*)",
      "Bash(gh run download:*)",
      "Bash(cargo check:*)",
      "Bash(cargo clippy:*)",
      "Bash(cargo test:*)",
      "Bash(cargo build:*)",
      "Bash(flutter pub get:*)",
      "Bash(flutter analyze:*)",
      "Bash(flutter test:*)",
      "Bash(flutter build:*)",
      "Bash(flutter doctor:*)",
      "Bash(flutter --version:*)",
      "Bash(dart --version:*)"
    ],
    "ask": [
      "Bash(git commit:*)",
      "Bash(git push:*)",
      "Bash(gh pr create:*)",
      "Bash(gh pr merge:*)",
      "Bash(gh pr comment:*)",
      "Bash(gh workflow run:*)",
      "Bash(winget:*)"
    ],
    "deny": [
      "Bash(git push --force:*)",
      "Bash(git push -f:*)",
      "Bash(git reset --hard:*)",
      "Bash(git clean:*)",
      "Bash(git branch -D:*)",
      "Bash(rm -rf:*)",
      "Bash(gh repo delete:*)",
      "Bash(gh release:*)"
    ],
    "additionalDirectories": [
      "D:\\dev\\harness",
      "D:\\dev\\logs",
      "D:\\dev\\artifacts",
      "D:\\dev\\backups"
    ]
  }
}
```

- PowerShell 도구 명령이나 스크립트 실행은 권한 창에서 사용자가 "항상 허용"을 고르면 Claude Code가 맞는 형식으로 이 파일에 더한다.
- 이 규칙은 실수를 막는 장치일 뿐 보안 경계가 아니다(같은 일을 다른 명령으로 할 수 있다). 3.4가 우선한다.

### 9.3 `D:\dev\harness\PROGRESS.md`

```markdown
# Windows 하네스 진행 상황

- PC: <컴퓨터 이름> / Windows <버전·빌드> / 계정 <관리자 여부> / 오디오 장치 <이름·드라이버>
- 저장소: D:\dev\Atmos  브랜치 <...>  HEAD <...>  (마지막 fetch <시각>)
- 마지막 갱신: <yyyy-MM-dd HH:mm>

## 지금
- 단계·게이트: <P? / G?>
- 다음 할 일(순서대로): 
- 사용자를 기다리는 것(무엇을, 어디서): 
- 재개 지점(재부팅·로그아웃·Claude 재시작 뒤 첫 명령): 

## 게이트 (상태: 대기 / 진행 / 통과 / 실패 / 보류 / 해당 없음)
| 게이트 | 내용 | 상태 | 날짜 | 증거 | 비고 |
|---|---|---|---|---|---|
| G0 | 사용자 사전 준비 | 대기 | | | |
| G1 | 저장소 | 대기 | | | |
| G2 | 도구(doctor FAIL=0) | 대기 | | | |
| G3 | 하네스·재시작 | 대기 | | | |
| G4-01~G4-15 | 빌드·자동 테스트(게이트마다 한 줄) | 대기 | | | |
| Z1~Z6 | CI zip | 대기 | | | |
| P6-A/B/E/C/D | 점검표 묶음 | 대기 | | RESULTS.md | |
| W1~W8 | Windows 작업 | 대기 | | | PR 번호 |

## 발견한 문제
| 번호 | 내용 | 증거 | 조치(W?·PR·HANDOFF) |
|---|---|---|---|

## 세션 기록
- <날짜>: <한 일 한 줄>
```

### 9.4 `D:\dev\harness\RESULTS.md`

```markdown
# Windows 현장 점검 결과

- PC·장치: <...>   앱: <커밋 / CI run id / 버전>

| ID | 점검표 항목(요약) | 묶음 | 결과 | 등급 | 증거 | 날짜 | 비고 |
|---|---|---|---|---|---|---|---|
| 0-1 | Windows CI 빌드 | A | OK | ① | PR #12 CI run <id> | | |
| X1 | VC++ 런타임 의존 | A | | | | | |
```

결과는 OK / 실패(증상) / 해당 없음 / 보류, 등급은 3.1의 ①~⑤.

### 9.5 PR 본문

```markdown
## 요약
## 변경
## 검증(Windows, 이 PC)
- 등급: ②/③/④/⑤ 중 무엇까지
- 명령과 결과(로그 D:\dev\logs\...): 핵심 줄만
## macOS 영향
## Mac 세션 확인 요청
## 남은 일·사용자 결정
```

맨 끝에 Claude Code가 정한 생성 표시 줄을 붙인다.

### 9.6 사용자 보고

```text
[P4 G4-06 cargo test] 이 PC 자동 테스트: 통과 210, 실패 3
- 증거: D:\dev\logs\2026-10-08\G4-06_cargo_test_101530.log
- 실패: test_x — 경로 구분자 차이로 보임(추정), W6 후보
- 다음: G4-07 flutter analyze
- 사용자 필요: 없음
```

## 10. 스크립트 (`atmos_mixer_pro/tool/windows/`)

실행: `powershell -NoProfile -ExecutionPolicy Bypass -File D:/dev/Atmos/atmos_mixer_pro/tool/windows/<이름>.ps1 <인자>`(env.ps1만 점 소싱). 파일은 UTF-8(BOM)이고 Windows PowerShell 5.1 기준이다.

| 스크립트 | 하는 일 | 예 | 종료 코드 |
|---|---|---|---|
| `env.ps1` | 점 소싱: 환경변수·PATH·UTF-8 출력·TLS 1.2, `$Atmos` 경로 표 | `. D:\dev\Atmos\atmos_mixer_pro\tool\windows\env.ps1` | — |
| `setup-env.ps1` | 사용자 환경변수·PATH 저장(PATH는 REG_EXPAND_SZ 유지), 하네스 폴더 만들기 | `setup-env.ps1` | 0 |
| `run.ps1` | 명령을 cmd로 돌려 출력 전체를 `D:\dev\logs`에 | `run.ps1 -Gate G4-03_cargo_check -Cwd D:\dev\Atmos\atmos_mixer_pro\rust -Run "cargo check"` | 명령의 종료 코드 |
| `doctor.ps1` | 도구·설정 점검 표(G2) | `doctor.ps1` | FAIL 있으면 1 |
| `preflight.ps1` | 실행 중인 Atmos, 디스크, 옛 dll, git 상태 | `preflight.ps1` | FAIL 있으면 1 |
| `fetch-artifact.ps1` | CI zip 받기·풀기·필수 파일·해시·exe 버전 | `fetch-artifact.ps1 -Pr 12` | MISS 있으면 1 |
| `deps.ps1` | exe·dll의 의존 DLL, VC++ 런타임 표시 | `deps.ps1 -Dir <폴더>` | 0 |
| `backup-appdata.ps1` | 앱 데이터·로그·HKCU Run 백업/되돌리기 | `-Label before-E` / `-Restore <백업 폴더>` | 실패 1 |
| `collect-logs.ps1` | 로그·신호 파일·오류 이벤트·WER·프로세스 상태 묶음 | `-Label 2-1 -Hours 2` | 0 |
| `monitor.ps1` | 장시간 자원 CSV(멈춤: `D:\dev\harness\monitor.stop`) | `-Hours 12` | — |

## 11. 사용자 결정 대기

| 결정 | 지금 | 근거 |
|---|---|---|
| 3D 뷰어 Windows 구현 방식(패키지) | W1 실험 뒤 제안 | HANDOFF 10 |
| WebView2 런타임 배포(오프라인 현장이면 설치 파일에 포함) | 대기 | 점검표 0·7절 |
| installer 방화벽 규칙(무인 PC 첫 실행 창) | 대기 | 점검표 5절 |
| VC++ 런타임 포함 방식 | X1·Z6 뒤 | X1 |
| Windows CI에 `cargo test --no-run` 단계 | G4-05 뒤 | HANDOFF 12 |
| ASIO 채널 이름 | 보류(시작하지 않음) | HANDOFF 9 |
| v1.1.3 태그와 `main` 버전 차이(현장 설치 버전) | 대기 | HANDOFF 11 |
| 앱 데이터 폴더 이름(`com.example`) 변경 | 대기(바꾸면 현장 데이터 이전 필요) | 4절 |
| 코드 서명(SmartScreen) | 대기 | X6 |
| Visual Studio Community 라이선스 적합성 | P2 전 | P2 |
| 이 PC의 Defender 예외·전원 계획·Windows Update | 사용자 판단 | 3.4 |
| 시험 뒤 설치본·로그인 자동 실행을 남길지 | P6-D 뒤 | 3.4 |
