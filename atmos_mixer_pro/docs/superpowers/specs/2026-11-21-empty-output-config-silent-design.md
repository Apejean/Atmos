# 출력 설정에서 켠 그룹이 없으면 소리 없음(짧은 설계)

> 상태: **사용자 결정(2026-11-21 "아무것도 안 켜면 소리 없음"), Windows Main 작성.** 경로는 `atmos_mixer_pro/` 기준. 1.3.15 범위(브랜치 `win/width-zero-based`, PR #134).

## 왜
1.3.14 설치본 확인(2026-11-21)에서 사용자가 출력 설정을 아무것도 켜지 않은 공연인데 소리가 나는 것을 보고, "아무것도 선택이 안 되어서 재생이 안 되어야 하는데 채널이 잡혀서 소리가 난다"고 지적했다. 지금 규칙은 2026-09-13(e5056f4, `docs/02_Planning_and_Specs/OUTPUT_CHANNEL_MAPPING_UNIFICATION_SPEC.md` 4절)부터 **"켠 그룹이 하나도 없으면 모든 채널 열림"**이다. 그때 문제는 화면과 엔진이 서로 다른 기준을 써서 "선택은 되는데 소리가 안 난다"였고, 둘 다 "전체 열림"으로 맞췄다. 사용자 결정으로 기록된 적은 없다. 이번에 사용자가 반대로 정했다: **켠 그룹이 없으면 엔진은 어느 채널로도 내보내지 않고, 화면도 고를 채널을 내놓지 않는다.** 화면과 엔진은 여전히 같은 기준을 쓴다(2026-09-13의 어긋남은 다시 생기지 않는다).

## 바꾸는 것
| 곳 | 지금 | 바뀐 뒤 |
|---|---|---|
| 엔진 채널 게이트 `api/simple.rs` `compute_enabled_channels` | 켠 그룹 없음 → 모든 채널 열림 | 켠 그룹 없음 → 모든 채널 닫힘(무음) |
| 엔진 처리 폭 `core/processing_channels.rs` `processing_channel_count` | 켠 그룹 없음 또는 켠 채널이 모두 장치 밖 → 장치 전체 | → 0(믹서 폭은 최소 16 그대로, 로그 "94채널 중 0채널 처리") |
| 화면 트랙 채널 목록 `lib/core/utils/channel_routing.dart` `buildChannelRoutingItems` | 켠 그룹 없음 → 모든 장치 채널로 자동 생성 | → 빈 목록. 트랙 카드는 이미 있는 안내 "Output Config에서 사용할 채널 그룹을 열어주세요"를 보인다 |
| 엔진 앱 로그 | 없음 | 설정을 적용할 때 켠 그룹이 없어지면 한 번 "출력 설정에서 켠 채널 그룹이 없어 어느 채널로도 내보내지 않는다" |

- 멀티 그룹만 켜졌을 때 "장치 끝까지 처리" 규칙은 그대로 둔다(파일 채널 수를 몰라서).
- 켠 그룹이 있는 공연(현장 설정 CH1~12 등)은 동작이 바뀌지 않는다.

## 덜어내는 것(어제 #131의 처리 폭 계산)
켠 그룹이 없으면 처리할 채널이 없으므로, 그때 트랙·서브우퍼 기준으로 처리 폭을 정하던 계산이 필요 없다(2026-11-20 설계 `2026-11-20-processing-width-fallback-design.md`는 이 문서로 대체).
- 뺌: `used_channel_ceiling`, `effective_processing_channels`, `current_processing_channels`, 파일 머리말 캐시 `cached_file_channels`, `audio/streaming.rs` `file_channels`, `GLOBAL_STATE.highest_sub_channel`과 스피커 배치에서 그 값을 바꾸던 부분, 시험 `tests/test_processing_width_fallback.rs`.
- 그 자리는 `processing_channel_count` 하나로 돌아간다(엔진 시작·설정 변경).
- 오늘 고친 0부터 세는 계산은 이 계산과 함께 빠진다.

## 남기는 것(오늘 고친 것)
- 출력 설정 저장 한 번에 엔진이 두 번 다시 시작하던 것 막기: `GLOBAL_STATE.processing_width_restart`, `processing_channels::width_change`·`take_width_restart_request`, 정지 이유 `processing_width_changed`. `width_change`는 `processing_channel_count`로 원하는 폭을 정한다. 시험 `tests/test_width_restart_request.rs`는 켠 그룹이 있는 설정으로 바꾼다.
- Rust `TrackConfig.output_channel` 주석 정정(0부터 센 장치 채널).

## 영향
- 새로 설치한 PC, 출력 설정이 빈 공연, 예전에 출력 설정 없이 만든 공연은 출력 설정에서 채널 그룹을 켜기 전까지 무음이다. 트랙 카드마다 안내가 보이고, 재생 중이면 기존 무음 경고(1분)도 뜬다.
- 바이노럴 미리듣기·서브우퍼 저역도 출력 채널을 거치므로 같이 무음이다.
- 현장 공연(그룹을 켠 공연)은 영향 없음.

## 시험
- Rust: `test_enabled_channels_multi_config.rs`의 "비었으면 전체 열림" 두 시험을 "비었으면 전체 닫힘"으로 바꾼다. `test_processing_channels.rs` "출력 설정을 건드리지 않았으면 장치 전체" → 0, 켠 채널이 모두 장치 밖이면 0 추가. `test_width_restart_request.rs`는 그룹을 켠 설정으로.
- Dart: `channel_routing_test.dart` "Output Config가 비어있을 때" 묶음을 "빈 목록"으로 바꾸고, 그 묶음에 있던 멀티 파일 부분 출력 시험은 멀티 그룹을 켠 설정으로 옮긴다.
- 장치(이 PC): 출력 설정 빈 공연 → 로그 "0채널 처리", LONG 루프 무음·트랙 카드 안내. 스테레오 CH1~24를 켜면 → 24채널 처리·소리 남, 저장 한 번에 재시작 한 번.

## 하지 않는 것
- 새 배너(대시보드 전체 경고)는 만들지 않는다. 트랙 카드 안내·무음 경고로 충분한지 장치 확인 뒤 사용자가 정한다.
- 출력 설정 구조(`config.json`)는 바꾸지 않는다.
