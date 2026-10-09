# 작업 인계 (2026-10-07 기준)

새 세션에서 "docs/HANDOFF.md 보고 이어서 해줘"로 시작하면 된다. 설계 결정의 근거는 각 스펙 문서와 테스트 주석에 있다.

> ⚠️ **현장(전시 운영) PC는 Windows다**(사용자 확인 2026-10-05). 개발·검증은 macOS에서 했다. macOS에서 확인한 결과를 현장 동작 보증으로 보지 말고, 새 변경마다 Windows에서도 되는지(경로, ASIO/WASAPI, 바탕화면, 웹뷰, 절전)를 먼저 따진다. 남은 일은 Windows 기준 우선순위로 정리했다.

## 최근 끝난 일

| 항목 | 결과 | 확인하는 테스트 |
|---|---|---|
| Windows(RME UFX+ 94채널 ASIO)에서 소리가 "버퍼가 안 맞는 것처럼" 늘어지고 리버브가 크게 걸림, 창을 닫을 때마다 크래시(2026-10-08~09, 브랜치 `win/audio-live-channels`) | ① **비정규 실수(denormal)**: 소리가 멈춘 뒤 리버브·필터·리미터 꼬리가 비정규 실수가 되면 x86 CPU가 그 계산을 수십 배 느리게 한다. 이 PC Release 벤치(40채널·1024프레임·채널 리버브 홀 80%, 1초 잡음 뒤 무음): 콜백 처리 시간 평균 11ms → 12초 뒤 22ms → 44초 뒤 최대 98ms(예산 21.3ms), 90초까지 62ms. 오디오 콜백(네 샘플 형식)과 분석 스레드에서 FTZ·DAZ를 켠다(`audio/denormal.rs` `DenormalGuard`, 콜백이 끝나면 드라이버 스레드의 원래 값으로 돌린다) → 같은 벤치에서 내내 11ms. Apple Silicon은 비정규 실수가 느려지지 않아 아무것도 하지 않는다(macOS에서 드러나지 않은 이유로 보인다). ② **마스터 리버브**: ALL OUTPUTS 리버브(기본 홀 80%)가 마스터 리버브로 하드웨어 CH1·CH2에 따로 걸려 CH1·CH2를 0%로 내려도 남았다 → 걸지 않고 기본 MIX 0%(확정된 결정). Dart는 마스터 리버브 명령을 보내지 않는다. ③ **처리 폭**: 장치는 94채널로 열고 믹서는 출력 설정에서 켠 가장 높은 채널(이 PC CH40)까지만 처리하며 나머지는 무음이다(`core/processing_channels.rs`). 멀티 그룹 규칙 때문에 94채널을 모두 처리해 1024프레임 예산의 77%를 쓰던 것. 출력 설정이 바뀌어 폭이 달라지면 엔진을 다시 시작한다. ④ **WASAPI 장치 감시**: 3초마다 ASIO 드라이버를 전부 불러와 Generic Low Latency ASIO가 계속 떴다 → Windows에서는 WASAPI 장치만 본다. ⑤ **창 닫기 크래시**: 창을 닫을 때마다 `flutter_windows.dll`+0x14F27(`FlutterDesktopMessengerSetCallback` 안) 접근 위반(0xC0000005)으로 끝났다. webview_windows 플러그인이 소멸하면서 남은 웹뷰의 채널 처리기를 엔진이 없어진 뒤 해제했다(`~WebviewBridge`) → 창을 닫기 전에 3D 웹뷰를 dispose한다(`ThreeJsEngineService.disposeWindowsWebview`, `main.dart` `onWindowClose`, 최대 1초). ⑥ 앱 로그 "오디오 처리 부하"(운용 메모). **이 PC 실행 확인**(Release, 2026-10-09 01:01): 엔진 "94채널 중 40채널 처리", 무음(All Mute) 재생·정지 동안 오디오 스레드 코어 35~39% 일정·부하 경고 없음, 창 닫기 종료 코드 0(같은 조작이 수정 전 4번 모두 0xC0000005 — 10-08 20:51·20:58, 10-09 00:52·00:58), Windows 오류 기록(APPCRASH) 없음. WASAPI로 바꿔 Generic Low Latency ASIO가 더 뜨지 않는 것은 사용자가 눈으로 확인했다. 사람이 들어 확인은 대기. **이 PC 빌드·테스트**: `cargo test --no-fail-fast` 실행 파일 86개 237 통과·0 실패·7 무시, `flutter test` 164 통과, `flutter analyze` 0건, `cargo clippy --lib` 경고 29건(모두 기존 코드, 바꾼 코드 0). macOS는 확인하지 않았다(비정규 실수 가드는 Apple Silicon에서 아무것도 하지 않고, 마스터 리버브 제거·기본 0%는 macOS에도 적용된다). 하루 동안 사용자가 RME 버퍼를 바꾸며 녹음해 Windows 일반 소리도 늘어졌는데, 그것은 앱과 별개였다(운용 메모). 측정 로그 `D:\dev\logs\2026-10-08~09\W6_*` | `test_denormal_guard`(2), `test_master_reverb_removed`(3, 고치기 전 2 실패), `test_processing_channels`(6), `test/spatial_reverb_test.dart`(기본 0%) |
| 풀·정리 채널이 가득 찰 때 인스턴스를 오디오 콜백에서 버리던 것(3법칙) | ① 끝난 인스턴스를 정리 채널로 못 보내면(채널 가득 참) 칸에 되돌려 다음 블록에 다시 보낸다. 받는 쪽이 끝난 경우(Disconnected — 정리 스레드 없음, 시험용 믹서 등)는 되돌려도 영영 못 보내므로 예전처럼 그 자리에서 해제한다(실제 앱에서는 정리 스레드가 믹서보다 먼저 끝나지 않는다). ② 풀(4096칸)이 가득 차면 새 재생 명령의 인스턴스를 콜백에서 버리지 않고 정리 스레드로 넘긴다 → 정리 스레드가 해제하고 재생 목록의 id도 지운다(예전에는 한 번도 울리지 않은 id가 재생 목록에 남아 재시작 복원이 틀 수 있었다). 정리 채널까지 가득 찬 극단적인 경우에만 돌려받은 인스턴스가 콜백에서 해제된다. 쓰지 않던 코드(빈칸 교체 때 옛 값 정리)는 구조를 바꾸며 빠졌다. 위험도: 풀 4096칸·정리 채널 8192칸을 전용 스레드가 비우므로 실제로는 거의 일어나지 않고, 채울 수 있던 누적 원인(끝난 스트리밍 단발이 풀에 남음)은 위 행으로 이미 고쳤다. 오디오 콜백에 더한 것은 `try_send`의 Full 분기 처리와 빈칸 탐색뿐(할당·잠금 없음), `rust/src/api/` 변경 없음. 검증(Main): `test_instance_gc_never_drops_in_callback` 고치기 전 2 실패 → 고친 뒤 2 통과(정리 채널이 가득 차면 칸에 남았다가 자리가 나면 넘어감 / 풀이 가득 차면 새 인스턴스가 정리 채널로 감). 첫 전체 실행에서 `test_field_scenario_bass_balance` 1건이 실패했다(받는 쪽을 버린 믹서를 쓰는데 처음 수정이 Disconnected에도 되돌려 정지한 트랙이 남음) → Full일 때만 되돌리도록 고친 뒤 통과. `cargo test --no-fail-fast` 전체 실행 파일 83개 233 통과·0 실패·9 무시, clippy lib 경고 26건 그대로. 장치 실기는 하지 않았다(오프라인 믹서 시험 경로) | `test_instance_gc_never_drops_in_callback`(2개), `test_field_scenario_bass_balance` |
| 끝난 스트리밍 단발이 재생 중으로 남음(재시작 복원이 매번 다시 틀던 것) | 디코더 스레드(`DiskStreamer`)는 파일 끝에서 링 Producer를 버리고 끝나는데, 믹서는 링이 비면 "디코더가 늦는 중"으로만 보고 기다려 재생 목록에 영원히 남았다(미리 불러온 단발은 파일 끝에서 멈추는 경로가 있어 정상). 이제 링이 빈 순간 보내는 쪽이 끝났고(rtrb `Consumer::is_abandoned`) 남은 묶음도 없으면(`is_empty`) 정지 페이드를 걸고, 기존 정리 경로(GC 스레드가 재생 목록에서 뺀다)가 처리한다. 보내는 쪽이 끝난 뒤에는 더 들어오지 않아 마지막 묶음을 놓치지 않는다. 루프는 디코더가 계속 살아 있어 영향 없고, 파일을 다시 열지 못해 디코더가 죽은 루프는 전에는 무음으로 영원히 남았지만 이제 멈춘다. 오디오 콜백에 더한 것은 원자 읽기 두 개뿐이라 3법칙 위반 목록은 늘지 않았다. 검증(Main): 고치기 전 1 통과·2 실패 → 고친 뒤 3 통과(끝나면 정리 / 늦을 뿐이면 안 멈추고 다음 묶음 재생 / 남은 묶음 1.5초를 끝까지 재생한 뒤 정리), 관련 기존 시험 통과, `cargo test` 전체 실행 파일 82개 231 통과·0 실패·9 무시, clippy lib 경고 26건 그대로. 장치 실기는 하지 않았다(오프라인 믹서 시험으로 확인되는 경로) | `test_stream_oneshot_ends`(3개), `test_stream_resume_onset`, `test_disk_streamer_start_offset`, `test_restart_resume`, `test_restart_resume_races`, `test_show_resume` |
| 프로젝트를 열 때 트랙·도면 경로 다시 연결(남은 일 4번 구현, PR #10으로 2026-10-06 병합) | **동작**: ① 저장된 경로에 파일이 있으면 그대로 쓴다. ② 없으면 `.atmos`가 있는 폴더와 그 하위 폴더에서 같은 파일 이름을 찾아 연결한다(대소문자 무시, 경로 구분자 `/`·`\` 모두 인식, 같은 이름이 여러 개면 원래 경로와 상위 폴더 이름이 가장 많이 겹치는 파일, 같으면 얕은 쪽, 숨김 파일·폴더(`._` 포함)는 보지 않고 16단계 깊이까지). ③ 그래도 못 찾은 파일이 있으면 "파일 N개를 찾지 못했습니다" 대화상자를 띄운다. "폴더 고르기"는 고른 폴더에서 같은 방법으로 찾고, "그대로 열기"는 그대로 연다. ④ 끝까지 못 찾은 파일은 원래 경로를 두고, 연 뒤 목록을 보여 주며 앱 로그에도 남긴다. ⑤ 다시 연결한 경로는 이 PC의 `config.json`과 도면 설정에 저장한다(저장 방식은 그 PC의 절대 경로로 그대로). ⑥ macOS 메뉴와 Windows 메뉴에 두 번 들어 있던 "Load Project" 코드를 대시보드의 `_loadProject` 하나로 합쳤다. **파일**: `rust/src/core/media_relink.rs`(`MediaFinder`, `relink_tracks`, `file_name_of`), `rust/src/api/project.rs`(`api_relink_track_paths`, `api_find_media`, `RelinkedConfig`), `lib/core/state/project_media.dart`, `lib/features/dashboard/widgets/missing_media_dialog.dart`, `dashboard_screen.dart`. **검증(구현 세션 보고)**: Rust 새 테스트 6개, `flutter test` 162개, 실제 앱 런타임 통합 테스트 2개(다시 연결 → 엔진이 그 경로의 오디오를 미리 읽음, 폴더 묻기 경로), `app_flow_test` 0~6단계 회귀 통과. 대화상자의 "폴더 고르기"를 실제로 누르는 것과 Windows에서 여는 것은 확인하지 못했다. | `test_media_relink`(Rust 6개), `test/project_media_test.dart`(4개), `integration_test/project_media_relink_test.dart`(2개) |
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
| 테마 루프 65분(BGM 11.6초 루프, 44.1kHz → 48kHz 리샘플) | ✅ 엔진 재시작 0, 무음 구간 0, BGM 빠짐 0, VU 공백 0. CH2 레벨 −29.4~−23.6dBFS로 유지 · ⚠️ 메모리 912→1059MB(남은 일 6) |

## 확정된 결정 (다시 제안하지 말 것)

- 크로스오버 **80Hz · LR24(24dB/oct) 유지**. 서브 저음이 메인 자리에서 들리는 건 의도된 상태("서브가 메인 뒤로 사라짐"). 120/150Hz 상향, 48dB/oct 모두 하지 않는다.
- 채널 번호는 사용자에게 CH1부터 말한다(내부 channel은 0부터).
- **프로젝트를 옮기는 방식(2026-10-05).** 오디오와 도면을 `.atmos` 파일과 같은 폴더(하위 폴더 가능)에 담아 옮긴다. 설계는 채팅으로 승인받았고 별도 스펙·계획 문서는 없다.
- **현장 PC는 Windows다(2026-10-05).** 지금 되는 기능은 Windows에서도 동일하게 동작해야 하고, **3D 방 뷰어는 무조건 Windows에 들어간다.** 메뉴 형태·종료 경로·절전 방지와 로그 경로 구현 방식 같은 OS 고유 차이는 허용한다. 설계는 macOS에서 하고 현장 운영은 Windows에서 한다.
- **공간 리버브(2026-10-09, Windows 실기에서 발견).** 마스터 리버브(옛 `mixer.reverb`, ALL OUTPUTS 설정으로 하드웨어 CH1·CH2에만 걸리던 것)는 출력에 걸지 않는다. 리버브는 채널별 리버브만 쓴다(바이노럴 처리 앞이라 헤드폰 미리듣기에도 들어간다). 새 설치·새 프로젝트의 ALL OUTPUTS MIX 기본값은 0%다(리버브는 사용자가 올려야 걸린다). 이미 저장된 설정은 바꾸지 않는다.
- **Windows 설치 파일(2026-10-08, 하네스 W4).** 현장 PC는 Windows 10 이상이고 설치 때 인터넷이 없을 수 있다. 그래서 설치 파일에 Visual C++ 런타임(최신 지원판 `VC_redist.x64.exe`)과 WebView2 런타임 Evergreen Standalone(오프라인 설치 파일)을 넣고, 없거나 오래됐을 때만 조용히 설치한다(Microsoft 재배포 약관 동의). OSC 방화벽 규칙을 설치 때 넣는다: 개인·도메인 네트워크는 앱의 UDP 수신 허용, 공용 네트워크는 같은 서브넷만. 현장 PC는 고정 IP와 네트워크 "개인"을 권장한다. 코드 서명은 하지 않고 현장 백신에 설치 폴더 예외를 넣는 절차로 간다(나중에 서명 인증서를 살 수 있다). 설치 경로는 기본값(`{autopf}`, 사용자가 바꿀 수 있음)과 "설치 직후 실행" 기본값을 그대로 둔다.
- **스피커 배치 → 채널 FX 자동 계산은 바이노럴과 상관없이 늘 한다(2026-10-09, 지금 동작 유지).** 현장 배치가 도면과 다르면 앱에서 스피커를 실제 자리로 옮기기만 하면 정렬 딜레이·거리 게인·자동 EQ가 따라온다. 귀로 손댄 게인·딜레이·EQ 밴드는 자동 계산이 덮어쓰지 않는다(위상만 자동이 다시 정한다). 다 맞춘 뒤 실수로 바뀌는 게 걱정되면 채널 튜닝 잠금(Lock Tuning)을 건다. 현장 순서: 실제 자리로 옮김 → 귀로 미세 조정 → 필요하면 튜닝 잠금 → 무인 운영. "바이노럴이 켜져 있을 때만 계산"은 구현·검증까지 했으나 커밋하지 않고 접었다 — 현장에서 자리를 고칠 때마다 바이노럴을 켰다 꺼야 하고, 켜 있는 동안 CH1·CH2 스피커로 헤드폰용 렌더가 나가 현장 작업이 늘어난다. 이 논의의 계기였던 "바이노럴을 꺼도 소리가 돈다"는 느낌은 5.1 시험 파일(`591775 surround-test_51_l-r-c-lfe-ls-rs.wav`, L→R→C→LFE→Ls→Rs를 1초씩 차례로 낸다)을 CH1·CH2 스피커만으로 들은 것이었다(왼쪽 1초 → 오른쪽 1초 → 3.5초 조용함 반복). 바이노럴을 끄면 채널 트랙을 스피커 사이로 옮기는 처리(팬)는 없고, 스피커를 옮기면 그 채널의 거리 게인·EQ·딜레이만 바뀐다.

## ⚠️ Windows 미검증 (실기에서 먼저 확인할 것)

이 저장소 작업은 전부 macOS에서만 이뤄졌다. Windows용 크로스 컴파일도 막혀 있다 — `cargo check --target x86_64-pc-windows-msvc`조차 `rusqlite`(bundled sqlite3)·`dart-sys`의 네이티브 C 빌드 스크립트가 MSVC 헤더(`assert.h`, `stdlib.h`)를 못 찾아 실패한다(2026-10-01 확인). 최소 검증도 실제 Windows 컴퓨터가 있어야 한다. 이 저장소에는 Windows CI 워크플로우도 없다(`git log`로도 한 번도 존재한 적 없음) — 과거 "Windows CI 빌드 에러 수정" 커밋들은 실기에서 수동으로 잡은 것으로 보인다.

Windows 실기에서 먼저 빌드·실행해 확인해야 하는 항목:
1. **cpal 0.15.3 → 0.16.0 업그레이드**(macOS 스트림 누수 수정 목적). ASIO/WASAPI 경로는 cpal 내부 구현이 다르므로 Windows에서 컴파일·동작 여부가 전혀 확인 안 됐다. 과거에도 cpal 마이너 업그레이드에서 Windows만 깨진 이력이 있다.
2. **엔진 세대 가드**(`engine.rs` `begin_callback`/`ENGINE_GENERATION`) — 네 콜백(F32/I16/I32/U16) 전부와 Windows 전용 코드(ASIO COM 초기화 등)가 같은 함수 안에 있다. 컴파일은 될 가능성이 높지만 실기 검증은 없다.
3. **통합 테스트의 `device_name: null`(기본 장치)**. Windows는 cpal 기본 장치가 WASAPI이고 ASIO는 항상 이름을 명시해야 잡힌다(`get_hosts`의 `[ASIO]`/`[WASAPI]` 접두사 분기). 설계는 일관되지만 테스트가 Windows에서 그대로 돌아가는지는 별도 확인 필요.
4. **Flutter integration_test(app_flow_test) 자체가 macOS 전용이다.** 실행이 `-d macos`로 고정돼 있고, 스피커 탭 재현에 쓰는 `SpeakerBridge` WebView 채널은 `webview_flutter`의 macOS 구현(WKWebView)을 탄다 — Windows는 별도 플러그인(WebView2)이라 같은 방식으로 동작하는지 미확인. 재시작 감지(`err_fn`)는 `"DeviceNotAvailable"`(공통)과 `"kAsioResetRequest"`(Windows ASIO)를 둘 다 같은 `device_needs_reset` 플래그로 받게 짜여 있어, 그 이후 스냅샷·복원 로직 자체는 플랫폼 무관하게 설계됐다. 다만 Windows에서 실제로 그 경로가 타는지, WebView2로 탭이 똑같이 재현되는지는 확인된 적 없다. → 2026-10-07 Windows에서 `app_flow_test -d windows` 0~6단계 통과(통합 테스트 Windows 이식 PR #27 + 3D 뷰어 남은 일 10, 5단계 재시작 이벤트→재개 105ms).
5. **재시작 복원(ASIO/WASAPI 재기동 경로)** — 2026-10-03 구현은 macOS 실기로만 검증했다(4번 참고).
6. **절전 방지(`SetThreadExecutionState`, 2026-10-05)** — `windows` 크레이트에 `Win32_System_Power` 기능을 더했다. API 이름·시그니처는 windows 0.58 소스로 확인했지만 컴파일·동작은 미확인. 실기에서는 관리자 명령 프롬프트의 `powercfg /requests` SYSTEM 항목에 앱이 보이면 걸린 것이다.
7. **로그 내보내기 바탕화면 경로(2026-10-05)** — PowerShell로 `[Environment]::GetFolderPath('Desktop')`을 묻는다. OneDrive로 바탕화면이 옮겨진 PC에서 맞는 폴더에 저장되는지 확인 필요.
8. **installer 버전(2026-10-05)** — `installer.iss`가 빌드된 exe의 `ProductVersion`을 읽는다(ISPP `GetStringFileInfo`). exe 경로는 기존 `[Files]`와 같은 기준(스크립트 폴더)이다. 그런데 `[Files]`의 `build\...` 경로는 스크립트가 있는 `windows\` 폴더 기준이라 그대로는 파일을 못 찾을 수 있다(예전부터 그랬다). 설치 파일을 만들 때 실제로 어떻게 빌드하는지 확인 필요.

9. **ASIO 재시작 순서와 장치 목록 검사** — ASIO는 한 번에 하나만 열린다. 옛 엔진을 닫고 새 엔진을 여는 순서와, 재생 중 3초마다 하는 장치 목록 검사(`get_hosts`, ASIO 스캔 타임아웃 있음)가 ASIO 사용 중에도 안전한지 확인된 적이 없다. → 2026-10-09: 장치 목록 검사는 ASIO 엔진에서는 돌지 않는다. WASAPI 엔진에서는 3초마다 ASIO 드라이버를 전부 불러와(cpal은 ASIO 목록을 만들 때 드라이버를 하나씩 연다) Generic Low Latency ASIO가 계속 떴다 → Windows에서는 WASAPI 장치만 본다(`api/simple.rs` `monitored_output_device_names`).
10. **작업 집합·스레드 우선순위** — `apply_windows_admin_optimizations`는 관리자 계정일 때만 MMCSS 승격과 작업 집합 500MB~2GB 고정을 건다. 큰 WAV 캐시(10분 4채널 약 470MB)와 메모리 증가가 상한에 닿는지, 일반 계정이면 드롭아웃이 없는지 미확인.
11. **방화벽** — OSC가 `0.0.0.0`으로 열려(`osc/listener.rs`) 무인 PC의 첫 실행에서 허용 창이 뜬다. 허용하지 않으면 외부 OSC가 막힌다. → 2026-10-07 설치본 첫 실행에서 허용 창이 뜨는 것을 확인했다. 2026-10-08 설치 파일이 규칙을 넣도록 했다(위 확정된 결정, `win/installer-prereqs`). 외부 장비에서 OSC가 실제로 들어오는지는 점검표 5절(C 묶음).
12. **메뉴 항목 동일성** — macOS는 네이티브 메뉴(Settings·Preferences 포함), Windows는 Material 메뉴로 따로 구현돼 있다. Windows 메뉴 구간(`dashboard_screen.dart` 415줄~)에서 Settings·Preferences가 보이지 않았다. 화면 다른 곳에 있는지 확인 필요.
13. **종료 경로** — `onWindowClose`는 엔진 정지를 1.5초까지만 기다린다. ASIO 해제에 충분한지 미확인. → 2026-10-07 Windows에서 창을 닫을 때마다 앱이 `flutter_windows.dll` 접근 위반(0xc0000005 → 0xc000041d, 종료 코드 −1073740771)으로 끝나던 것을 고쳤다(`win/fix-close-crash`). `window_manager`의 `destroy()`가 `PostQuitMessage`만 보내 창이 남은 채 소멸자에서 엔진이 창보다 먼저 지워졌다. `windows/runner/main.cpp`에서 메시지 루프가 끝난 뒤 `window.Destroy()`로 보통 닫기와 같은 순서(엔진 → 창)로 정리한다. `clean_exit`는 그 전에 남아 감시 동작은 그대로였다(크래시여도 다시 띄우지 않았다). → 3D 뷰어(WebView2)를 넣은 뒤 창을 닫으면 `flutter_windows.dll`+0x14F27(0xC0000005)로 끝나던 것은 2026-10-09에 고쳤다(최근 끝난 일 첫 행 ⑤, 종료 코드 0 확인).
14. **프로젝트 열 때 미디어 다시 연결** — macOS에서 저장한 `.atmos`와 오디오·도면 폴더를 옮겨 Windows에서 열었을 때 연결되는지(경로 구분자 `\`, 대소문자, 한글·공백 경로), 대화상자와 "폴더 고르기"가 Windows에서 정상인지 확인된 적이 없다. → 2026-10-07 Windows에서 Rust `test_media_relink` 6개와 relink 통합 테스트 2개 통과. 처음 실패는 테스트가 기대 경로를 `/`로 이어 만든 탓이었다(다시 연결된 경로는 폴더를 훑어 `\`로 온다). 테스트만 OS 구분자로 고쳤고 앱 코드는 그대로다(`win/relink-test-separators`). 실제 프로젝트 옮기기는 점검표 9절.
15. **비상 전환과 원래 장치로 돌아오기(2026-10-07, 남은 일 7)** — ASIO 인터페이스 USB를 30초 넘게 뽑으면 WASAPI 기본 장치로 −40dB 비상 전환하고, 다시 꽂으면 ASIO 장치로 원래 크기로 돌아오는지(앱 로그 "비상 전환 해제") 확인한다. 주의: `api_get_output_devices`의 ASIO 목록은 설치된 드라이버 기준이라 하드웨어가 빠져 있어도 있다고 나올 수 있다. 그러면 비상 엔진이 다시 시작할 때마다 ASIO 열기를 먼저 시도하고, 실패하면 바로 다시 비상 전환한다. 이 시도가 빨리 끝나는지(무음이 길어지지 않는지), 반복되지 않는지 실기에서 본다. 또 Windows에서는 ASIO 드라이버가 목록에 있으면 30초 기다림 없이 바로 열기 실패로 비상 전환될 수 있다.

점검 절차와 결과 기록 양식은 `docs/WINDOWS_FIELD_CHECKLIST.md`다(3D 방 뷰어, ASIO, 방화벽, 로그, 메뉴, 프로젝트 파일 이식성, 감시·재실행 포함). Windows PC에서 개발 환경을 갖추고 빌드·점검을 진행하는 순서(Windows의 Claude 세션이 따르는 하네스)는 `docs/windows/WINDOWS_HARNESS.md`, 사람이 먼저 읽는 짧은 안내와 시작 프롬프트는 `docs/windows/WINDOWS_QUICKSTART.md`다.

오디오 인터페이스 인식 설계는 macOS·Windows 둘 다 하드코딩 없이 그 순간 시스템에 등록된 장치를 스캔하는 구조로 일관되다(`get_hosts()` — macOS는 CoreAudio만, Windows는 ASIO+WASAPI 둘 다 스캔). 이 부분은 설계 검토 완료, 실기 검증만 남음.

## 남은 일

1. **Windows CI 빌드 — 확인됨(2026-10-05)**: CI(`.github/workflows/build_release.yml`)가 이제 `main` 대상 PR마다 돈다(PR #8에서 추가). PR #8·#10의 Windows·macOS 빌드가 통과해 cpal 0.16(ASIO), 재시작 복원, 절전 방지(`Win32_System_Power`)가 Windows에서 컴파일되는 것을 확인했다. Windows CI는 감시 크레이트 `cargo test`, 앱 Rust `cargo check`, 앱 빌드만 하고 앱의 `cargo test`·`flutter test`는 돌리지 않는다(12번). 배포(릴리스 단계)는 여전히 태그(`v*`)에서만 돈다.
2. **Windows PC 실기 확인(사용자가 Windows PC 앞에서)**: 위 "⚠️ Windows 미검증" 목록 전부. 진행 방법은 `docs/windows/WINDOWS_QUICKSTART.md`. 특히 다음 세 가지다.
   - 앱 기동과 3D 방 뷰어: `webview_flutter`에 Windows 구현이 없어 동작하지 않을 가능성이 가장 크다. → 2026-10-07 앱 기동 확인, 3D는 WebView2로 구현(남은 일 10).
   - ASIO·Dante 장치 스캔과 12ch 출력.
   - 장치를 뽑았다 꽂는 재시작.
3. **충돌 후 자동 재실행(Windows) — 코드 완료, macOS 소리 시나리오 확인 완료(2026-10-07)·Windows 미검증**: 설계 `docs/superpowers/specs/2026-10-05-crash-relaunch-supervisor-design.md`, 구현 계획 `docs/superpowers/plans/2026-10-05-crash-relaunch-supervisor.md`(12개 작업, "구현 상태" 표가 기준). 별도 Rust 감시 프로그램(`atmos_supervisor`)이 앱의 신호 파일(`app.lock`, `app.pid`, `heartbeat`, `clean_exit`)로 충돌·멈춤을 판단해 앱을 다시 띄우고 공연을 멈춘 위치부터 이어 간다. 운영자가 일부러 닫으면 다시 띄우지 않는다. 로그인 자동 실행은 환경설정 "로그인할 때 공연 자동 시작"(기본 켜짐)으로 정하고, CI는 PR마다 Windows 빌드를 돌리게 한다. 진행: 계획의 Task 1~11 코드가 끝났다(구현 세션 보고: 감시 크레이트 테스트 21개, 앱 Rust `cargo test` 78개 바이너리, `flutter test` 158개 통과, clippy·analyze 새 경고 없음, Windows 대상 감시 크레이트 clippy 통과. macOS 릴리스 앱의 소리 없는 실제 확인 12개와 `integration_test/app_flow_test.dart` 0~6단계도 통과했고, 이 확인에서 `--app` 상대 경로 버그를 찾아 고쳤다). macOS 소리 시나리오는 2026-10-07 11:36~11:38에 확인했다(Main 실행, 사용자가 옆에서 조작·청취). `tool/supervisor_e2e_macos.sh`, 릴리스 앱 2026-10-07 06:16 빌드(main dbea486 이후 코드와 같음), 실제 config, Scarlett 6i6, 결과 "통과 14, 실패 0": 0단계 감시가 앱을 `--supervised`로 띄우고 하트비트 수신. 1단계 `kill -9` — 감시 기록 "앱이 닫는다는 표시 없이 끝났다(충돌)" → 2.0초 뒤 `--supervised --auto-relaunched`로 재실행 → 앱 로그 "공연 이어 가기: 활성 방 None, 트랙 1개를 멈춘 위치부터 다시 튼다"(재실행 5초 뒤, 큰 파일 미리 불러오기 포함), 사용자가 귀로 "멈춘 곳 근처부터 났음" 확인. 2단계 `kill -STOP` — "UI 멈춤: 하트비트가 10초 넘게 바뀌지 않았다 — 강제 종료" → 4.0초 뒤 재실행(백오프 두 번째). 3단계 두 번째 감시는 "감시가 이미 돌고 있다 — 기존 창을 앞으로 가져오고 끝낸다"로 끝나고 앱은 하나. 4단계 앱을 직접 두 번째로 띄우면 시작 관문 Duplicate, 종료 코드 3. 5단계 감시만 끝내도 앱은 돌고 새 감시가 "떠 있는 앱을 넘겨받았다". 6단계 창 닫기 → "운영자가 앱을 닫았다 — 감시를 끝낸다", 다시 뜨지 않음. 시험 중 사용자가 CH1 스피커의 서브우퍼 설정을 직접 껐다(의도한 변경, 그대로 둠). config.json·단축키는 시험 전후 같다. 남은 것: Windows 실행 확인(Windows 빌드는 PR #8부터 CI를 통과했다)은 현장 점검표. Windows 점검은 `docs/WINDOWS_FIELD_CHECKLIST.md`. installer 방화벽 규칙·WebView2 런타임 설치는 2026-10-08에 정했다(확정된 결정 "Windows 설치 파일").
4. **트랙·도면 경로 이식성 — 구현·병합됨(PR #10), Windows 확인 대기**: 프로젝트를 열 때 파일을 못 찾으면 `.atmos` 폴더에서 같은 이름으로 다시 연결한다(동작과 파일은 위 "최근 끝난 일" 첫 행). 오디오와 도면을 `.atmos`와 같은 폴더에 담아 옮기는 방식이 사용자 결정이다. 남은 것: ① Windows PC에서 실제로 여는 확인(`docs/WINDOWS_FIELD_CHECKLIST.md` 9절). ② 대화상자의 "폴더 고르기"를 사람이 실제로 눌러 보는 확인(화면 조작 도구가 없어 자동 확인이 안 된다). 저장 방식은 바꾸지 않았다(다시 연결한 경로는 이 PC의 절대 경로로 `config.json`과 도면 설정에 저장된다).
5. **현재 config 정리(사용자 확인)**
   - OSC 주소가 겹친다. 모든 트랙이 `/play`·`/stop`, 모든 방이 `/room/clear`이고, 테마 시작·시스템 리셋 주소는 비어 있다. 앱은 같은 주소면 마지막 것만 기억한다. 그래서 `/room/clear`는 빈 방 '5전시'를 비우고, `/play`는 방 2 마지막 트랙만 튼다. OSC로 방·트랙을 제어할 수 없는 상태다. 앱에 중복 주소 경고가 있으면 좋겠다.
   - 테마 1 BGM이 CH1로 나가는데 스피커 배치에서 CH1이 서브다. 그래서 80Hz 아래만 나간다(헤드폰으로 거의 안 들림, −45~−49dBFS). 메인(CH2)으로 보내려던 것인지 확인 필요.
   - 10분짜리 4채널 24bit WAV(테마 1 세 번째 트랙)를 통째로 RAM에 올린다(약 470MB). 긴 단발 트랙은 스트리밍으로 두는 편이 낫다.
6. **메모리 증가 원인 가리기 — 측정 완료: Rust 엔진은 늘지 않고, 릴리스 앱은 재생 중 분당 0.09~0.3MB**: 처음 측정(65분 동안 RSS 912→1059MB, 분당 2.2MB, 12시간이면 약 1.6GB)은 디버그 빌드에 테스트 하네스가 같이 돈 것이었다. 2026-10-06에 나눠서 다시 쟀다(macOS Intel, Scarlett 6i6 12ch 48kHz).
   - 엔진만 20분 — 48kHz 스트리밍 루프 2개 + 미리 불러온 단발 10초마다: 힙 +0.010MB/분, RSS 평평.
   - 엔진만 25분 — 실제 테마 BGM과 같은 44.1kHz 스테레오 11.6초 루프(재생 중 리샘플) + 48kHz 루프 + 단발, `api_theme_start`로 시작: 힙 +0.008MB/분, phys_footprint 87→87MB.
   - 릴리스 앱 20분, 재생 없음(실제 config): RSS +0.016MB/분(Dart GC로 416~431MB 톱니), 웹뷰 48MB 그대로.
   - 릴리스 앱 25분, 무음 테마 루프(실제 config 복사본에서 방 1만, BGM을 같은 형식의 무음으로, OSC 테마 시작): RSS +0.36MB/분, phys_footprint 145→148MB(+0.29MB/분), 웹뷰 20MB 그대로.
   - 릴리스 앱 30분, 무음 테마 루프, 번들 ID 복사본으로 환경설정까지 분리(2026-10-06 23:30~00:00): phys_footprint 145→146MB(처음 6분) → 7분째 그래픽 메모리가 풀려(graphics 32→20MB, IOAccelerator 5→2MB) 128MB → 30분째 130MB(+0.09MB/분). 앱 RSS는 같은 구간 +0.07MB/분. 종류별로 꾸준히 는 것은 MALLOC_SMALL 26.0→27.1MB(+0.04MB/분)뿐이고, MALLOC_LARGE 35MB, IOKit 16MB, VM_ALLOCATE(Dart 힙으로 보임) 13~16MB는 그대로였다. 두 릴리스 재생 측정 모두 화면은 켜져 있었다(사용자의 caffeinate가 화면 꺼짐을 막았다). 7분째 그래픽 메모리가 풀린 이유는 확인하지 못했다.
   - 결론: Rust 엔진은 늘지 않는다. 릴리스 앱은 재생 중 분당 0.09~0.3MB(1차 +0.29, 2차 +0.09) 늘고, Dart 힙과 큰 할당은 그대로이며 MALLOC_SMALL만 조금씩 는다. 12시간이면 약 65~200MB다. 처음의 2.2MB/분은 디버그 빌드·테스트 하네스 영향이었다.
   - 측정은 여기서 마친다(사용자 지시로 시간 제한). 최종 확인은 현장 Windows PC의 Release 12시간 연속 재생(`docs/WINDOWS_FIELD_CHECKLIST.md` 4절)이다. 12시간에 수백 MB 넘게 늘면 그때 Flutter DevTools(프로필 모드)로 화면 그리기 쪽을 본다.
   - 엔진 진단 실행: `rust/`에서 `DIAG_MINUTES=20 cargo test --test diag_playback_memory -- --ignored --nocapture`(`rust/tests/diag_playback_memory.rs`, macOS 전용, 기본 출력 장치를 무음 트랙으로 연다).
7. **실제 장치를 뽑았다 꽂는 재시작(macOS) — ✅ 실기 확인·수정 완료(2026-10-07)**: Scarlett 6i6 12ch, 번들 ID 복사본과 시험 설정, BGM 자리에 −30dBFS 1kHz 시험음(CH2), 케이블은 사용자가 조작했다. 사용자 설정 파일은 시험 전후 해시가 같다.
   - 수정 전: 짧게 뽑기 2회(16초·11초)는 워치독이 1.08초 만에 감지하고 30초 안에 Scarlett을 다시 찾아 원래 크기로 멈춘 자리부터 이어 갔다 ✅. 30초 넘게 뽑으면 30초 찾다가 맥북 스피커로 비상 전환(−40dB)하고 재생은 이어 갔다 ✅. 다시 꽂으면 macOS가 기본 출력을 Scarlett으로 돌려 12ch로는 열렸지만 장치 이름 없이 열려 −40dB가 남았다 — 사용자 귀로 "거의 안 들림" ❌.
   - 수정(커밋 695cdfd): ① 설정의 장치를 기억한다. 공개 시작 API `api_init_audio_system`(Dart의 설정 로드·저장·환경설정·강제 재시작이 모두 이 길)에서만 저장하고, 비상 전환의 내부 재기동(장치 이름 없음)은 바꾸지 않는다. ② 엔진이 스스로 다시 시작할 때(워치독·장치 유실·장치 목록 변화) 비상 상태이고 설정의 장치가 장치 목록에 다시 있으면 그 이름을 지정해 연다 → 비상 −40dB가 풀린다. 목록에 없으면 기본 장치에 머문다(없는 장치를 지정하면 30초 찾느라 무음). 규칙은 `core::device_return::device_for_self_restart`(순수 함수). ③ 장치 유실을 cpal 오류 종류(`StreamError::DeviceNotAvailable`)로 판정한다. 표시 문구("The requested device is no longer available…")에 "DeviceNotAvailable"이 없어 문자열 검사로는 못 잡았고, 1초 뒤 워치독이 대신 잡고 있었다. ④ 앱 로그에 "비상 전환: …(출력 −40dB)", "비상 전환 해제 시도: 설정의 장치 '…'가 돌아와…", "비상 전환 해제: 지정한 장치로 다시 열었다(출력 감쇠 해제)"를 남긴다(전에는 println뿐이라 현장 로그에 없었다).
   - 수정 후: 뽑는 순간 오류 종류로 바로 감지(워치독 없이) → 30초 찾다가 비상 전환(앱 로그에 기록) → 다시 꽂고 약 2초 뒤 설정의 장치를 이름으로 다시 열어 비상 해제, BGM 이어 감 — 사용자 "원래 크기로 돌아왔다" ✅.
   - 검증(Main 세션): 새 단위 테스트 6개(`test_failover_return_device`) 통과. `cargo test` 전체 실행 파일 81개, 228 통과·0 실패·9 무시(수동 진단). clippy lib 경고 26건 그대로(바꾼 코드에는 없음). Dart는 바꾸지 않았다.
   - 사용자 결정(2026-10-07): 비상 전환의 −40dB 안전 패드는 그대로 둔다(맥북 볼륨 최대에서도 비상 중에는 들리지 않았다 — 설계대로. 기본 장치는 2채널이라 CH3 이후는 어차피 빠진다). 비상 상태는 "공연 시간을 이어 가며 장치를 기다리는 상태"로 둔다. 장치를 찾는 30초 동안은 무음이다(설계, 바꾸지 않음).
   - Windows 확인은 위 "⚠️ Windows 미검증" 15번.
8. **사용자 결정이 필요한 발견**(통합 테스트를 만들며 발견, 고치지 않음):
   - 대시보드 Start는 모든 방의 루프를 틀고, OSC 테마 시작은 첫 방의 루프만 튼다. 의도된 차이인지 확인 필요.
   - ✅ 해결(2026-10-09, 사용자 결정): 기본 공간 리버브(Hall 3.2초, 80%)가 하드웨어 CH1/CH2 마스터 버스에 걸리던 것(서브 채널에도, 정지 뒤 약 4초 꼬리, CH2 소리가 CH1으로 샘) — 마스터 리버브를 걸지 않고 기본 MIX를 0%로 했다(확정된 결정, 최근 끝난 일).
   - 1024×768 기본 창에서 인스펙터의 LFE 스위치가 아래 오버레이에 가려 탭되지 않는다(탭 위치 y=678 경고). 테스트는 스위치의 `onChanged`를 직접 부른다.
   - FX 자동 동기화(`acousticSyncProvider`)를 `main.dart`와 `dashboard_screen.dart` 두 곳에서 watch한다. 대시보드가 스피커 화면 아래 스택에 남아 있어 둘 다 지워야 3단계가 실패한다(대시보드 쪽은 중복).
   - 재시작 복원된 소리는 새 엔진의 부팅 뮤트 램프(3초, `mixer.rs` `StartupMuteRamp`)를 타고 커진다. 재개 순간 약 −15dB에서 시작해 3초 뒤 원래 크기다. 자기 재시작에서 이 램프를 줄일지(재개에는 이미 300ms 페이드 인이 있다) 결정이 필요하다.
   - (기존) 오디오 스레드 3법칙 위반: 재생 명령의 Box 해제(`engine.rs` PlayTrack), `target_bands.clone()`, `ApplyAllChannelTunings`의 해제, 큰 블록에서 `temp_buf.resize`, `ATMOS_TRACE_CMD`의 `eprintln!`, 음소거·페일오버·헤드룸·출력 채널·위상의 즉시 전환.
   - ✅ 해결(2026-10-07, 커밋 496e2f5): 풀·정리 채널이 가득 찰 때 인스턴스를 오디오 스레드에서 버리던 것(스트리밍이면 디코더 스레드 join) — 최근 끝난 일 참고.
   - ✅ 해결(2026-10-07, 커밋 0a7a840): 스트리밍 단발이 파일이 끝나도 재생 중으로 남던 문제 — 최근 끝난 일 참고.
   - 긴 파일을 먼 위치에서 재개하면 시작 위치까지 디코딩해 버리는 시간만큼(리샘플이 필요한 파일은 수 초) 늦게 나온다. `DiskStreamer::new_at`에 symphonia seek를 쓰면 줄일 수 있다.
   - 자기 재시작 때 Dart 재동기화가 두 번 돈다(`EngineReady`와 `EngineRestarted` 둘 다).
9. **Windows 채널 이름(ASIO)**: 사용자가 나중에 하기로 보류. `rust/src/audio/channel_names.rs`의 실행 계획(A안 asio-sys 패치, B안 직접 FFI)은 cpal 0.16.0에서도 유효하다(asio-sys 0.2.6 그대로).
10. **3D 방 뷰어 Windows 구현(필수, 사용자 결정 2026-10-05) — 구현·Windows 실기 확인(2026-10-07, PR #30), macOS 확인 완료(2026-10-08)**: Windows만 `webview_windows` 0.4.0(WebView2)으로 같은 HTML을 띄운다(`three_js_engine_provider.dart`의 `Platform.isWindows` 분기, `dynamic_3d_room.dart`). macOS는 `webview_flutter` 그대로다. HTML은 바꾸지 않고, 문서 시작 시점에 `window.SpeakerBridge` 호환 객체를 넣어 JS→Dart 메시지를 WebView2 웹 메시지로 받는다. WebView2는 화면 밖에서도 매 프레임 그려서(수정 전 대시보드에서도 144fps, GPU 약 14%) 3D 화면을 떠나면 멈추고(`suspend`) 다시 열면 재개한다(`resume`) — macOS 웹뷰가 창에서 빠지면 멈추는 것과 맞춘 것이다. 런타임이 없으면 3D 자리에 안내 문구를 보이고 앱·하트비트는 정상이다. 측정(Windows 10 19045, WebView2 150.0.4078.48): 3D 준비 1.4~2.1초, 144fps(모니터 상한), 클릭 3~7ms·드래그 평균 1~12ms(최대 85ms), 시점 전환·귀 높이 정상, JS 오류 0. Release 10분 재생 중 3D 조작: 엔진 재기동·비상 전환 0, 하트비트 `silent_warnings=0`(TotalMix 미터로 신호 확인, 스피커로는 듣지 못함). 3D 화면 밖 WebView2 CPU 0%·GPU 약 5%, 3D 화면 WebView2 CPU 2~6%·GPU 15~19%(12코어). `app_flow_test -d windows` 0~6단계 통과. 빌드 때 CMake가 nuget.exe와 WebView2 SDK를 내려받는다(인터넷 필요). 플러그인이 `/await`와 `<experimental/coroutine>`을 써서 CI 이미지의 새 MSVC(14.51, Visual Studio 18)에서는 STL1011 오류가 난다 — `windows/CMakeLists.txt`에서 `_SILENCE_EXPERIMENTAL_COROUTINE_DEPRECATION_WARNINGS`를 정의해 막았다. MSVC가 이 헤더를 곧 없앤다고 예고했으므로, 그때는 플러그인을 C++20 코루틴으로 고친 판(포크·새 버전)이나 다른 패키지(`flutter_inappwebview`)가 필요하다. WebView2 사용자 데이터는 `%LOCALAPPDATA%\flutter_webview_windows\atmos_mixer_pro`에 생긴다. macOS 확인(2026-10-08, Mac 세션 Main + 사용자): main 40af57b에서 `app_flow_test -d macos` 0~6단계 통과, 릴리스 빌드(`flutter build macos --release`)를 번들 ID 복사본으로 띄워 스피커 배치 화면의 3D가 잘 보이고 스피커 클릭(인스펙터)·드래그·시점 전환이 모두 됨을 사용자가 눈으로 확인했다(결과는 #30 댓글). 남은 것: 스피커로 듣는 확인(3D 조작 중 끊김). WebView2 런타임 배포는 2026-10-08에 정했다(확정된 결정 "Windows 설치 파일": Evergreen Standalone을 설치 파일에 넣고 없을 때만 설치).
11. **v1.1.3 태그와 main의 버전 차이(사용자 확인)**: `v1.1.3`은 `main`의 조상이 아니다(8/26에 갈라졌고 `main`에 없는 커밋이 32개: `c2bd1d0` println!→log_print! 교체, `5033735` 닫기 버튼·Cmd+Q, `7411d31` 캘리브레이션 자동 계산, `5ef4c92` 리버브 API 연결 등). `main`의 앱 버전은 1.1.2로 v1.1.3보다 낮다. 의도한 것인지, 현장에 설치된 버전이 무엇인지 확인한다. `main` 계통에서 Windows CI가 마지막으로 성공한 시점은 `v1.1.2`다.
12. **Windows `cargo test` 가드 — ✅ 완료(2026-10-06, 커밋 3273b83)**: `rust/tests/test_analysis_thread_idle_cpu.rs`에 `#![cfg(unix)]`를 붙였다(유닉스 `getrusage`로 CPU 시간을 잰다). `diag_engine_restart_leak.rs`에는 이미 `#![cfg(target_os = "macos")]`가 있었다(ccf776e, 10-04). 새 수동 진단 `diag_playback_memory.rs`도 macOS 전용이다. Windows에서 `cargo test`가 실제로 컴파일되는지는 아직 확인하지 못했다(Windows CI는 빌드만 한다). Windows CI에 `cargo test --no-run` 단계를 넣을지는 사용자 결정이다.
13. **Windows 로더 — ✅ 완료(2026-10-06, 커밋 d926a30)**: `lib/core/utils/rust_library.dart`의 `bundledRustLibrary()`. macOS는 기존대로 앱 번들 프레임워크를, Windows는 실행 파일(`Platform.resolvedExecutable`) 옆의 `rust_lib_atmos_mixer_pro.dll`을 직접 연다(없으면 기본 로더). 개발 PC에서 프로젝트 폴더로 `flutter run -d windows`해도 `rust/target/release/`의 옛 dll이 실리지 않는다. 경로 계산은 `test/rust_library_path_test.dart`가 확인한다. Windows에서 실제로 실리는 dll 확인은 현장 점검표 10절.

## 운용 메모

- 스테레오·멀티 트랙은 서브우퍼가 아닌 채널부터 라우팅한다. 서브로 간 채널은 크로스오버 아래만 남는다.
- 추적 로그: `ATMOS_TRACE_CMD=1`로 앱을 실행하면 재생/정지 요청, 인스턴스 정리, 엔진 시작/종료(세대), 베이스 라우팅이 stderr에 찍힌다.
- 수동 진단(자동 테스트 아님): `rust/tests/diag_theme1_track4.rs`, `rust/tests/diag_theme1_multitrack.rs`, `rust/tests/diag_engine_restart_leak.rs`, `rust/tests/diag_playback_memory.rs`(재생 중 엔진 메모리) — 파일 머리 주석에 실행법.
- 디스크 여유가 적으면 전체 `cargo test`가 ENOSPC로 실패한다(테스트 바이너리마다 최적화+디버그 정보라 수 GB 필요).
- **프로젝트 폴더에서 `flutter run -d macos`로 띄운 앱이 옛 Rust를 싣던 문제는 고쳤다(c7d2fd2).** FRB 기본 로더는 작업 디렉터리 기준 `rust/target/release/librust_lib_atmos_mixer_pro.dylib`을 앱 번들보다 먼저 연다. 그래서 `cargo build --release`로 남은 옛 빌드(2026-09-28)가 실렸다 — API가 같으면 조용히, 다르면 "Content hash … different" 오류로. 이제 `lib/core/utils/rust_library.dart`가 macOS는 번들 프레임워크를, Windows는 실행 파일 옆의 dll을 직접 연다(남은 일 13). 2026-10-03 이전에 `flutter run`으로 확인한 동작은 옛 Rust였을 수 있다.
- 엔진을 (재)기동하면 출력은 부팅 뮤트 램프(3초)로 0에서 커진다. 재시작 직후 레벨을 잴 때는 3초 뒤에 잰다.
- **Windows에서 RME 버퍼 크기를 바꾼 뒤 일반 소리가 늘어질 때(2026-10-08 실기)**: 녹음하려고 RME 설정 창에서 버퍼를 64↔1024로 여러 번 바꾼 뒤, 앱을 꺼도 유튜브 등 Windows 일반 소리(WDM)가 "버퍼가 안 맞는 것처럼" 늘어졌다. 에이블톤(ASIO)은 정상이었고 RME 설정(48kHz·Internal·1024, USB 오류 0)과 Windows 장치 형식(48kHz)도 맞았다. Windows Audio 서비스(AudioSrv)를 다시 시작하자 돌아왔다(관리자 권한, 사람이 한다). 앱 소리 문제와는 별개다.
- 앱 로그의 "오디오 처리 부하" 줄(10초 구간): 콜백 수, 평균·최대 처리 시간, 예산(버퍼 길이), 예산 70% 넘은 콜백 수, 예산 초과 수, 늦은 콜백 수(앞 콜백과의 간격이 장치를 연 버퍼 길이의 1.5배를 넘음 — 장치가 그 사이 이전 버퍼를 다시 냈을 수 있다)와 간격 최대, 처리 채널 수. 정상이면 10분에 한 번, 세 가지 중 하나라도 있으면 1분에 한 번 "높음"으로 남는다. 처리 시간은 명령 처리부터 장치 버퍼 채우기까지다.
- 통합 테스트는 기계가 바쁘면(예: Claude 앱 화면이 CPU를 크게 쓸 때) VU 전달이 들쭉날쭉해진다. 5단계는 재개 대기 구간(약 100ms) 안의 표본 수가 아니라 앞뒤 0.5초 구간으로 VU 스트림이 살아 있는지 본다.
- **측정용으로 macOS 앱을 다른 홈에서 띄울 때**: `CFFIXED_USER_HOME`로 앱을 띄우면 `config.json`·상태 파일은 그 홈으로 가지만, 환경설정(SharedPreferences, NSUserDefaults)은 실제 `~/Library/Preferences/com.example.atmosMixerPro.plist`에 쓴다. 2026-10-06 측정 때 이 때문에 실제 환경설정의 `flutter.tuning_state`가 바뀌어 측정 전 백업으로 되돌렸다. 또 `defaults import com.example.atmosMixerPro`는 컨테이너(`~/Library/Containers/com.example.atmosMixerPro`)가 있으면 그쪽 plist에 쓰므로, 실제 파일을 고칠 때는 도메인 대신 plist 전체 경로를 준다. 환경설정까지 분리하는 방법은 2차 측정(2026-10-06)에서 확인했다: 릴리스 앱을 `ditto`로 복사 → `plutil`로 `CFBundleIdentifier`를 `com.example.atmosMixerPro.memdiag`로 바꿈 → `codesign --force --deep --sign -`(원본도 adhoc 서명) → `CFFIXED_USER_HOME`(파일)과 `TMPDIR`(로그·신호 파일)을 함께 주고 실행. 끝나면 `defaults delete com.example.atmosMixerPro.memdiag`로 시험 도메인을 지운다.
