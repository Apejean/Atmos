# Windows 빌드 빠른 안내

- 2026-10-07. Windows PC 앞에서 사람이 먼저 읽는 짧은 안내다. 자세한 단계·규칙·명령은 [`WINDOWS_HARNESS.md`](WINDOWS_HARNESS.md)(Windows의 Claude가 따르는 하네스)에 있다.
- **빌드 기준: `main` 1907e73 이후.** PR #12(2026-10-07 병합)에 Windows 로더 수정이 들어 있다 — 앱이 실행 파일 옆의 `rust_lib_atmos_mixer_pro.dll`을 직접 연다. 그보다 앞선 커밋으로 빌드하지 않는다.
- 지금 상태: CI(PR #8·#10·#12)에서 Windows **빌드**는 통과했다. Windows에서 실행·소리·ASIO·감시 프로그램·3D는 아직 아무도 확인하지 않았다.

## 1. Claude를 열기 전에 직접 할 일 (약 20분)

준비: Windows 10 22H2 또는 Windows 11, 관리자 계정, D: 여유 60GB 이상, C: 여유 15GB 이상, `winget`(없으면 Microsoft Store에서 "앱 설치 관리자").

1. **개발자 모드 켜기** — `Win + R` → `ms-settings:developers` → "개발자 모드" 켬. Flutter 플러그인 빌드(심볼릭 링크)에 필요하다.
2. **Git·GitHub CLI 설치** — PowerShell(관리자 아님)에서:
   ```powershell
   New-Item -ItemType Directory -Force D:\dev | Out-Null
   winget install --id Git.Git -e --source winget --override "/VERYSILENT /NORESTART /NOCANCEL /SP- /SUPPRESSMSGBOXES /DIR=D:\dev\Git"
   [Environment]::SetEnvironmentVariable('CLAUDE_CODE_GIT_BASH_PATH', 'D:\dev\Git\bin\bash.exe', 'User')
   winget install --id GitHub.cli -e --source winget
   ```
3. **GitHub 로그인** — PowerShell 창을 새로 열고 아래를 실행한 뒤, 브라우저에 뜬 코드를 넣어 승인한다.
   ```powershell
   gh auth login --hostname github.com --git-protocol https --web
   gh auth setup-git
   ```
4. **Claude 설치·로그인** — Claude 데스크톱 앱을 설치·로그인하고 Code에서 작업 폴더 `D:\dev`를 연다. (터미널용 Claude Code를 쓰면 PowerShell에서 `irm https://claude.ai/install.ps1 | iex` 뒤 `D:\dev`에서 `claude`.)
5. (선택) **Defender 예외** — Windows 보안 → 바이러스 및 위협 방지 → 설정 관리 → 제외 → 폴더 `D:\dev`. 빌드가 크게 빨라지지만 그 폴더는 검사하지 않는다. 판단은 사용자 몫이다.

## 2. 시작 프롬프트 — Windows의 Claude에게 그대로 보낸다

```text
아래는 내가 직접 보내는 작업 지시다.

너는 Atmos Mixer Pro의 Windows 개발 PC에서 일하는 Claude다. 현장(전시 운영) PC가 Windows라서, 이 PC에서 개발 환경을 갖추고 빌드·테스트·현장 점검을 하고 Windows 전용 작업을 한다. 지금까지의 개발·검증은 macOS에서 했다.

1. 저장소를 받는다. D:\dev\Atmos가 없으면 `gh repo clone Apejean/Atmos D:\dev\Atmos`, 있으면 `git -C D:\dev\Atmos pull --ff-only`. main에 1907e73(PR #12, Windows 로더 수정)이 들어 있어야 한다.
2. D:\dev\Atmos\atmos_mixer_pro\docs\windows\WINDOWS_HARNESS.md를 처음부터 끝까지 읽고 이 PC 작업의 기준으로 삼는다. 저장소 루트 CLAUDE.md, atmos_mixer_pro\docs\HANDOFF.md, atmos_mixer_pro\docs\WINDOWS_FIELD_CHECKLIST.md도 읽는다.
3. 하네스 문서의 "0. 지금 할 일"과 단계 순서(P0 확인 → P1 저장소 → P2 도구 설치 → P3 하네스 설치와 재시작 → P4 빌드·테스트 → P5 CI zip 검사 → P6 현장 점검 → P7 Windows 작업 → P8 보고)대로 진행한다. 게이트를 통과해야 다음 단계로 간다.
4. 게이트마다 D:\dev\harness\PROGRESS.md를 갱신하고 결과를 한국어로 짧게 보고한다. 설치 승인(UAC), 라이선스 동의, 시스템 설정, 장치 연결, 소리 확인, 재부팅처럼 내가 해야 하는 일은 어디서 무엇을 누르는지까지 알려 주고 기다린다.
5. 서브에이전트·워크플로(여러 에이전트)는 쓰지 말고 혼자 순서대로 한다(토큰 비용).
```

다음 세션부터는 "PROGRESS 보고 이어서 진행해"만 보내면 된다(P3에서 이 PC 전용 규칙이 설치된다).

## 3. Claude가 하는 순서와 내가 할 일

| 단계 | Claude | 내가 할 일 |
|---|---|---|
| P1 저장소 | `D:\dev\Atmos`로 받기 | — |
| P2 도구 설치 | Visual Studio 2022 Community(C++ 데스크톱) → Rust → LLVM 15.0.7 → ASIO SDK → Flutter → Inno Setup, 모두 D: | 라이선스 동의 확인(VS Community 사용 조건, Steinberg ASIO SDK), UAC "예" |
| P3 하네스 | 이 PC 규칙·권한·진행 파일 만들기 | Claude를 트레이까지 완전히 종료 → 다시 열어 `D:\dev\Atmos` 폴더로 → "PROGRESS 보고 이어서 진행해" |
| P4 빌드·테스트 | CI와 같은 순서로 빌드 + `cargo test`, `flutter test`, 개발 실행, 통합 테스트 | 화면 확인(대시보드가 뜨는지, 3D 화면은 스피너가 정상), 소리 날 때 음량 |
| P5 CI zip | PR #12 zip 받기·확인·감시로 실행, 설치 파일 만들기 | 화면 확인, 창 닫기 |
| P6 현장 점검표 | `WINDOWS_FIELD_CHECKLIST.md` 항목 진행·기록 | 설치, 장치 연결·뽑기, 청음, 외부 OSC, 로그아웃·로그인 |
| P7 Windows 작업 | 3D 방 뷰어 Windows 구현(필수), 통합 테스트 Windows 이식, installer 경로 수정 → PR | PR 확인, 병합 지시 |

## 4. Claude 없이 직접 빌드할 때 — CI Windows 작업과 같은 순서

도구는 하네스 P2대로 설치되어 있어야 한다. `.github/workflows/build_release.yml`의 Windows 작업(PR #8·#10·#12에서 통과)을 로컬 명령으로 옮긴 것이다.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File D:\dev\Atmos\atmos_mixer_pro\tool\windows\setup-env.ps1   # 한 번만
powershell -NoProfile -ExecutionPolicy Bypass             # 이 셸에서만 스크립트 실행 허용(시스템 정책은 그대로)
. D:\dev\Atmos\atmos_mixer_pro\tool\windows\env.ps1        # 도구 경로 + Visual Studio 개발자 환경
cd D:\dev\Atmos\atmos_mixer_pro
cargo test --manifest-path supervisor\Cargo.toml             # CI: Supervisor tests
flutter pub get                                              # CI: Install dependencies
Push-Location rust; cargo check; Pop-Location                # CI: Cargo Check
flutter build windows --release                              # CI: Build
cargo build --release --manifest-path supervisor\Cargo.toml  # CI: Add supervisor to Release
Copy-Item supervisor\target\release\atmos_supervisor.exe build\windows\x64\runner\Release\
```

- env.ps1을 점 소싱하지 않은 창(일반 PowerShell, VS Code 터미널)에서 빌드하면 ASIO 빌드가 `Could not find vcvarsall.bat`으로 멈춘다. Visual Studio를 D:에 설치했기 때문이다(asio-sys는 `C:\Program Files`만 찾는다). env.ps1이 개발자 환경을 넣는다.
- 실행: `build\windows\x64\runner\Release\atmos_supervisor.exe`(감시 프로그램이 앱을 띄운다). 앱 exe를 직접 띄워도 옆에 감시 exe가 있으면 감시로 넘어간다. 끌 때는 창을 닫는다(X).
- 개발 PC 확인(HANDOFF 12): `Push-Location rust; cargo test --no-run; Pop-Location`이 컴파일되는지.
- 설치 파일: `installer.iss`가 `windows\` 폴더 기준으로 `build\...`를 찾는 문제(HANDOFF ⚠️ 8)가 있어 고치기 전에는 이렇게 우회한다(`windows\build`, `windows\Output`은 커밋하지 않는다).
  ```powershell
  robocopy build\windows\x64\runner\Release windows\build\windows\x64\runner\Release /MIR
  D:\dev\InnoSetup6\ISCC.exe windows\installer.iss            # → windows\Output\AtmosMixerPro_Setup.exe
  ```
- CI의 "Patch pdfx CMakeLists" 단계는 지금 pdfx 의존성이 없어 할 일이 없다. CMake 버전 오류가 나면 하네스 8절.

## 5. 빌드 뒤 확인 순서 (우선순위)

기준은 `docs/WINDOWS_FIELD_CHECKLIST.md`와 HANDOFF "⚠️ Windows 미검증" 1~15다.

1. 앱이 뜨는지 — 새 로더가 exe 옆 dll을 여는지(점검표 0절 앱 기동, 10절 로더)
2. 소리 — WASAPI 기본 장치, 그다음 현장 ASIO 장비와 12ch 출력(3절)
3. 재시작 복원 — ASIO 리셋, 장치 뽑았다 꽂기, 30초 넘게 뽑았을 때 비상 전환(−40dB)과 다시 꽂았을 때 원래 크기로 복귀(3절)
4. 충돌 후 자동 재실행 — 감시 프로그램(2절)
5. 절전 방지 — 관리자 명령 프롬프트 `powercfg /requests`(4절)
6. 로그 내보내기 경로 — OneDrive로 옮겨진 바탕화면, 한글 "바탕 화면"(6절)
7. 방화벽 허용 창 — 설치본 첫 실행(5절)
8. 메뉴 동일성 — Settings·Preferences 항목(8절)
9. 종료 경로 — 창을 닫은 뒤 ASIO 장치가 풀리는지(2절)
10. 프로젝트 미디어 다시 연결 — macOS에서 옮긴 폴더(9절)

- **3D 방 뷰어는 Windows 구현이 아직 없어 스피너만 도는 것이 지금은 정상이다**(HANDOFF 남은 일 10, 하네스 W1).
- 개발 PC라면 `rust\`에서 `cargo test --no-run`이 컴파일되는지도 본다(HANDOFF 12).

## 6. 사용자 결정 대기

- installer 방화벽 규칙(무인 PC 첫 실행의 허용 창)
- WebView2 런타임 설치 방식(오프라인 현장이면 설치 파일에 포함)
- 3D 방 뷰어 Windows 구현 방식(`webview_windows` 또는 `flutter_inappwebview` — Windows PC 실험 뒤)
- v1.1.3 태그와 `main`의 버전 차이(현장에 설치된 버전 확인)
- Windows CI에 `cargo test`(`--no-run`) 단계 추가
- ASIO 채널 이름(보류)
- Windows 실행 뒤 정할 것: VC++ 런타임 포함 방식, 코드 서명(SmartScreen), 앱 데이터 폴더 이름(`com.example`)
