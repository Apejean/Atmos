# 작업 인계 (2026-10-03 기준)

새 세션에서 "docs/HANDOFF.md 보고 이어서 해줘"로 시작하면 된다. 설계 결정의 근거는 각 스펙 문서와 테스트 주석에 있다.

## 최근 끝난 일

| 항목 | 결과 | 확인하는 테스트 |
|---|---|---|
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

오디오 인터페이스 인식 설계는 macOS·Windows 둘 다 하드코딩 없이 그 순간 시스템에 등록된 장치를 스캔하는 구조로 일관되다(`get_hosts()` — macOS는 CoreAudio만, Windows는 ASIO+WASAPI 둘 다 스캔). 이 부분은 설계 검토 완료, 실기 검증만 남음.

## 남은 일

1. **실제 전시 장비·설정으로 확인**(작업 순서 5단계의 나머지): 통합 테스트는 시스템 기본 출력 장치·합성 톤·12m 방 픽스처로 돈다. Scarlett 6i6 12ch와 실제 전시 config에서 재시작 복원(특히 OSC 룸 전환과 겹칠 때)·테마 시작 루프, `ATMOS_TRACE_CMD` 로그 대조는 아직이다.
2. **사용자 결정이 필요한 발견**(통합 테스트를 만들며 발견, 고치지 않음):
   - 기본 공간 리버브(Hall 3.2초, 80%)가 하드웨어 CH1/CH2 마스터 버스에 걸린다 — 서브로 지정한 채널에도. 정지 뒤 약 4초 꼬리가 남고 CH2 소리가 CH1으로 샌다. 테스트는 픽스처에서 이 리버브를 끈다.
   - 1024×768 기본 창에서 인스펙터의 LFE 스위치가 아래 오버레이에 가려 탭되지 않는다(탭 위치 y=678 경고). 테스트는 스위치의 `onChanged`를 직접 부른다.
   - FX 자동 동기화(`acousticSyncProvider`)를 `main.dart`와 `dashboard_screen.dart` 두 곳에서 watch한다. 대시보드가 스피커 화면 아래 스택에 남아 있어 둘 다 지워야 3단계가 실패한다(대시보드 쪽은 중복).
   - 재시작 복원된 소리는 새 엔진의 부팅 뮤트 램프(3초, `mixer.rs` `StartupMuteRamp`)를 타고 커진다. 재개 순간 약 −15dB에서 시작해 3초 뒤 원래 크기다. 자기 재시작에서 이 램프를 줄일지(재개에는 이미 300ms 페이드 인이 있다) 결정이 필요하다.
   - (최종 리뷰에서 나온 기존 문제) 방 비우기(대시보드·OSC)가 **모든 방**의 재생 목록을 지운다. 엔진은 그 방만 멈추므로, 이후 자기 재시작 때 다른 방 트랙이 재개되지 않고 OSC 자동 승격이 다음 방 BGM을 두 번 틀 수 있다.
   - (기존) 오디오 스레드 3법칙 위반: 재생 명령의 Box 해제(`engine.rs` PlayTrack), `target_bands.clone()`, `ApplyAllChannelTunings`의 해제, 큰 블록에서 `temp_buf.resize`, 풀·GC 채널이 가득 찰 때 인스턴스를 오디오 스레드에서 버림(스트리밍이면 디코더 스레드 join), `ATMOS_TRACE_CMD`의 `eprintln!`, 음소거·페일오버·헤드룸·출력 채널·위상의 즉시 전환.
   - (기존) 스트리밍 단발(루프 아님) 트랙은 파일이 끝나도 재생 중으로 남는다. 재시작 복원이 이것까지 매번 다시 튼다.
   - 긴 파일을 먼 위치에서 재개하면 시작 위치까지 디코딩해 버리는 시간만큼(리샘플이 필요한 파일은 수 초) 늦게 나온다. `DiskStreamer::new_at`에 symphonia seek를 쓰면 줄일 수 있다.
   - 자기 재시작 때 Dart 재동기화가 두 번 돈다(`EngineReady`와 `EngineRestarted` 둘 다).
3. **Windows 채널 이름(ASIO)**: 사용자가 나중에 하기로 보류. `rust/src/audio/channel_names.rs`의 실행 계획(A안 asio-sys 패치, B안 직접 FFI)은 cpal 0.16.0에서도 유효하다(asio-sys 0.2.6 그대로).

## 운용 메모

- 스테레오·멀티 트랙은 서브우퍼가 아닌 채널부터 라우팅한다. 서브로 간 채널은 크로스오버 아래만 남는다.
- 추적 로그: `ATMOS_TRACE_CMD=1`로 앱을 실행하면 재생/정지 요청, 인스턴스 정리, 엔진 시작/종료(세대), 베이스 라우팅이 stderr에 찍힌다.
- 수동 진단(자동 테스트 아님): `rust/tests/diag_theme1_track4.rs`, `rust/tests/diag_theme1_multitrack.rs`, `rust/tests/diag_engine_restart_leak.rs` — 파일 머리 주석에 실행법.
- 디스크 여유가 적으면 전체 `cargo test`가 ENOSPC로 실패한다(테스트 바이너리마다 최적화+디버그 정보라 수 GB 필요).
- **프로젝트 폴더에서 `flutter run -d macos`로 띄운 앱이 옛 Rust를 싣던 문제는 고쳤다(c7d2fd2).** FRB 기본 로더는 작업 디렉터리 기준 `rust/target/release/librust_lib_atmos_mixer_pro.dylib`을 앱 번들보다 먼저 연다. 그래서 `cargo build --release`로 남은 옛 빌드(2026-09-28)가 실렸다 — API가 같으면 조용히, 다르면 "Content hash … different" 오류로. 이제 macOS의 `main.dart`가 번들 프레임워크를 직접 연다. 2026-10-03 이전에 `flutter run`으로 확인한 동작은 옛 Rust였을 수 있다.
- 엔진을 (재)기동하면 출력은 부팅 뮤트 램프(3초)로 0에서 커진다. 재시작 직후 레벨을 잴 때는 3초 뒤에 잰다.
