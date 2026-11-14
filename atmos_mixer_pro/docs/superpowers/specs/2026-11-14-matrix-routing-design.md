# 트랙 연결 표(매트릭스 라우팅) 설계 (초안)

> 상태: **설계 확정(2026-11-14). 구현 전.** 검토 세션 작성, Main 검수 반영(2026-11-14), 사용자 승인(2026-11-14: 13절 U1~U5 "그대로 가", 완성 기준에 "? 같은 아이콘이 있고 기능 설명 정도 있으면 좋을 듯해").
> 기준 코드: origin/main `d60b8c5`(v1.3.12 + PR #98). 이 문서의 행 번호·함수 이름은 이 커밋 기준이다.
> **구현 시점: 안정성 검증 프로젝트 ③~⑨(HANDOFF 남은 일 16번)가 끝난 뒤**(사용자 결정, Main 전달 2026-11-13). 그 사이 코드가 바뀌므로 구현 세션은 11절 "구현 전 다시 확인할 것"부터 한다.
> 확인 수준: 이 문서의 "지금 동작"은 코드 읽기다. 실행·측정은 하지 않았다.
> 경로는 `atmos_mixer_pro/` 기준.

## 0. 구현 세션이 읽는 순서

1. 저장소 `CLAUDE.md`(3법칙, 코딩 규칙) → 2. `docs/HANDOFF.md` → 3. 이 문서 전체 → 4. 11절의 다시 확인 → 5. 10절 순서대로 구현.
이 문서가 정하지 않은 것은 12절 "구현자가 정하는 것"에 있다. 그 밖에 요구와 구조가 부딪히면 억지로 진행하지 않고 Main에 보고한다(저장소 CLAUDE.md 에스컬레이션). 이 작업은 `config.json` 구조와 FRB API 계약을 바꾸므로 그 자체가 에스컬레이션 대상이고, 이 문서의 승인이 그 승인이다.

## 1. 목적과 범위

### 1.1 왜
- 사용자 요구(2026-11-13): "모노 채널로 한 가지 소리가 여러 구역에서 나야 할 때. 예를 들어 룸 1에 있는 1번 트랙의 소리가 스피커 1번이랑 3번이랑 5번에서 동시에 나야 할 때." "나누기·합치기·음량까지 되는 것이어야 해."
- 지금은 같은 파일을 트랙 세 개로 복제해야 한다. 시작·정지·음량을 세 번 다뤄야 하고, 디스크가 밀리면 스피커끼리 어긋날 수 있다.

### 1.2 목표
트랙마다 "파일의 몇 번 채널 → 어느 출력 채널, 음량 얼마"의 **연결 표**를 둘 수 있게 한다. 표 하나로 다음 다섯 가지가 된다.

| 동작 | 예 |
|---|---|
| 나누기 | 모노 트랙 → CH1·CH3·CH5 |
| 음량 | CH1 100%, CH3 70%, CH5 50% |
| 건너뛰기 | 4채널 파일 → CH1·CH2·CH5·CH6 |
| 순서 바꾸기 | 4채널 파일의 1·2·3·4번 → CH3·CH1·CH4·CH2 |
| 합치기 | 스테레오 파일의 왼쪽+오른쪽 → CH2 |

- 재생 중에 표를 바꿔도 끊김·딸깍이 없다(현장에서 들으면서 맞춘다).
- 표가 없는 트랙은 지금과 **똑같이** 난다(옛 프로젝트 그대로).
- 트랙은 자기 방에 속한다. 방 볼륨·정지·방 클리어·OSC 게이트는 표의 모든 출력에 같이 걸린다(사용자 결정 2026-11-13 "같이 움직이는 걸로").

### 1.3 하지 않는 것(비목표)
- **현장 채널 한꺼번에 옮기기**(설계 출력 번호 → 장치 채널 패치 표). 2단계 후보로 남긴다(Main 제안). 이 문서의 표는 장치 출력 채널 번호를 직접 가리킨다.
- 27채널 이상·RF64 파일 열기(안정성 ⑨). 이 문서는 열린 파일의 채널을 다룬다.
- 믹서의 블록 단위 처리로의 구조 변경, 안 쓰는 공간 게인 계산 제거(검토 보고 A9), 옛 3D Object 경로 삭제. 각각 별도 작업이다(9절).
- 스피커(출력)마다 다른 방 볼륨을 따르게 하기. 사용자 결정으로 하지 않는다.
- OSC로 표 바꾸기, 표 프리셋 저장소.

### 1.4 하면 실패인 것(반목표)
- 표가 없는 옛 프로젝트의 소리가 조금이라도 달라진다.
- 오디오 콜백 안에서 할당·해제·잠금이 생긴다(3법칙 1·2), 표를 바꿀 때 딸깍 소리가 난다(3법칙 3).
- 화면에서는 연결돼 보이는데 소리가 안 나거나, 그 반대(예전 "선택은 되는데 소리가 안 난다" 사고의 재발).
- 설정 파일 왕복(저장 → 열기, 내보내기 → 다른 PC에서 열기)에서 표가 사라진다.

## 2. 지금 동작

- **설정**: `TrackConfig.output_channel: u32`(0부터 — `rust/src/common/config.rs`의 주석 "1 to 24"는 낡았다. Dart가 0부터 넣고 엔진이 그대로 색인으로 쓴다)과 `output_stereo: bool`뿐이다.
- **엔진 쓰기 규칙**(`rust/src/audio/mixer.rs` `process`의 인스턴스 루프, `let routes = [` 근처 879~974행):
  - `!output_stereo`이고 파일이 2채널 이상 → 전 채널 평균을 `output_channel` 하나로.
  - 그 밖 → 파일 채널 c를 출력 `output_channel + c`로. 파일이 모노이고 `output_stereo`면 `output_channel + 1`에도 복사.
  - 모든 쓰기는 `hw_ch < out_channels`와 `GLOBAL_STATE.enabled_channels[hw_ch]`(샘플마다 원자 읽기)를 통과해야 한다.
  - 파일에서 읽는 채널 수는 `ch_limit = channels.min(temp_vals.len())`이고 `temp_vals` 길이는 믹서 폭이다(747행, 358행). 믹서 폭보다 채널이 많은 파일은 앞쪽만 읽는다.
- **실행 중 변경**: `api_set_track_output` → `AudioCommand::SetTrackOutput` → `engine.rs` 818행. 인스턴스가 현재·직전 경로 두 개를 갖고 `output_fade`로 10ms(`OUTPUT_RAMP_S`) 크로스페이드한다. 페이드 도중 또 바뀌면 지금 경로를 옛 경로로 삼아 다시 시작해서, 그보다 옛 경로는 그 순간 끊긴다.
- **게이트·처리 폭**: 어떤 출력이 열리는지는 `api::simple::compute_enabled_channels`가, 믹서가 몇 채널까지 처리하는지는 `core::processing_channels::processing_channel_count`가 **출력 설정(mono/stereo/multi_configs)만 보고** 정한다. 폭이 바뀌면 `request_restart_if_processing_width_changed`가 엔진을 스스로 다시 시작한다(약 1초 끊김 뒤 이어 감).
- **재시작·이어 가기**: `core/restart_resume.rs`·`core/show_state.rs`는 `play_track_from`으로 설정에서 트랙을 다시 만든다.
- **화면**: 트랙 카드(`lib/features/dashboard/widgets/track_card.dart`)와 환경설정 트랙 매핑(`lib/features/settings/widgets/preferences_modal.dart`)이 `buildChannelRoutingItems`(`lib/core/utils/channel_routing.dart`)의 드롭다운 하나로 출력을 고른다. 값은 `mono_N`/`stereo_N`/`multi_N`(N은 0부터)이고 저장은 `outputChannel` + `outputStereo`다.
- 앞선 조사: 저장소 밖 `D:\dev\harness\REPORT_v1.3.10_followup.md` 1절.

## 3. 접근 방식

| 방식 | 내용 | 판단 |
|---|---|---|
| **A. 트랙마다 연결 표(선택)** | `TrackConfig`에 선택 칸 `routes`를 두고, 인스턴스가 미리 잡은 고정 용량 배열에 표를 담아 믹서가 그대로 섞는다 | 소리를 한 번만 읽어 출력끼리 샘플 단위로 같다. 표가 없으면 지금 경로 그대로. 다섯 가지를 한 구조로 한다 |
| B. 앱이 트랙을 복제 | 출력 수만큼 같은 파일을 따로 재생하고 그룹 시작으로 맞춘다 | 디코더·디스크 읽기가 출력 수만큼 늘고, 밀리면 어긋난다. 합치기를 못 한다 |
| C. 출력 쪽 큰 표 | 믹스 뒤 "가상 출력 → 장치 출력"으로 다시 나눈다 | 트랙마다 다르게 보낼 수 없다. 1.3의 "현장 채널 옮기기"에는 맞아 2단계 후보 |

사용자는 만드는 방법을 맡겼고(2026-11-13), Main은 A에 동의했다(같은 날). 방 단위 표는 같은 방에 배치가 다른 파일이 섞여 쓰지 않는다(Main).

## 4. 설계 결정 요약

| # | 결정 | 근거 |
|---|---|---|
| D1 | 표는 `TrackConfig.routes`(빈 목록 = 지금 방식) | 재시작·이어 가기·내보내기가 설정을 따라오므로 자동으로 실린다 |
| D2 | 표가 비어 있지 않으면 **언제나 표 경로**다. `output_channel`·`output_stereo`의 복사·평균 규칙은 무시한다 | 우선순위를 하나로(Main 요청) |
| D3 | 칸의 음량은 선형 `gain`(0 초과 1 이하, 1.0 = 그대로). 화면은 %(트랙 음량과 같은 단위). 증폭 없음 | 화면의 트랙·방 음량이 %다. 올리는 일은 트랙 음량·채널 게인이 한다 |
| D4 | 합칠 때 자동으로 줄이지 않는다. 표의 숫자대로 더한다 | 예측 가능. 넘치면 출력 리미터가 막는다. 옛 "평균" 동작은 표를 처음 열 때 같은 값(1/N)으로 채워 준다(7.3) |
| D5 | 표의 유효 칸이 0개여도(전부 장치 범위 밖 등) 옛 방식으로 돌아가지 않고 **무음**이다. 버린 칸 로그(6.1)가 그 트랙에 한 번 남는다 | 돌아가면 뜻하지 않은 출력으로 소리가 나간다 |
| D6 | 출력이 열리는지는 지금처럼 **출력 설정(Output Config)이 정한다.** 표는 닫힌 출력을 열지 않는다. 표 창은 열린 출력만 열로 보인다 | 지금 드롭다운과 같은 규칙. "닫았는데 소리가 난다"를 막는다. Main 검수 동의(2026-11-14), 사용자 확인 대기(13절 U1) |
| D7 | 믹서 처리 폭은 표가 가리키는(열려 있는) 가장 높은 출력까지 넓힌다 | 안 그러면 폭 밖 출력이 무음이다 |
| D8 | 실행 중 변경은 "현재·직전·대기" 세 자리로 한다. 10ms 크로스페이드 중에 온 변경은 대기에 두었다가 페이드가 끝나면 적용한다 | 한 콜백에 변경이 두 번 오면(슬라이더 드래그) 지금 규칙은 들리던 경로를 끊는다 |
| D9 | 표 내용은 오디오 스레드 밖에서 검증·변환하고, 콜백은 미리 잡은 배열에 복사만 한다 | 3법칙 1·2 |
| D10 | 옛 경로(표 없음)의 코드는 그대로 둔다. 표 경로는 옆에 더한다 | "옛 프로젝트는 똑같이"를 코드 수준에서 지킨다 |
| D11 | 한 트랙의 칸은 최대 512개(`MAX_TRACK_ROUTES`) | 모노 → 256출력, 64채널 1:1을 넘는 여유. 인스턴스당 12KB |
| D12 | 표가 있는 트랙의 마지막 칸은 끊을 수 없다(화면 규칙) | "표는 있는데 연결 0개"인 무음 트랙을 만들지 않는다. 표를 없애려면 "기본 방식으로" |

## 5. 설정 구조

### 5.1 Rust (`rust/src/common/config.rs`)

```rust
/// 트랙 연결 표의 칸 하나: 파일 채널 → 출력 채널, 음량.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackRoute {
    /// 파일 안 채널(0부터).
    pub file_channel: u32,
    /// 출력 채널(0부터). `TrackConfig.output_channel`과 같은 기준이고 화면에는 +1로 보인다.
    pub output_channel: u32,
    /// 선형 음량(0 초과 1 이하, 1.0 = 그대로). 트랙 `volume`과 같은 단위.
    #[serde(default = "default_route_gain")]
    pub gain: f32,
}

fn default_route_gain() -> f32 { 1.0 }

pub struct TrackConfig {
    // … 기존 칸 그대로 …
    /// 연결 표. 비어 있으면 지금 방식(`output_channel`·`output_stereo`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routes: Vec<TrackRoute>,
}
```

- `skip_serializing_if`: 표가 없는 트랙은 `config.json`에 `routes` 키가 없다. 표를 쓰지 않는 프로젝트의 파일은 지금과 같은 모양이다.
- `output_channel`의 낡은 주석("1 to 24 …")을 "0부터"로 고친다.
- `Vec`(Option 아님)으로 둔다. FRB가 만든 Dart `TrackConfig`에서 필수 칸이 되어, 트랙을 다시 만드는 Dart 코드가 빠뜨리면 컴파일러가 잡는다(HANDOFF 운용 메모 "RoomConfig·TrackConfig는 모든 칸이 필수").
- 표가 있는 트랙을 저장할 때 화면은 `output_channel`을 표의 가장 낮은 출력으로 맞춘다(7.4). 옛 버전으로 되돌렸을 때 엉뚱한 채널로 나가지 않게 하는 대비다.

### 5.2 옛 버전과의 관계
- 옛 설정(키 없음) → 새 앱: `routes = []`, 지금 방식.
- 새 설정 → 옛 앱(롤백): 모르는 키는 무시되어 `output_channel`부터 지금 방식으로 난다. 옛 앱이 저장하면 표는 사라진다. 운영 안내서의 롤백 절에 이 한 줄을 넣는다(10절 7단계).

### 5.3 저장되는 곳
`TrackConfig`는 `AppConfig.rooms[].tracks[]` 안에 있어 `config.json`, `.atmos` 프로젝트 파일, 내보내기(`project.atmos`)에 그대로 실린다. 따로 저장 경로를 만들지 않는다.

## 6. 엔진

### 6.1 새 모듈 `rust/src/audio/route.rs`

```rust
/// 한 트랙의 연결 표 칸 수 상한. 인스턴스는 이 용량으로 미리 잡는다(콜백 안 할당 금지).
pub const MAX_TRACK_ROUTES: usize = 512;
/// 표 경로가 한 파일에서 읽는 채널 수 상한(6.5). **따로 두지 않는다**: 안정성 ⑨가 정하는 "파일 채널 최대 수" 상수
/// 하나를 같이 쓴다. ⑨가 상한을 정하지 않았을 때만 이 상수(64)를 둔다(Main 검수).
pub const FILE_CHANNEL_READ_CAP: usize = 64;

/// 엔진이 쓰는 칸(검증·변환 끝).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteEntry {
    pub src: u16,  // 파일 채널
    pub out: u16,  // 출력 채널
    pub gain: f32, // 선형
}

/// 설정의 표를 엔진 칸으로 바꾼 결과와 버린 칸 수.
pub struct ResolvedRoutes {
    pub entries: Vec<RouteEntry>, // 길이 <= MAX_TRACK_ROUTES, (out, src) 오름차순
    pub dropped_bad_gain: usize,
    pub dropped_file_channel: usize,
    pub dropped_output: usize,
    pub dropped_duplicate: usize,
    pub dropped_over_limit: usize,
}

/// 오디오 스레드 밖에서만 부른다(할당). `file_channels`를 알면 그 밖의 칸을 버린다.
pub fn resolve_routes(routes: &[TrackRoute], file_channels: Option<usize>) -> ResolvedRoutes
```

`resolve_routes` 규칙(순서대로):
1. `gain`이 유한하지 않거나 0 이하면 버린다. 1을 넘으면 1.0으로 자른다.
2. `file_channel >= file_channels`(알 때)이거나 `>= u16::MAX`면 버린다.
3. `output_channel >= GLOBAL_STATE.enabled_channels.len()`(4096)이면 버린다. **장치·믹서 폭 밖인지는 여기서 보지 않는다**(엔진이 다른 폭으로 다시 떠도 표가 맞게, 6.4의 콜백 경계 검사가 맡는다).
4. 같은 (file_channel, output_channel)이 또 나오면 뒤의 것을 버린다.
5. (out, src) 오름차순으로 정렬하고 `MAX_TRACK_ROUTES`를 넘는 뒤쪽을 버린다.

버린 칸이 있으면 부른 쪽이 앱 로그에 남긴다(`GLOBAL_STATE.log`): `라우팅 표: 방 '…' 트랙 '…' 칸 N개 중 M개를 쓰지 않는다(음량 a, 파일 채널 b, 출력 c, 겹침 d, 상한 e)`. 같은 트랙은 `config_version`이 바뀔 때까지 한 번만 남긴다(효과음 재트리거마다 쌓이지 않게).

### 6.2 인스턴스(`rust/src/audio/player.rs` `SoundInstance`)

기존 칸(`output_channel`, `output_stereo`, `prev_output_channel`, `prev_output_stereo`, `output_fade`)은 뜻을 유지한다. 더하는 칸:

```rust
/// 현재 경로가 표인가(false면 output_channel·output_stereo의 옛 방식).
pub table_active: bool,
/// 직전 경로가 표였나(output_fade < 1 동안만 뜻이 있다).
pub prev_table_active: bool,
/// 현재·직전·대기 표. 셋 다 MAX_TRACK_ROUTES 용량으로 new()에서 잡고, 콜백은 clear·extend_from_slice·
/// mem::swap만 한다(할당 없음).
pub routes: Vec<RouteEntry>,
pub prev_routes: Vec<RouteEntry>,
pub pending_routes: Vec<RouteEntry>,
/// 페이드 중에 온 변경. 페이드가 끝난 블록 경계에서 적용한다.
pub pending_path: Option<PendingPath>,
```
```rust
#[derive(Clone, Copy)]
pub struct PendingPath { pub is_table: bool, pub output_channel: usize, pub output_stereo: bool }
```

- **`SoundInstance::new`의 인자는 바꾸지 않는다**(시험 20개 파일이 위치 인자로 부른다). 세 `Vec`은 `Vec::with_capacity(MAX_TRACK_ROUTES)`로, `table_active`·`prev_table_active`는 false, `pending_path`는 None으로 시작한다.
- 새 메서드(모두 할당 없음. 길이가 용량을 넘는 입력은 앞에서부터 용량만큼만 받는다):
  - `set_initial_routes(&mut self, entries: &[RouteEntry])`: 재생 전(오디오 스레드 밖)에 부른다. `routes`를 채우고 `table_active = true`. 페이드는 걸지 않는다(`output_fade` 1.0 그대로).
  - `request_table_path(&mut self, entries: &[RouteEntry])`, `request_legacy_path(&mut self, output_channel: usize, output_stereo: bool)`: 실행 중 변경(6.3).
  - `promote_pending(&mut self)`: 대기를 현재로(6.3).

### 6.3 실행 중 변경: 현재·직전·대기

`request_*`의 공통 규칙:
1. 바꾸려는 경로가 지금 **요청된 최종 경로**(대기가 있으면 대기, 없으면 현재)와 같으면 아무것도 하지 않는다. 표끼리는 칸 목록이 같을 때, 옛 방식끼리는 채널·스테레오가 같을 때.
2. `output_fade >= 1.0`(페이드 중이 아님)이면 **바로 전환**한다:
   - 현재를 직전으로 옮긴다: `mem::swap(routes, prev_routes)`, `prev_table_active = table_active`, `prev_output_channel = output_channel`, `prev_output_stereo = output_stereo`.
   - 새 경로를 현재에 넣는다: 표면 `routes.clear(); routes.extend_from_slice(entries); table_active = true`, 옛 방식이면 `output_channel`·`output_stereo`를 넣고 `table_active = false`.
   - `output_fade = 0.0`.
3. `output_fade < 1.0`(페이드 중)이면 **대기에 둔다**: 표면 `pending_routes`에 복사하고 `pending_path = Some(PendingPath { is_table: true, .. })`, 옛 방식이면 채널·스테레오를 실어 둔다. 대기가 이미 있으면 덮는다(대기는 한 번도 들린 적이 없으므로 딸깍이 없다).

`promote_pending`: 믹서가 **블록 시작**(활성 인스턴스를 추리는 단계)에 인스턴스마다 `pending_path.is_some() && output_fade >= 1.0`이면 부른다. 2번과 같은 전환을 하되 표는 `mem::swap(routes, pending_routes)`로 가져온다(직전으로 옮긴 뒤). `pending_path = None`. 대기가 현재와 같은 경로면 전환하지 않고 대기만 지운다.

- 결과: 변경이 아무리 몰려도 들리던 경로는 언제나 10ms에 걸쳐 줄고, 마지막 요청이 최대 "남은 페이드 + 1블록" 뒤에 적용된다.
- **`SetTrackOutput`도 이 규칙을 쓴다**(`request_legacy_path`). 지금의 "페이드 도중 또 바뀌면 그보다 옛 경로는 끊긴다"가 없어진다. 의도한 개선이고, `test_master_switch_ramps`의 출력 채널 시험(한 번 바꾸기)은 그대로 통과해야 한다.
- 대기(armed) 인스턴스와 아직 한 번도 소리 나지 않은 인스턴스도 같은 규칙을 쓴다(따로 가르지 않는다).

### 6.4 믹서(`rust/src/audio/mixer.rs`)

**블록 시작**(`process` 앞부분):
- `temp_enabled: Vec<bool>`(믹서 폭 길이, `new()`에서 미리 잡음)에 `GLOBAL_STATE.enabled_channels[..out_channels]`를 한 번 읽어 둔다. 표 경로는 이 사본을 쓴다(샘플마다 원자 읽기를 하지 않는다). 옛 경로는 지금처럼 원자를 읽는다(코드를 건드리지 않는다, D10). 인스턴스를 `&mut`로 빌린 동안 쓰므로 `temp_vals`처럼 `std::mem::take`로 꺼냈다가 블록 끝에 돌려놓는다.
- 활성 인스턴스를 추릴 때 `promote_pending`을 부른다(6.3).

**샘플 루프**(지금 `let routes = [` … `for &(output_channel, output_stereo, route_gain) in routes.iter().take(route_count)` 자리):
- 경로 두 개(현재: 비중 `fade`, 직전: 비중 `1 - fade`, 직전은 `fade < 1`일 때만)를 도는 구조는 유지한다. 각 경로가 표인지(`table_active` / `prev_table_active`)에 따라 가른다.
  - 옛 방식이면 **지금 코드 그대로**(Object 분기·평균·N:N·모노 복사).
  - 표면:
    ```rust
    let route_vol = current_vol * route_gain; // 지금과 같은 식
    for e in table.iter() {
        let (s, o) = (e.src as usize, e.out as usize);
        if s < read_limit && o < out_channels && temp_enabled[o] {
            output[frame * out_channels + o] += vals[s] * (route_vol * e.gain);
        }
    }
    ```
    `table`은 현재면 `&instance.routes`, 직전이면 `&instance.prev_routes`다(빌림 충돌을 피하려면 `output`·`vals`·`temp_enabled`와 인스턴스 칸을 따로 빌린다 — 구현자가 정함).
- 같은 파일 채널을 두 출력으로 보내면 곱하는 값이 같아(`gain`이 같을 때) 두 출력의 샘플이 비트 단위로 같다(채널 DSP 앞 기준).
- 페이드 전진(`instance.output_fade = (fade + output_fade_step).min(1.0)`), 스트림이 비었을 때의 처리, 커서 전진은 지금 그대로다.
- 표 경로도 `has_sample`이 참일 때만 돈다(지금 구조 안).
- 대기(armed) 인스턴스는 그 앞에서 `continue`하므로 표와 상관없이 소리가 없다. `has_sfx`·더킹 판단은 바꾸지 않는다.

**명령 처리**(`rust/src/audio/engine.rs` `process_commands`):
- `AudioCommand::SetTrackOutput`: 지금 본문을 `inst.request_legacy_path(output_channel, output_stereo)`로 바꾼다.
- 새 명령:
  ```rust
  /// 트랙의 연결 표를 바꾼다. `entries`는 오디오 스레드 밖에서 검증·변환했다(audio::route::resolve_routes).
  /// 콜백은 인스턴스가 미리 잡은 배열에 복사만 하고, 받은 Vec은 정리 스레드로 넘긴다.
  SetTrackRoutes { room_id: u32, track_id: u32, entries: Vec<RouteEntry> },
  ```
  처리: 같은 `(room_id, track_id)`의 모든 인스턴스에 `request_table_path(&entries)`. 그 뒤 `entries`를 `spatial_gc_tx.try_send(SpatialGarbage::Routes(entries))`로 넘긴다. **가득 차서 돌아오면** 믹서의 `route_garbage: Vec<Vec<RouteEntry>>`(용량 64, `new()`에서 미리 잡음)에 넣고, 그것도 차 있으면 `std::mem::forget`한다(지금 `local_recycle`의 "Emergency buffer leak"와 같은 방식 — 콜백에서 해제하지 않는다). `route_garbage`는 블록 끝에서 `local_recycle`을 비우는 자리 옆에서 다시 보낸다.
  새는 것이 보이게 한다(Main 검수): `forget`한 횟수와 `route_garbage`에 넣은 횟수를 `GLOBAL_STATE`의 원자 카운터에 더하고(콜백은 원자 저장만), 엔진 스레드의 점검 루프(부하 로그를 쓰는 곳)가 0이 아니면 앱 로그에 남기고 비운다.
- `match`에 catch-all을 두지 않는 규칙을 지킨다(새 변형을 열거).

### 6.5 믹서 폭보다 채널이 많은 파일
지금은 `temp_vals` 길이(= 믹서 폭, 최소 16)까지만 파일 채널을 읽는다. 표에서는 24채널 파일의 20번 채널을 CH3으로 보내는 일이 자연스러우므로 읽는 한도를 폭에서 뗀다.
- `temp_vals`를 `new()`에서 `vec![0.0; channels.max(파일 채널 최대 수)]`로 **한 번만** 잡는다. 실행 중에는 늘리지 않는다. 상수는 ⑨가 정한 것을 쓰고, 없으면 `FILE_CHANNEL_READ_CAP`(64)이다.
- 인스턴스 루프에서 두 한도를 쓴다.
  - `legacy_limit = channels.min(믹서 폭)` — 지금의 `ch_limit`과 같은 값. 옛 방식의 모든 식(평균의 분모, N:N 반복, `ch_limit == 1` 판단)은 이것을 쓴다. 믹서 폭은 `AudioMixer`에 `width: usize` 칸을 더해 `new()`에서 넣는다.
  - `read_limit = if 현재나 직전 경로가 표 { channels.min(temp_vals.len()) } else { legacy_limit }` — 파일에서 읽어 `vals`에 채우는 채널 수.
- 표가 없는 인스턴스는 `read_limit == legacy_limit`이라 지금과 같은 일을 한다.
- 안정성 ⑨가 이 부분(폭과 파일 채널 수의 관계)을 먼저 바꿨으면 ⑨의 구조를 따르고 이 절은 그에 맞춰 줄인다(11절).

### 6.6 재생 시작(`rust/src/api/simple.rs`)
- `build_play_track_command`의 인자는 바꾸지 않는다. `with_start` 옆에 새 도우미를 둔다.
  ```rust
  /// 트랙에 연결 표가 있으면 재생 명령의 인스턴스를 표 경로로 시작하게 한다(오디오 스레드 밖).
  fn with_routes(mut cmd: AudioCommand, room: &RoomConfig, track: &TrackConfig) -> AudioCommand
  ```
  `track.routes`가 비어 있으면 그대로 돌려준다. 아니면 `resolve_routes(&track.routes, Some(instance.stream_channels as usize))` → `instance.set_initial_routes(&resolved.entries)`, 버린 칸 로그(6.1).
- `play_track_from_in`의 재생 명령을 만드는 네 곳(스트리밍, 미리 읽은 음원, 캐시, 캐시에 없어 읽은 경우)을 모두 `with_start(with_routes(build_play_track_command(…), room, track), start_seconds, group_id)`로 감싼다. 테마 시작·방 클리어·OSC·이어 가기·재시작 복원은 모두 이 함수를 지나므로 따로 고칠 곳이 없다.
- 새 FRB 함수:
  ```rust
  /// 재생 중인 트랙의 연결 표를 바꾼다(화면이 표를 고칠 때). `routes`가 비어 있으면 오류 —
  /// 옛 방식으로 돌아갈 때는 api_set_track_output을 쓴다.
  pub fn api_set_track_routes(room_id: String, track_id: String, routes: Vec<TrackRoute>) -> Result<(), AtmosError>
  ```
  `resolve_routes(&routes, None)`(파일 채널 수는 인스턴스가 안다. 콜백의 `s < read_limit`이 거른다) → `AudioCommand::SetTrackRoutes`. `core::event_log::record_request("routes", …)`는 남기지 않는다(드래그 중 연속 호출).
- `api_set_track_output`은 시그니처 그대로다. 뜻이 "옛 방식 경로로 바꾼다"가 된다.

### 6.7 게이트와 처리 폭
- **게이트(`compute_enabled_channels`)는 바꾸지 않는다**(D6).
- **처리 폭(`core/processing_channels.rs` `processing_channel_count`)**: 지금 결과를 `base`라 하면, 모든 방·트랙의 `routes`에서 `output_channel < hw_channels`이고 게이트가 열린(`compute_enabled_channels(config, hw_channels)[output_channel]`) 칸의 `output_channel + 1` 최댓값을 `routes_highest`로 구해 `base.max(routes_highest)`를 돌려준다. 표가 하나도 없으면 지금과 같은 값이다.
- **열린 출력이 폭을 넘는 경우는 하나뿐이다**(Main이 d60b8c5에서 확인): Multi 그룹이 더 높은 Mono·Stereo 그룹 아래에 있을 때. Multi k는 k부터 장치 끝까지 열지만 폭은 가장 높은 Mono·Stereo까지다(예: Multi 1 + Mono 10 → 열림은 CH1~끝, 폭은 10 → 믹서 폭 16). 그 밖의 구성에서는 열린 출력이 늘 폭 안이라 표 때문에 폭이 바뀌지 않고, 아래 확인 창도 뜨지 않는다.
- 폭이 달라지면 지금처럼 `request_restart_if_processing_width_changed`가 엔진을 다시 시작한다(설정 저장 때). 표를 고치다가 예고 없이 약 1초 끊기지 않게, 화면이 미리 물을 수 있도록 새 FRB 함수를 둔다.
  ```rust
  /// 이 설정을 저장하면 믹서 처리 폭이 달라져 엔진이 다시 시작되는가. 엔진이 꺼져 있으면 false.
  pub fn api_would_restart_for_config(config: AppConfig) -> bool
  ```
  `request_restart_if_processing_width_changed`의 판단 부분을 순수 함수로 떼어 둘이 함께 쓴다.
- 화면은 "열린 출력"을 엔진과 같은 규칙으로 알아야 한다. `compute_enabled_channels`는 `rust/src/api/`의 `pub fn`이라 이미 Dart에 `computeEnabledChannels({config, hwLen})`로 나와 있다(`lib/src/rust/api/simple.dart`). 표 창은 이것을 부른다. Dart에 같은 규칙을 다시 쓰지 않는다(두 쪽 규칙이 어긋나 "선택은 되는데 소리가 안 난다"가 났던 이력).
- 그래서 이 작업이 더하는 FRB 함수는 `api_set_track_routes`와 `api_would_restart_for_config` 두 개다. `rust/src/api/`의 `pub fn`은 모두 Dart로 나가므로 도우미(`with_routes` 등)는 `pub`으로 두지 않는다.

### 6.8 바뀌지 않는 것
- 방 볼륨·트랙 볼륨·페이드·더킹은 `current_vol`에 들어 있어 표의 모든 칸에 같이 걸린다.
- 베이스 매니지먼트·채널 DSP(지연·EQ·게인·극성·리버브)·바이노럴·리미터·VU·분석은 믹스 뒤 출력 채널 단위라 그대로다. 한 소리를 세 출력으로 보내면 각 출력의 보정이 따로 걸린다.
- 재생 위치·그룹 시작·lag 건너뛰기·이어 가기는 커서 쪽이라 그대로다.
- 옛 3D Object 분기(`output_channel == usize::MAX`)는 손대지 않는다. 표가 있는 인스턴스는 표 경로가 먼저다.
- OSC 규격(주소·응답·스냅샷)은 그대로다.

## 7. 화면(Dart)

### 7.1 새 순수 함수 `lib/core/utils/track_routes.dart`
- `List<TrackRoute> normalizeRoutes(List<TrackRoute> routes)`: `gain <= 0` 제거, 1 초과는 1로, 같은 (파일, 출력)은 뒤의 것만 남김, (출력, 파일) 오름차순.
- `List<TrackRoute> seedRoutesFromLegacy({required int fileChannels, required int outputChannel, required bool outputStereo})`: 지금 방식과 같은 소리가 나는 표(7.3).
- `List<TrackRoute> toggleRoute(List<TrackRoute> routes, int fileChannel, int outputChannel)`: 없으면 `gain 1.0`으로 더하고, 있으면 뺀다. 빼서 0개가 되면 원래 목록을 그대로 돌려준다(D12).
- `List<TrackRoute> setRouteGain(List<TrackRoute> routes, int fileChannel, int outputChannel, double gain)`: `gain`을 0.01~1.0으로 자른다.
- `String routesSummary(List<TrackRoute> routes)`: 출력 번호를 모아 `표 · Ch-1, 3, 5`(여섯 개를 넘으면 `… 외 n`).
- `int highestRouteChannel(List<TrackRoute> routes)`: 가장 높은 출력의 1부터 센 번호(없으면 0).
- `RouteProblems routeProblems(List<TrackRoute> routes, {required int? fileChannels, required List<bool> enabled, required int deviceChannels})`: 파일에 없는 채널 칸, 장치에 없는 출력 칸, 닫힌 출력 칸의 목록.

### 7.2 표 창 `lib/features/dashboard/widgets/track_routing_dialog.dart`

```
라우팅 — 방 1 · 1번 트랙 (모노, 1채널)                          [기본 방식으로] [처음 상태로] [닫기]
            Ch-1      Ch-2      Ch-3      Ch-4      Ch-5      Ch-6   →(가로 스크롤)
            Analog 1  Analog 2  Analog 3  Analog 4  Analog 5  Analog 6
파일 1      [100%]    [    ]    [ 70%]    [    ]    [ 50%]    [    ]
────────────────────────────────────────────────────────────────────────
선택: 파일 1 → Ch-3    음량 [=======|---] 70%    [연결 끊기]
```

- **행**: 파일 채널 1…N(`apiGetAudioFileChannels`). 파일을 못 읽으면 표에 있는 가장 높은 파일 채널까지 보이고 "파일을 읽지 못해 채널 수를 모릅니다"를 띄운다.
- **열**: 열린 물리 출력 = `computeEnabledChannels(config: …, hwLen: 장치 채널 수)`가 참이고 `isPhysicalOutputChannel`인 채널. 장치 채널 수와 이름은 `outputChannelsProvider`의 `channelNames`(트랙 카드가 이미 본다). 머리글은 `Ch-N`과 장치 이름(`hardwareChannelName`), 출력 설정의 사용자 이름이 있으면 함께(지금 드롭다운의 `_withCustomSuffix`와 같은 규칙).
- **문제 있는 칸**: 표에는 있는데 열에 없는 출력(장치에 없음·닫힘·가상 채널)은 오른쪽 끝에 주황색 열로 따로 보이고 머리글에 이유를 쓴다(`channelOutOfRangeName` 등). 파일에 없는 채널의 칸은 주황색 행으로 보인다. 둘 다 끊을 수 있고, 저장값은 사용자가 끊기 전까지 그대로 둔다.
- **조작**:
  - 빈 칸을 누르면 100%로 연결하고 그 칸을 선택한다.
  - 연결된 칸을 누르면 선택한다. 아래 줄에서 음량(1~100%, 정수)을 슬라이더로 바꾸거나 "연결 끊기"를 누른다. 마지막 남은 칸이면 "연결 끊기"가 꺼지고 "연결이 하나는 있어야 합니다. 표를 없애려면 '기본 방식으로'를 누르세요"를 보인다.
  - "기본 방식으로": 표를 지운다(지금 방식으로 돌아감). 확인을 묻는다.
  - "처음 상태로": 창을 열었을 때의 값으로 되돌린다.
- **도움말(사용자 요구 2026-11-14)**: 창 제목 줄에 `?` 아이콘 단추(`Icons.help_outline`, 툴팁 "사용법")를 둔다. 누르면 창 안에 아래 설명이 펼쳐지고 다시 누르면 접힌다(펼친 채로도 표를 조작할 수 있어야 한다). 문구는 이대로 쓴다.
  > **여러 출력으로 보내기**
  > - 왼쪽은 파일의 채널, 위쪽은 출력(스피커)입니다.
  > - 빈 칸을 누르면 그 파일 채널이 그 출력으로 나갑니다. 한 줄에서 여러 칸을 켜면 같은 소리가 여러 출력에서 동시에 납니다.
  > - 연결된 칸을 누르면 아래에서 음량(1~100%)을 바꾸거나 연결을 끊을 수 있습니다.
  > - 여러 파일 채널을 같은 출력에 연결하면 소리가 더해집니다. 너무 크면 음량을 낮추세요.
  > - 재생 중에 바꿔도 바로 반영됩니다.
  > - 방 볼륨·정지·방 클리어는 이 트랙의 모든 출력에 함께 적용됩니다.
  > - 출력 설정(Output Config)에서 닫은 출력은 여기에 나오지 않습니다. 보내려면 먼저 출력 설정에서 여세요.
  > - 주황색은 지금 장치에 없거나 닫힌 출력, 또는 파일에 없는 채널입니다. 그 칸은 소리가 나지 않습니다.
  > - "기본 방식으로"는 표를 지우고 예전처럼 보냅니다. "처음 상태로"는 이 창을 열었을 때로 되돌립니다.
- **크기**: 열 94개·행 64개까지 스크롤로 쓸 수 있어야 한다(행 머리글·열 머리글 고정). 1024×768 기본 창에서 아래 음량 줄이 가려지지 않아야 한다(인스펙터 LFE 스위치가 가려졌던 이력). 도움말을 펼쳤을 때도 같다(도움말이 길면 도움말 안에서 스크롤).
- 창은 값을 스스로 저장하지 않는다. 부모가 준 콜백을 부른다.
  ```dart
  /// [commit]이 false면 드래그 중(엔진에만 반영), true면 확정(저장까지).
  typedef RoutesChanged = void Function(List<TrackRoute> routes, {required bool commit});
  ```
  슬라이더를 끄는 동안은 초당 30회 이하로 `commit: false`, 놓으면 `commit: true`를 한 번 부른다. 칸 켜기·끊기·"기본 방식으로"·"처음 상태로"는 바로 `commit: true`다.

### 7.3 표를 처음 열 때
트랙에 표가 없으면 창은 `seedRoutesFromLegacy`의 값으로 시작한다(아직 저장하지 않는다. 사용자가 무엇이든 바꾸는 순간 그 표가 저장된다).

| 지금 방식 | 채워지는 표 |
|---|---|
| 모노 파일, 모노 출력 | 파일 1 → `out` 100% |
| 모노 파일, 스테레오 출력 | 파일 1 → `out` 100%, 파일 1 → `out+1` 100% |
| N채널 파일(N≥2), 스테레오/멀티 출력 | 파일 c → `out+c` 100% (c = 0…N−1) |
| N채널 파일(N≥2), 모노 출력(평균) | 파일 c → `out` (100/N)% — 스테레오면 50%씩 |

(100/N)%는 화면에 정수로 반올림해 보이되 저장값은 `1/N` 그대로 둔다.

### 7.4 트랙 카드와 환경설정
- **트랙 카드**(`track_card.dart`): 출력 드롭다운 오른쪽에 표 창을 여는 단추(격자 아이콘, 툴팁 "여러 출력으로 보내기")를 둔다. 표가 있는 트랙은 드롭다운 자리에 `routesSummary`를 보이고(문제 있는 칸이 있으면 주황색, 툴팁에 이유), 드롭다운은 보이지 않는다. 새 콜백 `onRoutesChanged`(RoutesChanged 형)와 표 창에 줄 값(채널 이름, 열린 출력)을 받는다.
- **방 카드**(`room_card.dart`, 트랙 카드의 부모): `onRoutesChanged`에서
  - `commit: true`이면 **먼저** 새 `TrackConfig`(표와, 표가 있으면 `outputChannel` = 표의 가장 낮은 출력)를 넣은 설정으로 `apiWouldRestartForConfig`를 묻는다. 참이면 "이 변경은 처리 채널 수를 바꿔 오디오 엔진을 다시 시작합니다(약 1초 소리가 끊깁니다). 계속할까요?"를 띄우고, 아니오면 엔진에도 설정에도 보내지 않고 표 창의 값을 직전 상태로 되돌린다.
  - 엔진 반영: 표가 비어 있지 않으면 `rust_api.apiSetTrackRoutes(...)`, 비었으면(기본 방식으로) `rust_api.apiSetTrackOutput(...)`(지금 출력 변경과 같은 순서 — 엔진 먼저, 저장 다음).
  - 저장: `commit: true`일 때만 `configProvider.notifier.saveConfig(…, skipPreload: true)`. 드래그 중(`commit: false`)에는 엔진에만 보낸다(이미 연결된 칸의 음량만 바뀌므로 처리 폭은 달라지지 않는다). 드래그 중 값은 저장 전이라, 그 사이 엔진이 스스로 다시 시작하면 이어 가는 트랙은 **저장된 표**로 돌아간다(의도된 동작).
- **환경설정 트랙 매핑**(`preferences_modal.dart`): 같은 단추와 요약을 둔다. 여기서는 지금처럼 `_tempConfig`만 바꾸고(엔진에 바로 보내지 않음) 환경설정 저장 때 함께 저장된다.
- 두 화면이 같은 요약 문구·같은 창을 쓴다(지금 "두 화면이 같은 라벨" 규약).
- 파일을 바꿔 채널 수가 달라진 트랙의 표는 지우지 않는다. 문제 있는 칸으로 보인다.

### 7.5 그 밖에 고칠 Dart
- `TrackConfig(`를 만드는 모든 곳에 `routes:`를 넣는다(컴파일러가 알려 준다). d60b8c5 기준: `lib/core/state/project_export.dart` 1곳, `room_card.dart` 6곳, `preferences_modal.dart` 6곳, 시험 `test/` 6개 파일과 `integration_test/project_media_relink_test.dart`. 기존 값을 옮기는 곳은 `routes: track.routes`, 새 트랙은 `routes: const []`.
- `lib/core/state/project_media.dart` `highestProjectChannel`: 표가 있는 트랙은 `highestRouteChannel(track.routes)`를 본다(표가 없으면 지금 식).
- `lib/core/state/config_copy.dart`: `AppConfig.copyWith`는 `rooms`를 통째로 옮기므로 바꿀 것이 없다(시험 `test/config_copy_test.dart`의 `full()`에 표가 있는 트랙을 하나 넣어 확인).

### 7.6 FRB
`rust/src/api/`와 `common/config.rs`가 바뀌므로 `flutter_rust_bridge_codegen generate`(2.12.0)로 다시 만든다. 생성 파일(`lib/src/rust/**`, `rust/src/frb_generated.rs`)만 커밋하고, 줄바꿈만 바뀐 플러그인 등록 파일은 커밋하지 않는다(PR #98 때와 같다). Mac 세션이 다시 생성해 차이가 0인지 확인한다.

## 8. 시험

### 8.1 깨지면 안 되는 기존 시험
전체 `cargo test`·`flutter test`가 기준이다. 특히: `test_audio_thread_no_heap`, `test_instance_gc_never_drops_in_callback`, `test_multichannel_passthrough`, `test_mono_to_stereo_upmix`, `test_processing_channels`, `test_enabled_channels_multi_config`, `test_master_switch_ramps`, `test_group_start`, `test_group_start_theme`, `test_clear_room_advance`, `test_restart_resume`, `test_restart_resume_races`, `test_stream_resume_onset`, `test_analysis_channel_width`, `test_bass_management_routing`, `test_bass_route_per_room`, `test_binaural_outputs_audio`, `test_phase_invert_no_click`, `test_eq_smoothing_no_click`, `test_restart_startup_ramp`, `test_startup_ramp_channel_independent`, `test_zero_defect_e2e`, Dart `config_copy_test`·`project_export_test`·`project_media_test`·`track_card_controls_test`(Main 목록).
`TrackConfig { … }`를 직접 쓰는 Rust 시험 17개 파일은 `routes: vec![]`를 더해야 컴파일된다(동작 변화 없음).

### 8.2 새 Rust 시험
가능하면 실패하는 시험 → 구현 → 통과 순서로 한다.

`rust/tests/test_track_routes.rs`(순수 함수):
- 0 이하·NaN 음량은 버리고 1 초과는 1로 자른다.
- 파일 채널 수를 넘는 칸, 4096 이상 출력은 버린다. 겹친 칸은 앞의 것만 남는다. 512개를 넘으면 뒤를 버린다. 결과는 (출력, 파일) 오름차순이다.
- 옛 `config.json`(키 없음)을 읽으면 `routes`가 비어 있다. 빈 표는 저장할 때 키가 없다. 표가 있는 설정은 저장 → 읽기에서 같다.

`rust/tests/test_matrix_routing.rs`(오프라인 믹서, `test_multichannel_passthrough`의 방식 — 파일 채널마다 다른 순음, 채널 DSP 중립, 부팅 램프 끝):
- **나누기**: 모노 → CH1·CH3·CH5. 세 출력이 샘플 단위로 **같고**(비트 비교), CH2·CH4는 0이다.
- **음량**: 100%·50%면 두 출력의 비가 0.5다(오차 1e-6).
- **순서 바꾸기·건너뛰기**: 4채널 → CH3·CH1·CH6·CH2. 각 출력에 맞는 순음만 있다.
- **합치기**: 스테레오 두 채널 → CH2. 출력이 두 순음의 합이다(평균이 아님).
- **옛 방식과 같은 소리**: 7.3의 네 경우마다, 옛 방식 인스턴스의 출력과 `seedRoutesFromLegacy`에 해당하는 표 인스턴스의 출력 차이가 1e-6 이하다(Rust 쪽에 같은 표를 직접 적는다).
- **게이트**: 닫힌 출력으로 가는 칸은 무음이고 다른 칸은 난다. 믹서 폭 밖 출력의 칸, 파일에 없는 채널의 칸이 있어도 패닉이 없고 나머지는 난다.
- **유효 칸 0개**: 무음이고 옛 방식으로 돌아가지 않는다(D5).
- **폭보다 채널이 많은 파일**: 믹서 폭 16, 24채널 파일의 20번 채널 → CH3이 난다. 같은 파일을 표 없이 틀면 지금과 같은 출력이다(6.5).
- **방과 함께**: 방 볼륨을 반으로 하면 모든 표 출력이 반이 되고, `ClearRoom`·`StopTrack`이면 모두 멈춘다.
- **더킹**: 더킹이 걸리면 표의 모든 출력이 함께 줄어든다(더킹 설정 작업이 병합돼 있으면 그 설정으로, 아니면 지금 규칙으로).
- **대기(armed)**: 표가 있는 그룹 인스턴스는 `StartGroup` 전에 어떤 출력에도 소리가 없다.

`rust/tests/test_matrix_route_change_no_click.rs`(`test_master_switch_ramps`의 방식 — 1kHz 사인, 인접 샘플 변화가 정상 상태의 2배를 넘지 않는다):
- 옛 방식 → 표, 표 → 표, 표 → 옛 방식, 음량만 바꾸기.
- **한 콜백에 변경 5번**(같은 블록 앞에 명령 5개): 딸깍이 없고 마지막 표로 끝난다.
- **페이드 도중 변경**: 딸깍이 없고, 남은 페이드 + 1블록 + 10ms 안에 새 표로 끝난다.
- `SetTrackOutput`을 한 콜백에 두 번 보내도 딸깍이 없다(6.3의 개선).

`rust/tests/test_audio_thread_no_heap.rs`에 더하기:
- `SetTrackRoutes` 명령 처리(정리 채널이 받는 정상 경우)가 힙을 건드리지 않는다.
- 표 인스턴스를 포함한 `process`(페이드 중, 대기 승격 포함)가 힙을 건드리지 않는다.
- 정리 채널이 가득 찬 경우에도 콜백에서 해제하지 않는다(`route_garbage`로 간다 — `test_instance_gc_never_drops_in_callback`의 방식). 시험은 `mixer.spatial_gc_tx`(pub 칸)를 받는 쪽이 비우지 않는 `bounded(1)` 채널로 바꿔 가득 찬 상태를 만든다.

`rust/tests/test_processing_channels.rs`에 더하기:
- Multi 1 + Mono 10을 켠 설정(열림 CH1~끝, 지금 폭 10)에서 표가 CH20을 가리키면 폭이 20이다(6.7의 유일한 경우). 닫힌 출력·장치 밖 출력을 가리키는 칸은 세지 않는다. 표가 없으면 지금 값과 같다.

`rust/tests/test_play_track_routes.rs`(`test_group_start_theme`의 방식 — 설정을 넣고 `api_play_track` 뒤 명령 큐를 꺼내 본다):
- 표가 있는 트랙의 `PlayTrack` 인스턴스가 `table_active`이고 칸이 설정과 같다. 스트리밍·미리 읽은 음원·캐시·캐시 미스 네 경로 모두.
- 네 경로 모두 `resolve_routes`가 **실제 파일 채널 수**를 쓴다(스트리밍은 스트리머의 채널 수, 메모리 트랙은 `SoundData.channels`): 2채널 파일에 파일 채널 3번 칸이 있으면 네 경로 모두 그 칸이 빠진다.
- `api_set_track_routes`가 `SetTrackRoutes`를 보낸다. 빈 표는 오류다.
- `api_would_restart_for_config`: 엔진이 꺼져 있으면 false.

### 8.3 새 Dart 시험
- `test/track_routes_test.dart`: 7.1의 함수 전부(7.3의 네 경우, 마지막 칸은 안 빠짐, 요약 문구, 문제 칸 분류).
- `test/track_routing_dialog_test.dart`: 빈 칸을 누르면 `commit: true`로 100% 칸이 더해진 목록이 온다. 마지막 칸은 끊기 단추가 꺼져 있다. 문제 열이 주황색으로 보인다. 1024×768에서 음량 줄이 화면 안에 있다. `?` 단추를 누르면 도움말 문구가 보이고 다시 누르면 사라지며, 펼친 상태에서도 칸을 누를 수 있고 음량 줄이 화면 안에 있다.
- `test/track_card_controls_test.dart`에 더하기: 표가 있으면 요약이 보이고 드롭다운이 없다.
- `test/project_media_test.dart`에 더하기: `highestProjectChannel`이 표의 출력을 센다.
- `test/project_export_test.dart`에 더하기: 내보낸 `project.atmos`에 표가 그대로 있다.

### 8.4 부하 측정(이 PC, 장치 없음)
- `rust/src/bin/mixer_bench.rs`와 같은 방식의 새 벤치 `rust/src/bin/route_bench.rs`: 믹서 폭 94, 블록 1024, (가) 표 없는 스테레오 3트랙 (나) 같은 3트랙을 `seedRoutesFromLegacy`와 같은 표로 (다) 모노 3트랙을 각각 12출력으로 (라) 모노 1트랙을 94출력으로. 블록당 평균·최대 처리 시간을 구현 전(같은 벤치를 구현 전 커밋에서 (가)만)과 구현 후에 잰다.
- 기준: (가)는 구현 전후 차이가 측정 편차 안(같은 조건 5회 실행의 최댓값−최솟값보다 작음). (나)~(라)는 값을 기록한다(Main 예상: 채널 DSP에 비해 작다).

### 8.5 앱 실행·장치 확인(Windows, RME ASIO MADIface USB)
소리가 나는 시험이다. 시작 전에 장치와 음량을 사용자에게 확인받고, 앱 데이터는 `backup-appdata.ps1`로 백업한다.
1. 모노 트랙을 표로 CH1·CH3·CH5에 보낸다 → TotalMix 미터에서 세 채널이 같은 레벨로 움직인다(장치로 확인). 사람이 들어 확인.
2. 재생 중 칸을 켜고 끄고 음량을 끈다 → 딸깍·끊김 없음(사람이 들어 확인). 앱 로그에 부하 경고·엔진 재시작이 없다.
3. 폭을 넘는 출력을 연결 → 확인 창이 뜨고, 예를 누르면 엔진이 다시 시작한 뒤 이어서 난다.
4. 표가 있는 루프 트랙을 틀어 둔 채 앱을 강제 종료 → 감시가 다시 띄우고 표대로 이어 간다.
5. 표가 있는 프로젝트를 내보내 다른 폴더에서 열기 → 표가 같다. 맥에서 만든 표 프로젝트를 Windows에서 열기(Mac 세션 협조).
6. 원래 프로젝트(표 없음, 루프 3개)를 30분 재생 → 부하 평균이 구현 전 기록(1.3.12: 평균 2.9~3.3ms, 12채널 처리)과 같은 수준, 경고 0.
7. 재생 중 음량 슬라이더를 30초 동안 계속 끈다 → 앱 로그에 부하 경고·엔진 재시작·`route_garbage`/forget 로그가 없다.
8. 사용자가 `?` 도움말만 읽고 모노 트랙을 세 출력으로 보내 본다(사람이 확인 — 14.2).

옛 버전이 표가 든 설정을 여는 동작(5.2)은 실행해 보지 않는다. 근거는 코드다: `TrackConfig`에 `deny_unknown_fields`가 없어 모르는 키를 무시한다(d60b8c5에서 확인).

### 8.6 macOS
Mac 세션에 요청: FRB 다시 생성 차이 0, `cargo test`, `flutter test`, `flutter analyze`, 앱에서 표 창 열기·칸 켜기·재생 중 변경(사람이 들어 확인).

## 9. 함께 정리할 일(이 작업 밖, 순서만 적는다)
- **구현 전에**: 안정성 ③~⑨. 검토 보고 A9(안 쓰는 공간 게인 계산 제거 — Main 동의: 매트릭스 구현 전).
- **별도 작은 PR 후보**: 옛 3D Object 분기(`usize::MAX`) 삭제. 지금 닿을 수 없는 코드다(Main 확인). 매트릭스 앞에 지우면 핫루프의 분기가 줄어 좋다(Main). 지울지는 사용자 확인 뒤 정한다(U5). 이 작업은 그 분기가 있든 없든 성립한다.
- **이 작업 뒤 후보**: 블록 단위 처리(표의 `RouteEntry` 목록은 `out[o] += in[s] * g`를 블록으로 도는 형태로 그대로 옮길 수 있다), 옛 경로의 게이트 원자 읽기를 블록당 한 번으로, 현장 채널 옮기기(패치 층).

## 10. 구현 순서
브랜치 `win/matrix-routing`(구현 시점의 origin/main에서). Draft PR을 먼저 열고 단계마다 커밋한다. 각 단계 끝에 적힌 확인이 통과해야 다음으로 간다.

1. **설정 구조**: `TrackRoute`·`TrackConfig.routes`, FRB 다시 생성, Rust·Dart의 모든 `TrackConfig` 생성 지점 수정. → `cargo test`·`flutter test`·`flutter analyze` 전부 지금과 같은 결과(동작 변화 없음). 새 시험: 8.2의 설정 왕복.
2. **순수 함수**: `audio/route.rs`의 `resolve_routes`. → `test_track_routes`.
3. **인스턴스와 변경 규칙**: 6.2·6.3. `SetTrackOutput`을 `request_legacy_path`로. → `test_master_switch_ramps` 그대로 통과, 새 `SetTrackOutput` 두 번 시험.
4. **믹서 표 경로와 명령**: 6.4·6.5, `SetTrackRoutes`, `SpatialGarbage::Routes`, `route_garbage`. → `test_matrix_routing`, `test_matrix_route_change_no_click`, `test_audio_thread_no_heap` 추가분.
5. **재생 시작·API·폭**: 6.6·6.7. → `test_play_track_routes`, `test_processing_channels` 추가분, 전체 `cargo test`, `cargo clippy`(새 경고 0).
6. **Dart 순수 함수와 화면**: 7절. → 8.3, `flutter analyze` 0건, 전체 `flutter test`.
7. **측정·실행·문서**: 8.4 → Release 빌드 → 8.5 → 문서(`docs/HANDOFF.md` 최근 끝난 일·확정된 결정, `docs/WINDOWS_FIELD_CHECKLIST.md`에 8.5의 항목, 운영 안내서에 표 쓰는 법과 롤백 주의 한 줄, 이 문서의 상태 줄) → Mac 확인 요청(8.6).

## 11. 구현 전 다시 확인할 것
③~⑨와 그 밖의 병합으로 아래가 달라졌을 수 있다. 달라졌으면 이 문서의 해당 절을 먼저 고쳐 Main에 알린 뒤 구현한다.
- `mixer.rs` 인스턴스 루프의 경로 두 개 구조(`let routes = [`)와 `ch_limit`·`temp_vals`(6.4·6.5). ⑨가 파일 채널 상한·믹서 폭 규칙을 바꿨는지.
- `SoundInstance`의 칸과 `new` 인자, `build_play_track_command`·`with_start`·`play_track_from_in`의 재생 명령 네 곳(6.6. PR #99가 캐시 경로를 고쳤다).
- `engine.rs` `process_commands`의 `SetTrackOutput` 처리, `SpatialGarbage` 변형과 정리 채널 용량.
- `compute_enabled_channels`·`processing_channel_count`·`request_restart_if_processing_width_changed`(6.7). ③이 엔진 재시작 경로를 바꿨는지.
- 검토 보고 A9가 처리됐는지(공간 게인 계산이 인스턴스 루프 앞에 아직 있는지).
- 더킹 설정 작업(검토 보고 A4, Sub 예정): 믹서의 `current_vol`·더킹 경로가 바뀌었는지. 표 경로는 `current_vol`을 그대로 쓰므로 따라가지만 8.2의 더킹 시험을 그 설정에 맞춘다.
- 미리 올리기 실패 처리(검토 보고 A5, Sub 예정): `play_track_from_in`의 재생 명령을 만드는 자리 수가 바뀌었는지(`with_routes`로 감쌀 곳).
- 워치독 단조 시계(A2): 이 작업과 겹치지 않는지 확인만 한다.
- Dart의 `TrackConfig(` 생성 지점 수(7.5), 트랙 카드·방 카드의 출력 변경 콜백 이름.
- `cargo test`·`flutter test`의 기준 통과 수(구현 전 값을 PR 본문에 적는다).

## 12. 구현자가 정하는 것
- 세부 구현 계획(작업 단위·커밋 나누기)은 구현을 시작할 때 11절 확인 뒤에 쓴다. ③~⑨로 코드가 바뀌므로 지금 쓰지 않는다. 10절이 그 뼈대다.
- 도움말을 펼치는 모양(창 안 패널, 접이식 등). 문구는 7.2 그대로.
- 표 창의 세부 모양(색·간격·아이콘): 기존 대화상자(`export_project_dialog.dart`, `osc_monitor_dialog.dart`)와 `AppColors`를 따른다.
- 믹서 샘플 루프에서 빌림 충돌을 피하는 코드 모양(지역 함수로 뗄지 등). 단, 옛 방식의 식과 순서는 바꾸지 않는다.
- 로그 문구의 세부, 시험의 도우미 함수 구성. "같은 트랙은 `config_version`마다 한 번만" 남기는 기록을 어디에 둘지(오디오 스레드 밖의 정적 표 등).
- Dart에서 새 FRB 함수를 동기로 부를지 비동기로 부를지(기존 비슷한 함수의 방식을 따른다).

## 13. 정한 것(사용자 2026-11-14 "1번 그대로 가")

| # | 내용 | 결정 | 상태 |
|---|---|---|---|
| U1 | 표가 닫힌 출력을 가리킬 때 | 출력 설정이 이긴다(무음, 화면에 주황색). 표 창은 열린 출력만 보인다(D6). Main 동의 | 확정 |
| U2 | 칸 음량의 단위와 범위 | % (1~100), 증폭 없음(D3) | 확정 |
| U3 | 마지막 칸은 끊을 수 없음 | D12 | 확정 |
| U4 | 처리 채널 수가 바뀌는 연결 | 확인 창 뒤 엔진 다시 시작(7.4) | 확정 |
| U5 | 옛 3D Object 분기 삭제 | 이 작업에서는 건드리지 않는다. 지울지는 따로 묻는다(9절) | 확정(삭제 여부는 미정) |
| U6 | 완성 기준 | 14절 | 확정 |

## 14. 완성 기준

### 14.1 작성자 기준
- 1.2의 다섯 가지가 화면에서 되고 8.2·8.3의 시험이 모두 통과한다.
- 전체 `cargo test` 실패 0, `flutter test` 실패 0, `flutter analyze` 0건, `cargo clippy` 새 경고 0, CI(windows·macos) 통과.
- 8.4의 (가)가 구현 전후로 같다.
- 8.5의 1~7번이 이 PC에서 통과한다(장치로 확인, 1·2번은 사람이 들어 확인).
- 8.6을 Mac 세션이 확인한다.
- 10절 7단계의 문서가 갱신됐다.
- 1.4의 네 가지가 하나도 일어나지 않는다.

### 14.2 사용자 기준(2026-11-14)
- 표 창에 `?` 아이콘이 있고, 누르면 기능 설명이 나온다(7.2의 도움말).
- 그 설명만 읽고 사용자가 모노 트랙을 세 출력으로 보낼 수 있다(8.5의 8번, 사용자가 직접 확인).

## 15. 위험

| 위험 | 대비 |
|---|---|
| 표 경로를 더하다 옛 경로의 식을 건드린다 | D10, 8.1의 기존 시험, 8.2 "옛 방식과 같은 소리" |
| 인스턴스가 12KB 커져 효과음을 많이 겹칠 때 메모리가 는다 | 인스턴스는 이미 채널 게인 배열로 32KB를 쓴다(옛 Object 경로). 8.4에서 메모리도 본다 |
| 한 콜백에 명령이 몰려 처리 시간이 는다 | 명령 처리는 복사뿐(최대 512칸 × 인스턴스 수). 화면이 드래그를 초당 30회로 묶는다 |
| 표를 고치다 엔진이 다시 시작한다 | 7.4의 확인 창, 8.5의 3번 |
| Dart와 Rust의 "열린 출력" 규칙이 어긋난다 | 규칙을 Rust 하나에 두고 FRB로 묻는다(6.7) |
| 롤백 때 표가 사라진다 | 5.2, 운영 안내서 한 줄 |
| ③~⑨ 뒤 코드가 달라져 이 문서와 어긋난다 | 11절 |
