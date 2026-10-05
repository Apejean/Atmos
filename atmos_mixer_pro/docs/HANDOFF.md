# 작업 인계 (2026-10-05 기준)

새 세션에서 "docs/HANDOFF.md 보고 이어서 해줘"로 시작하면 된다. 설계 결정의 근거는 각 스펙 문서와 테스트 주석에 있다.

> ⚠️ **현장(전시 운영) PC는 Windows다**(사용자 확인 2026-10-05). 개발·검증은 macOS에서 했다. macOS에서 확인한 결과를 현장 동작 보증으로 보지 말고, 새 변경마다 Windows에서도 되는지(경로, ASIO/WASAPI, 바탕화면, 웹뷰, 절전)를 먼저 따진다. 남은 일은 Windows 기준 우선순위로 정리했다.

## 최근 끝난 일

| 항목 | 결과 | 확인하는 테스트 |
|---|---|---|
| 무인 운영 기본기(Windows 기준 우선순위 3·4번 중 바로 할 수 있던 것) | **로그 회전**: 로그 파일이 10MB를 넘으면 뒤로 밀고 최대 5개(약 50MB)만 남긴다(`core/log_file.rs`). 로그 내보내기는 밀린 파일까지 복사한다. **로그 내보내기 위치**: Windows는 셸이 아는 실제 바탕화면(OneDrive로 옮겨진 경우 포함)을 쓰고, 못 찾으면 폴더를 묻는다(`lib/core/utils/log_export_dir.dart`). **절전 방지**: 앱이 켜져 있는 동안 시스템 절전을 막는다(화면은 꺼져도 됨). Windows는 `SetThreadExecutionState`, macOS는 `caffeinate -i -w <pid>`(`core/keep_awake.rs`, `api_init_app`에서 시작). **installer 버전**: 빌드된 exe의 `ProductVersion`(= pubspec 버전)에서 읽는다(예전에는 `1.0.41` 고정). Windows 쪽은 CI·실기로 아직 확인 못 함 | `test_log_rotation`, `test_keep_awake`(macOS), `test/log_export_dir_test.dart` |
| OSC로 루프·스트리밍 트랙이 안 나오고, OSC로 방을 비운 뒤 다음 방 BGM이 안 나옴. 방 비우기가 모든 방의 재생 목록을 지움 | OSC 재생과 다음 방 BGM 자동 재생은 대시보드와 같은 `api_play_track` 경로를 탄다(예전에는 RAM 캐시만 봤는데 루프·스트리밍 트랙은 거기 없다). 방 비우기(대시보드·OSC)는 그 방 트랙만 재생 목록에서 뺀다(엔진도 그 방만 멈춘다). 그래서 다른 방 BGM이 목록에서 사라졌다가 자동 승격 때 겹쳐 두 번 나오는 일도 없다 | `test_osc_room_routing`(실제 OSC 리스너에 UDP로 보냄) |
| 엔진이 스스로 재시작하면 재생이 사라짐(화면은 재생 중으로 남음) | 재생 중이던 트랙을 **멈춘 위치부터 자동으로** 다시 튼다(`rust/src/core/restart_resume.rs`). 감시 루프가 재시작을 정한 직후(옛 엔진 drop 전) 재생 목록·커서를 떠 두고, 새 엔진이 뜨면 `EngineRestarted:<순번>` 이벤트 → Dart 재동기화(서브 라우팅·FX) → `apiAckEngineRestart` → Rust 재개(응답이 2초 안에 없으면 그대로 재개). 기다리는 사이 트랙 정지·전체 정지·방 비우기(대시보드·OSC 모두)는 재개를 취소하고, 재개가 도는 도중의 정지도 이긴다. 최종 리뷰(2026-10-04)로 보강: All Mute가 엔진 재시작에도 유지되고, 스트리밍 재개는 첫 묶음이 올 때까지 페이드를 멈춰 딸깍이 없고, OSC 정지는 재생 목록에서 바로 빠지고, 인스턴스 번호는 시계 대신 전역 카운터다. 실측: 위치 6.01→9.64초(흐른 시간 4.30초 — 재시작 동안 앞으로 건너뛰지 않음), CH1 −34.23→−34.23·CH2 −31.39→−31.46dBFS, 재시작 알림→재개 107ms 동안 디지털 무음 | `test_restart_resume`, `test_restart_resume_races`, `test_master_mute_survives_restart`, `test_stream_resume_onset`, 통합 테스트 5단계 |
| 재생 위치 조회 API | `api_get_playback_positions`(트랙별 파일 기준 초, 루프는 한 바퀴 안). 오디오 스레드는 원자 칸에 적기만 한다(`audio/playback_cursor.rs`) | `test_playback_cursor_table`, `test_playback_positions_api`, `test_instance_position`, `test_disk_streamer_start_offset` |
| 실제 앱 흐름 자동 검증(통합 테스트) | `integration_test/app_flow_test.dart`: 0 부팅 → 1 재생/정지 → 2 스피커 배치·인스펙터 → 3 스피커 이동 → FX → 4 서브 라우팅 → 5 재시작 복원(+재기동 창 5회) → 6 테마 시작. 실행: `cd atmos_mixer_pro && flutter test integration_test/app_flow_test.dart -d macos`(약 2~3분, 시스템 기본 출력 장치로 −30dBFS 톤이 실제로 난다, macOS 전용·수동 실행·CI 아님, 다른 Atmos 앱을 먼저 끌 것). 되돌리면 실패하는 변형 9개 확인, 3회 연속 통과. 보장하지 않는 것은 설계 10절 "알려진 한계"(`docs/superpowers/specs/2026-10-01-app-flow-integration-test-design.md`) | 자체 |
| 워치독 재시작마다 옛 엔진이 통째로 남음(cpal 누수) | cpal 0.15.3(macOS)은 이름으로 고른 장치의 스트림에 장치 분리 리스너를 달면서 리스너 안에 스트림 자신(Arc)을 넣었다. 순환 참조라 pause·drop해도 AudioUnit·콜백·믹서가 남았다. cpal **0.16.0**으로 올림(리스너가 Weak만 잡는다). 0.17 이상은 `SampleRate` 타입 변경 등 호환성 깨짐이 커서 올리지 않았다 | `test_engine_drop_releases_stream` |
| 재시작마다 분석 스레드가 쌓임 | 엔진이 시작할 때마다 띄우는 분석 스레드(`analysis.rs`)가 엔진이 사라져도 끝나지 않고 5ms마다 깨어났다. 생산자(엔진의 믹서)가 사라지고 버퍼가 비면 끝낸다 | `test_analysis_thread_exits_with_engine` |
| 서브 라우팅 수치 재검증(80Hz·LR24) | 50Hz: 서브 −1.24dB·메인 −17.57dB, 200Hz: 서브 −32.05dB·메인 −0.22dB. LR4 이론값과 0.01dB 안에서 일치 | `test_bass_management_routing`의 `크로스오버_80hz에서_50hz와_200hz가_lr24_이론값대로_나뉜다` |
| 정지해도 소리가 계속 남 | 워치독 재시작 뒤 옛 cpal 스트림이 살아서 명령 큐를 나눠 먹었다. 콜백이 엔진 세대를 확인해 옛 세대는 무음만 내도록 수정(`engine.rs` `begin_callback`). 누수는 위에서 없앴고, 세대 확인은 옛 엔진이 drop되기 전 구간을 위해 남긴다 | `test_stale_engine_generation` |
| 멀티트랙만 FX에 반응하는 것처럼 들림 | 위와 같은 원인(FX 갱신도 두 엔진이 나눠 받음). 채널 FX는 모노·스테레오·멀티 구분 없이 같은 채널이면 똑같이 걸린다 | `test_multichannel_passthrough` |
| 스피커 없는 채널이 헤드폰에 섞임 | 하나라도 배치했으면 배치 안 된 채널은 미리듣기에서 뺀다 | `test_binaural_active_room` |
| 헤드폰 전체 음량이 작음 | HRTF 보정 +6dB 메이크업(기준 거리 정면 = 원음 0dB) | `test_headphone_preview_level` |
| 스피커 드래그 중 끊김 | 전파 지연은 새 거리가 0.15초 유지될 때만 옮김, 공기 흡음 감쇠 계단 제거 | `test_binaural_propagation`, `test_binaural_field_physics` |
| CH2를 옮기면 FX가 따라가는지 | 정상(게인·지연·서브 지연·EQ 모두 추종) | `test/fx_follow_main_with_sub_test.dart` |

