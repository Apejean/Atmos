# 매트릭스 라우팅 구현 계획 (2026-11-24, Windows Main)

설계: `docs/superpowers/specs/2026-11-14-matrix-routing-design.md`(개정 3 — 11.1이 본문보다 우선). 브랜치 `win/matrix-routing`(PR #146 `win/wav-reader` cd6134f 위). 사용자 지시 2026-11-24 "매트릭스 라우팅 지금 시작해". 단계마다 커밋하고, 각 단계 끝의 확인이 통과해야 다음으로 간다. 커밋·PR은 PR 머지 작업 세션이 한다.

## 기준값 (구현 전, 이 PC)
- `cargo test --no-fail-fast`: 415개 통과·0개 실패·12개 무시(`D:\dev\logs\2026-11-24\W1124_X_base_cargo_120013.log`)
- `flutter analyze` / `flutter test`: 0건 / 306개 통과(`W1124_X_base_flutter_120500.log`)
- `cargo clippy --tests` 라이브러리 경고: 28
- 부하(설계 8.5의 12번 기준): 설치본 1.3.18 현장 공연(매핑, 루프 3개, 12채널 처리) 평균 3.1~3.3ms·최대 5ms 안팎, 경고 0(2026-11-24 앱 로그).

## 0단계 — A9 정리: 미룸 (2026-11-24 Main 판단, 사용자에게 알림)
- 조사: 채널별 공간 게인(`channel_spatial_gains*`, 방 볼륨·전역 궤적 DBAP·스피커 `pan_deg`)을 블록마다 계산해 `GLOBAL_STATE.spatial_gains`에 쓰지만 출력에는 쓰지 않는다. 읽는 곳은 FRB `api_get_spatial_gains`(Dart는 부르지 않음)와 시험 두 개(`test_mono_file_own_channel`가 값을 넣고, `test_multi_room_field_scenario`의 궤적 시험이 목표값을 확인)뿐이다.
- 지우면 스피커 `pan_deg`·OSC `/audience/pos`(궤적)가 하는 일이 완전히 없어지고 FRB 함수 하나가 빠진다(지금도 소리에는 영향 없음). 기능 표면이 바뀌므로 매트릭스와 분리해 사용자 결정 뒤 따로 한다. 매트릭스는 이 계산과 상관없이 성립한다(설계 9절).
- 매트릭스 표 경로는 이 계산을 쓰지 않는다.

## 1단계 — 설정 구조 (설계 5절, 10절 1)
- `common/config.rs`: `TrackRoute`, `TrackConfig.routes`(`skip_serializing_if = Vec::is_empty`), `OutputRoutingMode`, `AppConfig.output_routing_mode`(기본 생략), `output_channel` 주석 "0부터".
- FRB 다시 생성(`flutter_rust_bridge_codegen generate`, 2.12.0). Rust 시험의 `TrackConfig { … }`에 `routes: vec![]`, Dart `TrackConfig(` 19곳에 `routes:`, `config_copy.dart`·`config_copy_test.dart`.
- 시험(먼저): `tests/test_track_routes.rs`의 설정 왕복(옛 키 없음 → 빈 표·매핑, 빈 표·기본 방식은 키 없음, 방식 왕복에도 `routes`와 매핑 값 그대로).
- 확인: 전체 `cargo test`·`flutter analyze`·`flutter test`가 기준과 같다(동작 변화 없음) + 새 시험.

## 2단계 — 순수 함수 `audio/route.rs` (6.1)
- `RouteEntry`, `ResolvedRoutes`, `resolve_routes`(상한 `MAX_TRACK_ROUTES` 512, 파일 채널 상한 `wav_reader::MAX_FILE_CHANNELS`).
- 시험(먼저): `test_track_routes.rs` 나머지.

## 3단계 — 인스턴스와 변경 규칙 (6.2·6.3)
- `SoundInstance`: `table_active`·`prev_table_active`·`routes`·`prev_routes`·`pending_routes`(용량 512)·`pending_path`, `set_initial_routes`·`request_table_path`·`request_legacy_path`·`promote_pending`. `new` 인자 그대로.
- `engine.rs` `SetTrackOutput` → `request_legacy_path`.
- 시험(먼저): `SetTrackOutput` 한 콜백 두 번 딸깍 없음, `test_master_switch_ramps` 그대로.

## 4단계 — 믹서 표 경로·명령 (6.4·6.5, 11.1 ①③⑤)
- 블록 시작: `promote_pending`, 표 경로면 `route_open` 목표 1(③).
- 샘플 루프: 두 경로(현재·직전) 구조 유지, 표면 `for e in table { if s < read_limit && o < gate_len && gate>0 { output += vals[s]*route_vol*e.gain*gate } }`.
- `temp_vals` = `width.max(MAX_FILE_CHANNELS)`, `legacy_limit`/`read_limit`(6.5).
- `AudioCommand::SetTrackRoutes`, `SpatialGarbage::Routes`, `route_garbage`(용량 64)·forget 카운터와 점검 루프 로그.
- 시험(먼저): `test_matrix_routing.rs`, `test_matrix_route_change_no_click.rs`, `test_audio_thread_no_heap` 추가분.

## 5단계 — 재생 시작·API·재동기화 (6.6, 11.1 ②⑥⑦)
- `with_routes`(네 곳), `api_set_track_routes`(FRB 하나), 방식에 맞지 않는 호출 거절, 설정 교체 재동기화 함수(다섯 곳), 버린 칸 로그(`config_version`마다 한 번).
- 시험(먼저): `test_play_track_routes.rs`(네 경로, 실제 파일 채널 수, 방식별, 순서, 소스 감사, 재동기화), `test_processing_channels`에 "표가 있어도 폭은 출력 설정만으로".
- 확인: 전체 `cargo test`, clippy 새 경고 0.

## 6단계 — Dart (7절, 11.1 ④⑪⑫)
- `core/utils/track_routes.dart`(7.1, 7.3은 개정 3 표), 표 창 `track_routing_dialog.dart`(7.2, 확인 창 없음), 트랙 카드·방 카드·환경설정(7.0·7.4), `project_media.dart`, 상태 영역 "출력: 매트릭스".
- 시험(먼저): 8.3.
- 확인: `flutter analyze` 0, 전체 `flutter test`.

## 7단계 — 측정·실행·문서
- 8.4 부하 벤치 `bin/route_bench.rs`((가)는 구현 전 커밋에서도 잰다), 인스턴스 메모리.
- Release 빌드 → 8.5(개정 3: 10번은 "다시 시작하지 않음") — 사용자에게 장치·음량·앱 끄는 시간을 먼저 묻는다.
- 8.7 결과표(PASS/FAIL/NOT VERIFIED), HANDOFF(확정된 결정·남은 일 16), 현장 점검표, 운영 안내서(표 쓰는 법·롤백 한 줄), 설계 상태 줄, Mac 확인 요청(8.6).

## 멈추는 조건
- 6.9 표 밖의 신호 흐름을 바꿔야 할 때 → 사용자에게 범위·영향을 먼저 설명.
- A9 공간 게인을 화면·OSC가 쓰고 있을 때 → 보고.
- 시험 기준값과 다른 실패가 생겼는데 원인을 모를 때 → 멈추고 보고.

## 진행 (2026-11-24 12:3x, 재개 지점)
- 1~4단계 구현·시험 통과(커밋 전, 작업 폴더 D:/dev/worktrees/matrix-routing). 1~3단계 합친 전체: cargo 426/0/12, clippy 28, flutter analyze 0·test 306. 4단계 새 시험: test_matrix_routing 10, test_audio_thread_no_heap +2, test_matrix_route_change_no_click 2 통과. 4단계 전체 확인(W1124_X4_*) 결과는 로그에서 확인할 것.
- 5단계: 수정 스크립트 D:/dev/logs/2026-11-24/staged/patch_step5.py, 시험 staged/test_play_track_routes.rs 준비됨 — 아직 적용 안 함. 순서: 시험 복사(RED 확인) → 스크립트 적용 → FRB 다시 생성(api_set_track_routes, 생성 파일은 실제 바뀐 것만, CRLF 맞춤) → 시험 → 전체 확인.
- 그다음 6단계(Dart), 7단계(측정·장치 확인·문서).
- 4단계 전체 확인 통과(12:2x): cargo 438/0/12(W1124_X4_cargo_122344.log), clippy 라이브러리 28, flutter analyze 0·test 306. 다음은 5단계부터.
- 5단계 끝(18:1x): 재생 시작 네 곳 `with_routes`, `api_set_track_routes`(FRB), `api_set_track_output`은 매트릭스·표 트랙이면 보내지 않음, 설정 교체 네 곳(`api_get_config`·`api_save_config`·`api_preload_all_sounds`·장면 불러오기) `resync_routing`, 버린 칸 로그(설정 버전마다 한 번). 시험 `test_play_track_routes` 5개(소스 감사 포함), `test_track_routes`에 처리 폭 시험. cargo 444/0/12.
- 6단계 끝(18:2x): `core/utils/track_routes.dart`(7.1 + `seedMissingRoutes`), 표 창 `track_routing_dialog.dart`(7.2, 확인 창 없음), 트랙 카드(매트릭스면 요약·표 단추, 매핑이면 지금 그대로)·방 카드(엔진 먼저, 확정 때 저장)·새 트랙 표 채우기, 환경설정 출력 방식 선택·`?`·채우기 안내·트랙 행 요약/표 단추, `highestProjectChannel`, 메인 머리줄 "출력: 매트릭스". Dart 시험: `track_routes_test` 10, `track_routing_dialog_test` 7, `track_card_controls_test` +3, `project_media_test` +1, `config_copy_test` 확장.
- 구현 메모: 트랙 카드 요약의 주황은 파일에 없는 채널·장치에 없는 출력만 본다(닫힌 출력은 엔진 규칙을 비동기로 물어야 해서 표 창에서 보인다).
- 남은 7단계: 8.4 부하 벤치·인스턴스 메모리, Release 빌드, 8.5 장치 확인(사용자가 PC 앞에 있을 때, 10번은 "다시 시작하지 않음"), 8.7 결과표, 현장 점검표·운영 안내서, Mac 확인 요청(8.6), FRB를 Mac에서 다시 생성해 차이 0 확인.