## 재시작 누적 실측 (2026-10-01, Scarlett 6i6 12ch 48kHz, 워치독과 같은 순서로 재시작)

| 재시작 1회당 | 수정 전 (cpal 0.15.3) | 수정 후 |
|---|---|---|
| 풀리지 않은 옛 엔진 | +1 | 0 |
| 실제 힙 사용 | +52MB | 0 (엔진 1개 57.7MB 고정) |
| 상주 메모리(RSS) | +30MB | 30회 동안 약 110MB에서 멈춤(해제 후 할당자가 붙잡은 분량) |
| 스레드 | +4 | 0 (12~13개) |
| CPU | +0.35%p | 0 (약 24.5%) |

수정 전이면 재시작 100번에 힙 약 5GB·스레드 400개가 쌓이는 셈이었다. 워치독은 콜백 공백 1초, 장치 목록 변화(3초마다 검사), 장치 분리 때 재시작한다. 재현: `rust/tests/diag_engine_restart_leak.rs`(파일 머리 주석에 실행법).

## 실기 검증 (2026-10-04~05, Scarlett 6i6 12ch, 실제 config 복사본)

실제 config(`~/Library/Application Support/com.example.atmosMixerPro/config.json`)와 앱 설정(SharedPreferences)을 복사해 같은 트랙·장치·전시 모드로 확인했다. 원본은 건드리지 않았다. 복사본에서 바꾼 것은 세 가지다.
- 방·트랙마다 다른 OSC 주소, 테마 시작·시스템 리셋 주소
- 방 2 첫 트랙을 루프로(다음 방 BGM 자동 재생 경로를 보려고)
- 테마 1 BGM 출력을 CH1(서브)에서 CH2(메인)로(장시간 실행만)

검증에 쓴 테스트는 일회용이라 지웠다.

| 확인 | 결과 |
|---|---|
| OSC 테마 시작 | ✅ 방 1 활성, BGM 재생 |
| OSC로 단발 트랙 재생(미리 로드) | ✅ |
| OSC로 루프·스트리밍 트랙 재생 | ❌ 아무것도 안 나옴 → 2026-10-05 고침(최근 끝난 일) |
| 재생 중 자기 재시작 | ✅ 멈춘 위치부터 재개. 재개 대기 중 디지털 무음. 재시작 약 0.7초 + 대기 0.13~0.18초 |
| 재시작 직전 OSC 정지 | ✅ 정지한 트랙은 안 살아남 |
| 재개 대기 중 OSC 방 비우기 | ✅ 옛 방 재개 안 됨, 다음 방 활성 · ❌ 다음 방 BGM 자동 재생 안 됨 → 2026-10-05 고침 |
| 재개 뒤에 온 OSC 방 비우기 | 옛 방 트랙이 잠깐 재개됐다가 방 비우기로 멈춤(최종 상태 정상) |
| OSC 시스템 리셋 | ✅ 무음 |
| `ATMOS_TRACE_CMD` 로그 대조 | ✅ 엔진 종료·시작 세대가 짝을 이루고 재개 재생이 새 세대 뒤에 나온다. 재동기화 명령이 두 번씩 나간다(남은 일 4) |
| 테마 루프 65분(BGM 11.6초 루프, 44.1kHz → 48kHz 리샘플) | ✅ 엔진 재시작 0, 무음 구간 0, BGM 빠짐 0, VU 공백 0. CH2 레벨 −29.4~−23.6dBFS로 유지 · ⚠️ 메모리 912→1059MB(남은 일 2) |

## 확정된 결정 (다시 제안하지 말 것)

- 크로스오버 **80Hz · LR24(24dB/oct) 유지**. 서브 저음이 메인 자리에서 들리는 건 의도된 상태("서브가 메인 뒤로 사라짐"). 120/150Hz 상향, 48dB/oct 모두 하지 않는다.
- 채널 번호는 사용자에게 CH1부터 말한다(내부 channel은 0부터).

## ⚠️ Windows 미검증 (실기에서 먼저 확인할 것)

이 저장소 작업은 전부 macOS에서만 이뤄졌다. Windows용 크로스 컴파일도 막혀 있다 — `cargo check --target x86_64-pc-windows-msvc`조차 `rusqlite`(bundled sqlite3)·`dart-sys`의 네이티브 C 빌드 스크립트가 MSVC 헤더(`assert.h`, `stdlib.h`)를 못 찾아 실패한다(2026-10-01 확인). 최소 검증도 실제 Windows 컴퓨터가 있어야 한다. 이 저장소에는 Windows CI 워크플로우도 없다(`git log`로도 한 번도 존재한 적 없음) — 과거 "Windows CI 빌드 에러 수정" 커밋들은 실기에서 수동으로 잡은 것으로 보인다.

Windows 실기에서 먼저 빌드·실행해 확인해야 하는 항목:
1. **cpal 0.15.3 → 0.16.0 업그레이드**(macOS 스트림 누수 수정 목적). ASIO/WASAPI 경로는 cpal 내부 구현이 다르므로 Windows에서 컴파일·동작 여부가 전혀 확인 안 됐다. 과거에도 cpal 마이너 업그레이드에서 Windows만 깨진 이력이 있다.
2. **엔진 세대 가드**(`engine.rs` `begin_callback`/`ENGINE_GENERATION`) — 네 콜백(F32/I16/I32/U16) 전부와 Windows 전용 코드(ASIO COM 초기화 등)가 같은 함수 안에 있다. 컴파일은 될 가능성이 높지만 실기 검증은 없다.
3. **통합 테스트의 `device_name: null`(기본 장치)**. Windows는 cpal 기본 장치가 WASAPI이고 ASIO는 항상 이름을 명시해야 잡힌다(`get_hosts`의 `[ASIO]`/`[WASAPI]` 접두사 분기). 설계는 일관되지만 테스트가 Windows에서 그대로 돌아가는지는 별도 확인 필요.
4. **Flutter integration_test(app_flow_test) 자체가 macOS 전용이다.** 실행이 `-d macos`로 고정돼 있고, 스피커 탭 재현에 쓰는 `SpeakerBridge` WebView 채널은 `webview_flutter`의 macOS 구현(WKWebView)을 탄다 — Windows는 별도 플러그인(WebView2)이라 같은 방식으로 동작하는지 미확인. 재시작 감지(`err_fn`)는 `"DeviceNotAvailable"`(공통)과 `"kAsioResetRequest"`(Windows ASIO)를 둘 다 같은 `device_needs_reset` 플래그로 받게 짜여 있어, 그 이후 스냅샷·복원 로직 자체는 플랫폼 무관하게 설계됐다. 다만 Windows에서 실제로 그 경로가 타는지, WebView2로 탭이 똑같이 재현되는지는 확인된 적 없다.
5. **재시작 복원(ASIO/WASAPI 재기동 경로)** — 2026-10-03 구현은 macOS 실기로만 검증했다(4번 참고).
6. **절전 방지(`SetThreadExecutionState`, 2026-10-05)** — `windows` 크레이트에 `Win32_System_Power` 기능을 더했다. API 이름·시그니처는 windows 0.58 소스로 확인했지만 컴파일·동작은 미확인. 실기에서는 관리자 명령 프롬프트의 `powercfg /requests` SYSTEM 항목에 앱이 보이면 걸린 것이다.
7. **로그 내보내기 바탕화면 경로(2026-10-05)** — PowerShell로 `[Environment]::GetFolderPath('Desktop')`을 묻는다. OneDrive로 바탕화면이 옮겨진 PC에서 맞는 폴더에 저장되는지 확인 필요.
8. **installer 버전(2026-10-05)** — `installer.iss`가 빌드된 exe의 `ProductVersion`을 읽는다(ISPP `GetStringFileInfo`). exe 경로는 기존 `[Files]`와 같은 기준(스크립트 폴더)이다. 그런데 `[Files]`의 `build\...` 경로는 스크립트가 있는 `windows\` 폴더 기준이라 그대로는 파일을 못 찾을 수 있다(예전부터 그랬다). 설치 파일을 만들 때 실제로 어떻게 빌드하는지 확인 필요.

오디오 인터페이스 인식 설계는 macOS·Windows 둘 다 하드코딩 없이 그 순간 시스템에 등록된 장치를 스캔하는 구조로 일관되다(`get_hosts()` — macOS는 CoreAudio만, Windows는 ASIO+WASAPI 둘 다 스캔). 이 부분은 설계 검토 완료, 실기 검증만 남음.

## 남은 일

1. **Windows CI 빌드 확인(사용자)**: CI는 태그(`v*`) 푸시 때만 돌고 마지막이 2026-08-26이다. 그 뒤 cpal 0.16(ASIO), 재시작 복원, 절전 방지(`Win32_System_Power`)가 Windows에서 컴파일되는지 아무도 확인하지 않았다. GitHub Actions의 "Build and Release" → "Run workflow"(main)로 실행한다. 릴리스 단계는 태그에서만 돌아 배포는 일어나지 않는다. Claude는 이 실행이 권한상 막혀 있다(자동 모드 분류기: 공개 표면 생성).
2. **Windows PC 실기 확인(사용자가 Windows PC 앞에서)**: 위 "⚠️ Windows 미검증" 목록 전부. 특히 다음 세 가지다.
   - 앱 기동과 3D 방 뷰어: `webview_flutter`에 Windows 구현이 없어 동작하지 않을 가능성이 가장 크다.
   - ASIO·Dante 장치 스캔과 12ch 출력.
   - 장치를 뽑았다 꽂는 재시작.
3. **충돌 후 자동 재실행(Windows, 설계 필요)**: 로그인 때 자동 실행은 installer가 이미 등록한다(HKCU Run 키). 앱이 죽었을 때 다시 띄우는 것은 없다. 작업 스케줄러 또는 작은 감시 프로그램 중 방식을 정해야 한다.
4. **트랙 경로 이식성(설계 필요)**: 프로젝트 파일(.atmos)은 엔진 설정 JSON이라 트랙 오디오 경로가 macOS 절대 경로(`/Users/...`)로 저장된다. Windows PC에서 열면 파일을 못 찾는다. 오디오 파일을 현장 PC로 옮기는 방식에 맞춰, 프로젝트 폴더나 지정한 오디오 폴더에서 파일 이름으로 다시 연결하는 방식을 정해야 한다. 도면 이미지 경로도 같다.
5. **현재 config 정리(사용자 확인)**
   - OSC 주소가 겹친다. 모든 트랙이 `/play`·`/stop`, 모든 방이 `/room/clear`이고, 테마 시작·시스템 리셋 주소는 비어 있다. 앱은 같은 주소면 마지막 것만 기억한다. 그래서 `/room/clear`는 빈 방 '5전시'를 비우고, `/play`는 방 2 마지막 트랙만 튼다. OSC로 방·트랙을 제어할 수 없는 상태다. 앱에 중복 주소 경고가 있으면 좋겠다.
   - 테마 1 BGM이 CH1로 나가는데 스피커 배치에서 CH1이 서브다. 그래서 80Hz 아래만 나간다(헤드폰으로 거의 안 들림, −45~−49dBFS). 메인(CH2)으로 보내려던 것인지 확인 필요.
   - 10분짜리 4채널 24bit WAV(테마 1 세 번째 트랙)를 통째로 RAM에 올린다(약 470MB). 긴 단발 트랙은 스트리밍으로 두는 편이 낫다.
6. **메모리 증가 원인 가리기**: 65분 동안 RSS가 912→1059MB로 분당 2.2MB(시간당 약 133MB) 꾸준히 늘었다. 12시간이면 약 1.6GB다. 디버그 빌드에 테스트 하네스가 같이 돈 측정이라 Rust 엔진인지 Dart·테스트 쪽인지 아직 모른다. 스트리밍 묶음 GC 채널은 전용 스레드가 비우고 있어 그 경로는 아니다. Rust만 도는 장시간 루프(`diag_*` 같은 수동 테스트)와 릴리스 앱으로 다시 재서 가린다.
7. **실제 장치를 뽑았다 꽂는 재시작(macOS)**: 결함 주입·워치독 경로는 확인했지만, Scarlett 케이블을 실제로 뽑았다 꽂을 때(장치 유실 → 기본 장치로 비상 전환 → 다시 연결)는 아직이다. 사용자가 장치 옆에 있어야 한다.
8. **사용자 결정이 필요한 발견**(통합 테스트를 만들며 발견, 고치지 않음):
   - 대시보드 Start는 모든 방의 루프를 틀고, OSC 테마 시작은 첫 방의 루프만 튼다. 의도된 차이인지 확인 필요.
   - 기본 공간 리버브(Hall 3.2초, 80%)가 하드웨어 CH1/CH2 마스터 버스에 걸린다 — 서브로 지정한 채널에도. 정지 뒤 약 4초 꼬리가 남고 CH2 소리가 CH1으로 샌다. 테스트는 픽스처에서 이 리버브를 끈다.
   - 1024×768 기본 창에서 인스펙터의 LFE 스위치가 아래 오버레이에 가려 탭되지 않는다(탭 위치 y=678 경고). 테스트는 스위치의 `onChanged`를 직접 부른다.
   - FX 자동 동기화(`acousticSyncProvider`)를 `main.dart`와 `dashboard_screen.dart` 두 곳에서 watch한다. 대시보드가 스피커 화면 아래 스택에 남아 있어 둘 다 지워야 3단계가 실패한다(대시보드 쪽은 중복).
   - 재시작 복원된 소리는 새 엔진의 부팅 뮤트 램프(3초, `mixer.rs` `StartupMuteRamp`)를 타고 커진다. 재개 순간 약 −15dB에서 시작해 3초 뒤 원래 크기다. 자기 재시작에서 이 램프를 줄일지(재개에는 이미 300ms 페이드 인이 있다) 결정이 필요하다.
   - (기존) 오디오 스레드 3법칙 위반: 재생 명령의 Box 해제(`engine.rs` PlayTrack), `target_bands.clone()`, `ApplyAllChannelTunings`의 해제, 큰 블록에서 `temp_buf.resize`, 풀·GC 채널이 가득 찰 때 인스턴스를 오디오 스레드에서 버림(스트리밍이면 디코더 스레드 join), `ATMOS_TRACE_CMD`의 `eprintln!`, 음소거·페일오버·헤드룸·출력 채널·위상의 즉시 전환.
   - (기존) 스트리밍 단발(루프 아님) 트랙은 파일이 끝나도 재생 중으로 남는다. 재시작 복원이 이것까지 매번 다시 튼다.
   - 긴 파일을 먼 위치에서 재개하면 시작 위치까지 디코딩해 버리는 시간만큼(리샘플이 필요한 파일은 수 초) 늦게 나온다. `DiskStreamer::new_at`에 symphonia seek를 쓰면 줄일 수 있다.
   - 자기 재시작 때 Dart 재동기화가 두 번 돈다(`EngineReady`와 `EngineRestarted` 둘 다).
9. **Windows 채널 이름(ASIO)**: 사용자가 나중에 하기로 보류. `rust/src/audio/channel_names.rs`의 실행 계획(A안 asio-sys 패치, B안 직접 FFI)은 cpal 0.16.0에서도 유효하다(asio-sys 0.2.6 그대로).

## 운용 메모

- 스테레오·멀티 트랙은 서브우퍼가 아닌 채널부터 라우팅한다. 서브로 간 채널은 크로스오버 아래만 남는다.
- 추적 로그: `ATMOS_TRACE_CMD=1`로 앱을 실행하면 재생/정지 요청, 인스턴스 정리, 엔진 시작/종료(세대), 베이스 라우팅이 stderr에 찍힌다.
- 수동 진단(자동 테스트 아님): `rust/tests/diag_theme1_track4.rs`, `rust/tests/diag_theme1_multitrack.rs`, `rust/tests/diag_engine_restart_leak.rs` — 파일 머리 주석에 실행법.
- 디스크 여유가 적으면 전체 `cargo test`가 ENOSPC로 실패한다(테스트 바이너리마다 최적화+디버그 정보라 수 GB 필요).
- **프로젝트 폴더에서 `flutter run -d macos`로 띄운 앱이 옛 Rust를 싣던 문제는 고쳤다(c7d2fd2).** FRB 기본 로더는 작업 디렉터리 기준 `rust/target/release/librust_lib_atmos_mixer_pro.dylib`을 앱 번들보다 먼저 연다. 그래서 `cargo build --release`로 남은 옛 빌드(2026-09-28)가 실렸다 — API가 같으면 조용히, 다르면 "Content hash … different" 오류로. 이제 macOS의 `main.dart`가 번들 프레임워크를 직접 연다. 2026-10-03 이전에 `flutter run`으로 확인한 동작은 옛 Rust였을 수 있다.
- 엔진을 (재)기동하면 출력은 부팅 뮤트 램프(3초)로 0에서 커진다. 재시작 직후 레벨을 잴 때는 3초 뒤에 잰다.
- 통합 테스트는 기계가 바쁘면(예: Claude 앱 화면이 CPU를 크게 쓸 때) VU 전달이 들쭉날쭉해진다. 5단계는 재개 대기 구간(약 100ms) 안의 표본 수가 아니라 앞뒤 0.5초 구간으로 VU 스트림이 살아 있는지 본다.
