# `pan_deg` (DBAP 방위 트림) & `early_ref_mix` (초기반사음) 설계 명세서

- 작성일: 2026-09-09
- 작성자: @Architect (system-architect)
- 조사 방법: `rust/src/audio/mixer.rs`, `rust/src/audio/dsp.rs`, `rust/src/audio/acoustic.rs`, `rust/src/audio/reverb.rs`, `rust/src/common/config.rs`, `rust/src/common/commands.rs`, `rust/src/audio/engine.rs`, `rust/src/api/simple.rs`, `lib/features/exhibition/models/speaker_node.dart`, `lib/features/exhibition/models/room_zone.dart`, `lib/features/exhibition/state/{speaker_layout_state,room_zone_state}.dart`, `lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart` 전수 대조.
- 전제: `docs/02_Planning_and_Specs/task.md`가 진단한 P0/P1 이슈와 무관한 신규 기능 설계이며, 본 문서에서 발견된 기존 결함은 4장에 별도 기재하고 본 작업 범위에서 제외한다.

---

## 0. 핵심 결정 요약

| 항목 | 결정 | 근거 요약 |
|---|---|---|
| `pan_deg` 저장 위치 | `Point3D`에 추가하지 **않음**. `AudioMixer.channel_pan_deg: Vec<f32>` 신설 | `reverb_send`가 이미 `Point3D`/`channel_positions` payload에 키만 실려 있고 실제로는 별도 커맨드(`SetChannelReverbSend`)로 동작하는 선례가 확인됨. `Point3D`는 `Trajectory.waypoints`/`RoomZone.boundary_*`에도 재사용되는 범용 구조체이므로 스피커 전용 필드를 넣지 않는다 |
| `pan_deg` 회전 피벗 | 해당 채널이 바인딩된 `RoomZone`의 수평 중심(`(boundary_min.xy+boundary_max.xy)/2`). 미바인딩 시 무동작(회전 없음) | 물리적 스피커 위치는 불변, DBAP 계산에서만 유효 방위를 튼다는 사용자 결정을 그대로 구현하는 가장 단순한 피벗 |
| `pan_deg` 적용 범위 | `AudioMixer::process()`의 DBAP 가중치/거리 계산(현재 mixer.rs:386-389)에만 적용. `recalculate_spatial_dsp()`의 시간정렬 딜레이·off-axis EQ 계산에는 적용하지 않음 | 사용자 제약 "물리적 스피커는 그대로 두고, DBAP 연산에서만" — 실제 전파 지연/조준각은 물리적 사실이므로 가상 트림의 영향을 받으면 안 됨 |
| `early_ref_mix` 반사 차수 | 1차 반사만 (shoebox 6면: 바닥/천장/±X벽/±Y벽) | 탭 개수 상한을 유한하게 고정해야 하는 실시간 제약 + "단순 읽기만" 요구사항 충족. 2차 이상은 후속 과제로 명시 |
| 흡음계수 소스 | `RoomZone.absorption_coeff`(단일 브로드밴드 값)을 사용. `alphaOctaves`(6밴드)는 이번 구현에서 미사용 | 탭당 밴드 EQ를 두면 오디오 스레드에 필터 상태가 추가되어 "단순 tap 배열 읽기" 요구를 위반. 브로드밴드 계수만으로 delay+gain 튜플 유지 |
| 리스너(수음점) 기준 | `RoomZone.ear_level`(신규 필드, 기본값 1.2, 수평은 zone 중심) | 프로젝트 표준 1.2m을 하드코딩 상수로 중복시키지 않고, 이미 존재하는(Dart에만 있던) `earLevel` 설정을 Rust까지 정합화하여 단일 소스로 사용 |
| 탭 계산 스레드 | **Rust 측, `api_update_spatial_config_json` 핸들러 내부(FRB 비동기 워커 스레드)**. 신규 스레드 생성 없음 | 실제 크래시 스택으로 오디오 스레드임이 확인된 `recalculate_spatial_dsp`/`process_commands`와 달리, `api_update_spatial_config_json`은 `#[frb(sync)]`가 붙지 않은 일반 FRB API라 FRB 런타임의 워커 스레드 풀에서 실행됨(대조군: 같은 파일의 `api_calculate_eq_response_curve`만 `#[frb(sync)]`로 명시돼 있음 — 나머지는 비동기). 이미 이 함수는 `serde_json::from_str`로 힙 할당을 수행 중이므로 오디오 스레드가 아님이 코드로도 방증됨 |
| 탭 전달 메커니즘 | 기존 `AudioCommand::UpdateSpatialConfig` + 기존 `GLOBAL_STATE.command_sender`(crossbeam-channel) 재사용. 오디오 스레드는 `Vec` 포인터 교체만 수행하고 구 버퍼는 기존 `SpatialGarbage` GC 채널로 회수 | 신규 스레드/신규 채널을 만들지 않고 `channel_positions`/`room_zones`가 이미 쓰는 정확히 같은 왕복 경로를 재사용 — Karpathy "단순함 우선" 원칙 |

---

## 1. 기능 ① `pan_deg` — DBAP 방위 오프셋

### 1.1 좌표계 규약

프로젝트 전역 좌표계는 **Z-up**이다: `x`=폭(width), `y`=깊이(depth), `z`=높이(height, 미터). 이는 `channel_positions` payload(`buildChannelPositionsPayload`: `'z': node.heightZ`)와 `mixer.rs`의 룸 바인딩 로직(`pos.x`/`pos.y`로만 AABB 판정, `mixer.rs:359-360`, `946-950`)이 일관되게 따르는 규약이다.

**주의(기존 결함, 재사용 금지):** `rust/src/audio/acoustic.rs`의 `calculate_forward_vector()`는 Y-up 가정(`y = pitch_rad.sin()`)으로 작성되어 있어 위 Z-up 규약과 어긋난다. 이 함수는 `mixer.rs:1032`의 off-axis EQ 계산에서만 쓰이며 그 자체로 기존 버그 소지가 있으나 본 작업 범위 밖이다. `pan_deg` 구현은 이 함수를 참조/재사용하지 않고 X-Y 평면에서 직접 회전을 계산한다.

### 1.2 수식

채널 `ch`가 `RoomZone` Z(바인딩 결과, `mixer.rs:356-364`와 동일 로직 재사용)에 바인딩된 경우:

```
pivot_x = (zone.boundary_min.x + zone.boundary_max.x) / 2
pivot_y = (zone.boundary_min.y + zone.boundary_max.y) / 2
theta   = channel_pan_deg[ch].to_radians()

dx = pos.x - pivot_x
dy = pos.y - pivot_y

eff_x = pivot_x + dx * cos(theta) - dy * sin(theta)
eff_y = pivot_y + dx * sin(theta) + dy * cos(theta)
eff_z = pos.z   // 높이는 변경하지 않음
```

바인딩된 Zone이 없으면 `pan_deg`는 무동작(`eff_x=pos.x, eff_y=pos.y`).

**방향 규약:** 양의 `pan_deg`는 +X축을 +Y축 방향으로 돌리는 표준 수학적 CCW 회전이다. 이는 `lib/features/exhibition/models/room_zone.dart:93-104`의 `RoomZone.containsPoint()`가 이미 사용 중인 회전 행렬과 동일한 부호 규약이므로(그 함수는 역회전을 쓰지만 정회전 공식은 동일 형태), 코드베이스 내에 새 회전 부호 규약을 추가로 발명하지 않는다. `yaw_rotation`(스피커 조준각, `atan2(dx,dz)` 기반 Y-up 규약)과는 축·용도가 다르므로 사용자 요구대로 명확히 구분된다.

**적용 지점:** `mixer.rs:386-389`의 `dx/dy/dz` 계산 직전에 `eff_x`/`eff_y`로 치환. `mixer.rs:356-364`(룸 바인딩)와 `recalculate_spatial_dsp()`(`mixer.rs:888-1102`, 시간정렬/off-axis EQ)는 원본 `pos.x/pos.y`를 그대로 사용 — 무수정.

### 1.3 DBAP 에너지 보존 증명 (Σg²=1 불변)

기존 정규화는 `norm_factor = 1 / sqrt(Σ weight_k²)`이며 최종 패닝비는 `pan_ratio[ch] = weight[ch] * norm_factor`이다(`mixer.rs:397-398`, `410-414`). 이 정규화는 **weight 배열이 무엇으로 계산되었는지와 무관하게 항상**:

```
Σ (weight[ch] * norm_factor)² = Σweight[ch]² / Σweight[ch]² = 1
```

을 만족하도록 구성되어 있다(자기 정규화). `pan_deg` 회전은 `weight[ch]`를 산출하는 거리 `dist`의 입력(`pos.x, pos.y`)을 바꿀 뿐 정규화 공식 자체를 건드리지 않으므로, 회전 각도·피벗과 무관하게 Σg²=1은 항상 자동으로 보존된다. 별도의 보정 로직이 필요 없다.

### 1.4 값 범위 & UI

- Rust 저장 타입: `f32`, 클램프 없음(사인/코사인은 임의 각도에 안전). 방어적으로 `SetChannelPanDeg` 핸들러에서 `-360.0..360.0`으로만 감싸 비정상 입력을 방지 권장.
- UI 슬라이더 범위: **-45.0° ~ +45.0°** 권장(요구사항이 "미세 보정"임을 명시 — 사용자가 "정위감 미세 보정" 용도라 밝혔으므로 Yaw 슬라이더(-180~180)보다 좁게), 단 최종 범위는 front-engineer 재량. 기본값 0.0.

### 1.5 데이터 흐름

```
SpeakerNode.panDeg (Dart, 기존 존재)
  → speaker_layout_state.dart: updateSpeaker()에서
    rust_api.apiSetChannelPanDeg(channel, panDeg) 신규 호출 추가
    (reverbSend 동기화 라인 바로 옆, 동일 패턴)
  → api_set_channel_pan_deg() [rust/src/api/simple.rs, 신규]
    → AudioCommand::SetChannelPanDeg{channel, pan_deg} [common/commands.rs, 신규]
  → engine.rs process_commands() 핸들러 [신규 match arm]
    → mixer.channel_pan_deg[channel] = pan_deg  (단순 대입, 힙 할당 없음)
  → mixer.rs process() DBAP 루프에서 매 버퍼 사용
```

`buildChannelPositionsPayload()`에 이미 있는 `'pan_deg'` 키는 죽은 키가 되므로 **제거 권장**(4장 참고, 필수는 아님).

---

## 2. 기능 ② `early_ref_mix` — Image-Source 초기반사음

### 2.1 모델

Shoebox(직육면체) 룸 가정 하에 **1차 반사만** 계산한다. `RoomZone.boundary_min/boundary_max`(`Point3D`, 미터)가 정의하는 AABB의 6개 평면(바닥 `z=min.z`, 천장 `z=max.z`, 벽 `x=min.x`, `x=max.x`, `y=min.y`, `y=max.y`) 각각에 대해 스피커 위치를 거울반사(image source)시켜 리스너까지의 왕복 경로를 구한다.

리스너(수음점) 좌표:
```
listener = (
  (zone.boundary_min.x + zone.boundary_max.x) / 2,
  (zone.boundary_min.y + zone.boundary_max.y) / 2,
  zone.ear_level   // 신규 필드, 기본 1.2 — 프로젝트 표준 [W/2, D/2, 1.2m]과 정합
)
```

평면 `P`(예: `x = boundary_max.x`)에 대한 이미지 소스는 스피커 좌표 중 해당 축 성분만 평면에 대해 대칭시킨 점이다. 6평면 각각에 대해:

```
path_len    = |image_source - listener|        // 미터
direct_len  = |speaker - listener|             // 미터
extra_len   = max(path_len - direct_len, 0)    // 직접음보다 더 지나는 거리
delay_ms    = calculate_acoustic_delay_ms(extra_len)   // 340 m/s
reflect_r   = sqrt(1.0 - zone.absorption_coeff.clamp(0.0, 0.99))   // 압력 반사계수
near_fade   = clamp((extra_len - 0.5) / 1.0, 0, 1)     // 스피커에 바짝 붙은 면은 제외
gain        = clamp(reflect_r * direct_len / path_len * near_fade, 0, 1)
```

> **2026-09-26 개정 (구현과 일치).** 처음 명세는 1m 기준 절대값(`delay = path/c`,
> `gain = reflect_r / path`)이었다. 그 식은 직접음도 거리만큼 감쇠한다는 가정에서 나왔는데,
> 실제 엔진에서 스피커→청취자 직접음에는 거리 감쇠가 없다(`mixer.rs`의 `1/max(dist,1)`은
> 가상 음원↔가장 가까운 스피커 거리에 쓰인다). 그래서 큰 방에서 반사음이 직접음보다
> 20dB 이상 작고, 지연도 직접음 경로 시간만큼(수십 ms) 길었다 — 사실상 들리지 않았다.
>
> 합성 반사음도 스피커에서 나와 청취자까지 같은 거리를 다시 지나므로, **직접음 대비
> 상대값**이 맞다. 또한 스피커를 벽·천장에 바짝 붙여 다는 현장에서는 그 면의 반사가
> 1ms도 차이 없이 거의 같은 크기로 겹쳐 콤필터를 만들고, image-source 모델은 스피커
> 방향성과 면의 크기를 무시해 이를 과장한다. 그 근접면 효과는 자동 EQ(경계면 저음/바닥
> 반사 딥)가 따로 다루므로 여기서는 경로 차 0.5~1.5m 구간에서 서서히 제외한다.
>
> **2026-09-27 추가 — 반사음 고역 감쇠.** 위 식만으로는 반사음이 직접음의 고역을 그대로
> 되돌려줘서, 직접음과 겹치며 4~8kHz에 +5~7dB 콤 피크를 만들었다(실기 보고: "바이노럴
> 켜면 고역이 세게 들어온다"). 실제 반사음이 고역을 잃는 세 경로를 탭마다 반영한다
> (2kHz 하이셸프, 총합 상한 -18dB):
> - **스피커 방향성** — 반사음이 출발하는 방향이 분산각 밖이면 1도당 0.2dB(상한 18dB).
>   Dart 오프액시스 자동 EQ와 같은 규약(position_eq.dart). 수평·수직을 따로 재서 더한다.
> - **재질의 고역 흡음** — 브로드밴드 흡음률 x 2.5를 4kHz 흡음률로 본다.
> - **표면 산란(-4dB)과 추가 경로의 공기 흡음(-0.03dB/m)** — 실제 면은 완전한 거울이
>   아니라 고역을 흩뿌린다. 그 성분은 후기 잔향이 담당한다.
>
> 셸프 지점을 직접음(4kHz)보다 낮은 2kHz로 둔 이유: 실제 반사음은 1~2kHz부터 둔해지고,
> 셸프는 꺾이는 지점에서 게인의 절반만 걸리기 때문이다. 연출용 정규화
> (`early_reflection_effect_scale`)도 이 감쇠를 반영해야 100%가 -6dB로 유지된다.
>
> 측정(사용자 실제 레이아웃, 천장 밑 14.75m 스피커): 4kHz 콤 피크 +5.3dB -> **+1.1dB**,
> 8kHz 콤 골 -4.5dB -> -2.4dB.
>
> **적용 방식도 둘로 나뉜다** (`mixer.rs::refresh_early_ref_mix`):
> - **방 시뮬레이션** — 바이노럴(헤드폰 미리듣기)을 켤 때만 위 탭을 물리 그대로(믹스 1.0)
>   적용한다. 현장에서는 실제 벽이 같은 반사를 만들기 때문에 겹치면 반사가 두 번 들어간다.
> - **연출용 ER Mix(스피커별)** — 항상 적용하되, 전체 반사 에너지를 직접음 대비 -6dB
>   (`EARLY_REFLECTION_EFFECT_TARGET_DB`)로 정규화해 방이 달라도 슬라이더 감각이 같게 한다.
>   새 스피커 기본값은 0%다.

스피커가 어떤 `RoomZone`에도 바인딩되지 않으면 해당 채널의 6개 탭은 모두 `gain=0`(무음), 폐기하지 않고 슬롯만 0으로 채운다(고정 6슬롯 유지, 4.4절 참고).

### 2.2 탭 개수 상한 & 버퍼 크기

- 채널당 고정 **6슬롯**(가변 길이 금지 — 오디오 스레드가 항상 동일한 인덱싱으로 읽도록).
- 전체 상한: `channels * 6` (예: 24채널 → 144 tap).
- 탭 지연을 담을 채널별 링버퍼: **`EARLY_REFLECTION_BUFFER_SIZE = 9600` 샘플(48kHz 기준 200ms)** 신설. `dsp.rs`의 기존 `DELAY_BUFFER_SIZE=48000`(1초, 메인 시간정렬용) 관례를 그대로 따르되 용도에 맞게 축소. `compute_early_reflection_taps()`는 `delay_ms`가 200ms를 초과하면 방어적으로 클램프(비정상적으로 거대한 방 설정을 흡수, 링버퍼 랩어라운드 방지).

### 2.3 RoomZone 신규 필드 (Rust ↔ Dart 정합)

`rust/src/common/config.rs`의 `RoomZone`에 추가:

```rust
#[serde(default)]
pub absorption_coeff: f32,
#[serde(default = "default_ear_level")]
pub ear_level: f32,   // fn default_ear_level() -> f32 { 1.2 }
```

**중요 — 이미 존재하던 데이터 배관 결함(필수 수정, 옵션 아님):**
`lib/features/exhibition/state/room_zone_state.dart:96-113`의 `_notifyBackend()`가 만드는 `room_zones` payload는 이미 `'absorption_coeff': r.absorptionCoeff`를 보내고 있으나 Rust `RoomZone`에 대응 필드가 없어 지금까지 조용히 버려지고 있었다(사용자가 보고한 `pan_deg`/`early_ref_mix`와 동일한 버그 클래스). 위 필드 추가로 이 값이 비로소 살아난다.
또한 같은 함수의 `boundary_max.z`가 `r.ceilingHeight`를 쓰지 않고 **`2.0`으로 하드코딩**되어 있다 — 천장 반사 탭이 항상 잘못된 높이(2m)로 계산되므로 **`'z': r.ceilingHeight`로 고치는 것이 이번 기능의 필수 선행 작업**이다(front-engineer 태스크 F-2에 포함, 5장 참고).
`'ear_level': r.earLevel` 키도 이 함수에 신규 추가해야 한다(현재 아예 전송되지 않음).

### 2.4 스레드 배치 — 상세 근거

**금지된 경로:** `recalculate_spatial_dsp()`(`mixer.rs:888`)와 `process_commands()`(`engine.rs:530`)는 이번 세션의 실제 크래시 스택에서 `set_render_callback` ← `process_commands` ← `recalculate_spatial_dsp` 호출관계로 **CoreAudio 렌더 콜백(오디오 스레드) 내부에서 실행됨이 확인**되었다. 따라서 이 두 함수 내부에서 image-source 탭 계산(제곱근, 삼각함수 다수, 배열 순회)을 새로 수행하는 것은 금지된다(Law 2 위반 소지는 크지 않지만 Law 1 — 임시 `Vec` 없이 짜더라도 매 버퍼 반복 계산은 낭비이며, 무엇보다 사용자가 명시적으로 "비-오디오 스레드"를 요구함).

**채택된 경로:** `rust/src/api/simple.rs::api_update_spatial_config_json()`(`simple.rs:1622`)는 `#[flutter_rust_bridge::frb(sync)]` 어노테이션이 **없는** 일반 FRB API 함수다. 같은 파일의 `api_calculate_eq_response_curve()`(`simple.rs:1695`)만 `#[frb(sync)]`로 명시되어 있는 것과 대조하면, 어노테이션이 없는 함수들은 flutter_rust_bridge 런타임이 자체 관리하는 워커 스레드 풀에서 비동기 실행됨을 알 수 있다. 실제로 `api_update_spatial_config_json`은 이미 `serde_json::from_str`로 힙 할당을 수행하고 있어(오디오 스레드였다면 Law 1 위반), 이 함수가 오디오 스레드가 아님은 기존 코드로도 방증된다. 추가로 `api_calculate_dbap_heatmap()`(`simple.rs:1671`)이 이미 `Vec::with_capacity` 등 힙 할당을 동반한 공간음향 계산을 FFI로 노출하는 선례이므로, "비실시간 컨텍스트에서 공간음향 수학을 수행해 FFI로 노출"하는 패턴은 코드베이스에 이미 확립되어 있다.

**결정:** 새 전용 워커 스레드를 만들지 않는다. `api_update_spatial_config_json()` 내부(JSON 파싱 직후, 커맨드 전송 직전)에서 `payload.channel_positions`와 `payload.room_zones`를 순회하며 `compute_early_reflection_taps()`(신규, `audio/acoustic.rs`, 순수 함수)를 호출해 `Vec<[EarlyReflectionTap; 6]>`(길이 = 채널 수)를 만들고, 이를 **기존** `AudioCommand::UpdateSpatialConfig`에 신규 필드로 실어 **기존** `GLOBAL_STATE.command_sender`(crossbeam-channel, 이미 lock-free MPSC)로 보낸다. 신규 채널·신규 스레드·`AtomicPtr`을 새로 도입하지 않고, `channel_positions`/`room_zones`가 이미 왕복하는 것과 완전히 동일한 경로를 재사용한다 — Karpathy 단순함 우선 원칙에 가장 부합하는 최소 변경.

오디오 스레드(`engine.rs`의 `UpdateSpatialConfig` 핸들러)는 다음만 수행한다(Law 1 준수 — 포인터 이동만, 신규 할당 없음):

```rust
let old_taps = std::mem::replace(&mut mixer.channel_early_ref_taps, early_reflection_taps);
let _ = mixer.spatial_gc_tx.try_send(SpatialGarbage::EarlyReflectionTaps(old_taps));
```

이는 현재 `channel_positions`/`room_zones`/`track_positions`에 이미 적용 중인 처리와 완전히 동일한 패턴이다(`SpatialGarbage` enum, `mixer.rs:6-13`).

### 2.5 데이터 구조

`rust/src/audio/acoustic.rs`에 신규:

```rust
pub const MAX_EARLY_REFLECTION_TAPS: usize = 6;

#[derive(Debug, Clone, Copy, Default)]
pub struct EarlyReflectionTap {
    pub delay_ms: f32,  // 샘플 변환은 소비 시점(dsp.rs process())에서 fs로 수행 — 기존 delay_buffer 패턴과 동일
    pub gain: f32,
}

pub fn compute_early_reflection_taps(
    speaker_pos: &crate::common::config::Point3D,
    zone: &crate::common::config::RoomZone,
) -> [EarlyReflectionTap; MAX_EARLY_REFLECTION_TAPS] { ... }
```

`delay_ms`(샘플이 아닌 ms)로 저장하는 이유: 계산 시점의 샘플레이트와 실제 재생 샘플레이트가 어긋날 위험을 없애기 위해, 기존 `ChannelDspState.target_delay_ms/current_delay_ms`(`dsp.rs:138-139, 251`)와 동일하게 ms 단위로 저장하고 `process(input, fs)` 호출 시점의 `fs`로 샘플 변환한다.

### 2.6 DSP 스레드 소비 (읽기 전용)

`rust/src/audio/dsp.rs`의 `ChannelDspState`에 추가:

```rust
pub early_ref_buffer: Vec<f32>,          // vec![0.0; EARLY_REFLECTION_BUFFER_SIZE]
pub early_ref_write_idx: usize,
pub taps: [EarlyReflectionTapState; MAX_EARLY_REFLECTION_TAPS],
pub target_early_ref_mix: f32,
pub current_early_ref_mix: f32,

pub struct EarlyReflectionTapState {
    pub target_delay_ms: f32,
    pub current_delay_ms: f32,
    pub target_gain: f32,
    pub current_gain: f32,
}
```

`process()` 내부, phase-invert 처리 직후·기존 `self.reverb.process_mono(out)` 호출 직전(`dsp.rs:291-298` 사이)에 삽입:

```rust
self.early_ref_buffer[self.early_ref_write_idx] = out;
let mut er_sum = 0.0;
for tap in self.taps.iter_mut() {
    // Law 3: target/current 1-pole 스무딩. 기존 current_delay_ms 스무딩과 동일 계수(0.005) 재사용
    if (tap.target_delay_ms - tap.current_delay_ms).abs() > 0.001 {
        tap.current_delay_ms += (tap.target_delay_ms - tap.current_delay_ms) * 0.005;
    } else { tap.current_delay_ms = tap.target_delay_ms; }
    if (tap.target_gain - tap.current_gain).abs() > 0.0001 {
        tap.current_gain += (tap.target_gain - tap.current_gain) * 0.005;
    } else { tap.current_gain = tap.target_gain; }

    let delay_samples = (tap.current_delay_ms / 1000.0 * fs)
        .clamp(0.0, (EARLY_REFLECTION_BUFFER_SIZE - 1) as f32) as usize;
    let read_idx = (self.early_ref_write_idx + EARLY_REFLECTION_BUFFER_SIZE - delay_samples)
        % EARLY_REFLECTION_BUFFER_SIZE;
    er_sum += self.early_ref_buffer[read_idx] * tap.current_gain;
}
self.early_ref_write_idx = (self.early_ref_write_idx + 1) % EARLY_REFLECTION_BUFFER_SIZE;

if (self.current_early_ref_mix - self.target_early_ref_mix).abs() > 0.0001 {
    self.current_early_ref_mix += (self.target_early_ref_mix - self.current_early_ref_mix) * 0.005;
} else { self.current_early_ref_mix = self.target_early_ref_mix; }

out += er_sum * self.current_early_ref_mix;
```

v1은 정수 샘플 인덱싱만 사용(hermite 보간 없음) — 초기반사음은 연속 도플러 이동체와 달리 지연이 이산적(형상 변경 시에만)으로만 바뀌므로 서브샘플 보간 없이도 Law 3(스무딩)만으로 clip/zipper를 회피 가능. 후속 과제로 보간 추가는 선택.

### 2.7 Late Reverb와의 믹스 규칙

**병렬(parallel) 합산 → 그 결과가 후단 late-tail reverb에 직렬(serial)로 입력.** 즉 `out += er_sum * current_early_ref_mix`로 dry 신호에 더한 뒤, 바로 다음 줄의 기존 `if self.reverb.is_enabled && self.reverb.mix > 0.0 { out = self.reverb.process_mono(out); }`가 (dry+early) 신호를 입력받는다. 이는 실제 음향학적으로도 타당하다(초기반사음이 후기 확산 잔향을 가진다). 정규화 규칙은 별도로 두지 않는다 — `early_ref_mix` 자체가 0.0~1.0 스케일의 게인 곱셈이므로 총합이 과도해지면 기존 `master_headroom_db`/피크 리미터가 흡수한다(신규 정규화 로직 불필요, 기존 게인 스테이징 인프라 재사용).

### 2.8 커맨드/전달 계약 요약

```rust
// common/commands.rs — UpdateSpatialConfig 확장
UpdateSpatialConfig {
    channel_positions: Vec<Option<Point3D>>,
    room_zones: Vec<RoomZone>,
    trajectory: Option<Trajectory>,
    track_positions: HashMap<String, Point3D>,
    early_reflection_taps: Vec<[crate::audio::acoustic::EarlyReflectionTap; 6]>,  // 신규, len == channel_positions.len()
},

// 신규 스칼라 커맨드 (pan_deg / early_ref_mix 각각 즉시 반영용, reverb_send와 동일 패턴)
SetChannelPanDeg { channel: usize, pan_deg: f32 },
SetChannelEarlyRefMix { channel: usize, mix: f32 },
```

```rust
// mixer.rs — SpatialGarbage 확장
pub enum SpatialGarbage {
    ...,
    EarlyReflectionTaps(Vec<[crate::audio::acoustic::EarlyReflectionTap; 6]>),
}
```

---

## 3. `pan_deg` × `early_ref_mix` 상호작용

두 기능은 독립적이다. `pan_deg`는 DBAP 패닝 가중치만, `early_ref_mix`는 채널별 초기반사 tap 합산만 건드리며 서로 다른 코드 경로(전자는 `mixer.rs::process()`, 후자는 `dsp.rs::ChannelDspState::process()`)에 위치해 데이터 의존성이 없다. 단, `early_ref_mix`의 tap 계산(`compute_early_reflection_taps`)은 **회전되지 않은(raw) 물리적 스피커 위치**를 입력으로 받아야 한다 — `pan_deg`로 트림된 가상 위치를 쓰면 안 된다(반사음은 실제 스피커가 실제로 방출하는 물리 현상이므로).

---

## 4. 사전 발견된 기존 결함 (참고용 — 본 작업 필수 범위 아님)

| # | 위치 | 증상 | 본 작업과의 관계 |
|---|---|---|---|
| D1 | `room_zone_state.dart:107` | `boundary_max.z`가 `r.ceilingHeight` 대신 `2.0` 하드코딩 | **필수 수정** — 천장 반사 탭 계산의 전제 조건이므로 태스크 F-2에 포함 |
| D2 | `room_zone_state.dart:111` vs `config.rs` `RoomZone.transmission_loss_db` | payload 키 `'transmission_loss'`와 Rust 필드명 `transmission_loss_db`가 달라 serde가 조용히 버림 | 본 작업과 무관. 고치려면 `'transmission_loss_db'`로 키 1줄만 바꾸면 되므로, F-2 작업 중 같은 함수를 열어보는 김에 **선택적으로** 함께 고칠 것을 권장하되 필수 아님 |
| D3 | `dsp.rs::ChannelDspState::process()` | `target_reverb_send`/`current_reverb_send` 필드가 설정만 되고 `process()`에서 전혀 읽히지 않음(리버브는 `self.reverb.mix`로만 동작) → UI의 Reverb Send 슬라이더가 사실상 무동작 | 본 작업과 무관. `early_ref_mix`를 `target_reverb_send`와 같은 방식(죽은 필드)으로 구현하지 않도록 2.6절에서 실제 `process()` 삽입을 명시했음 |
| D4 | `acoustic.rs::calculate_forward_vector()` | Y-up 가정이 나머지 코드의 Z-up 규약과 불일치, off-axis EQ 각도 계산 오차 가능성 | 본 작업과 무관. `pan_deg`가 이 함수를 재사용하지 않도록 1.1절에서 명시적으로 배제 |

---

## 5. 원자 단위 태스크 분할

에이전트 간 **동일 파일 동시 편집 금지** 원칙에 따라 아래 순서를 지킨다. `[Back]`/`[Front]` 태그, 완료 기준은 기계적으로 검증 가능한 테스트로 명시한다.

### Phase 1 — Rust 데이터 모델 & 커맨드 (Back 단독, 이 Phase 끝나야 Phase 2 시작)

- **B-1** `rust/src/common/config.rs`
  - `RoomZone`에 `absorption_coeff: f32`(`#[serde(default)]`), `ear_level: f32`(`#[serde(default = "default_ear_level")]`, 1.2) 추가.
  - 완료 기준: `cargo check` 통과. 기존 `RoomZone` 관련 테스트(있다면) 회귀 없음. `serde_json::from_str::<RoomZone>("{}")`가 `absorption_coeff=0.0, ear_level=1.2`로 파싱되는 단위 테스트 신규 추가.

- **B-2** `rust/src/audio/acoustic.rs`
  - `EarlyReflectionTap`, `MAX_EARLY_REFLECTION_TAPS`, `compute_early_reflection_taps()` 추가.
  - 완료 기준(6장 검증 계획의 손계산 테스트 포함): `cargo test acoustic::` 신규 테스트 전부 통과, 힙 할당 없이 `[EarlyReflectionTap; 6]` 고정 배열 반환(시그니처로 강제됨).

- **B-3** `rust/src/common/commands.rs`
  - `UpdateSpatialConfig`에 `early_reflection_taps` 필드 추가. `SetChannelPanDeg`, `SetChannelEarlyRefMix` variant 추가.
  - 완료 기준: `cargo check` 통과(이 시점에서 `engine.rs`/`simple.rs`는 아직 미수정이므로 컴파일 에러 발생이 정상 — B-4/B-5에서 해소).

의존관계: B-1 → B-2(같은 파일 아님, 순차 필요 없음, 사실 병렬 가능하나 리뷰 단순화를 위해 순서대로), B-3은 B-1 완료 후(타입 참조 필요).

### Phase 2 — Rust 배선 (Back 단독)

- **B-4** `rust/src/audio/dsp.rs`
  - `ChannelDspState`에 2.6절 필드/메서드 추가, `process()`에 tap 합산 삽입.
  - `EARLY_REFLECTION_BUFFER_SIZE` 상수 추가.
  - 완료 기준: `cargo test`의 `test_limiter_process`/`test_svf` 등 기존 바이너리 회귀 없음. 신규 단위 테스트: 1kHz 사인파 입력 + 단일 tap(delay_ms=10, gain=0.5) 설정 시 출력에 정확히 `10ms`(±1 샘플) 지연된 진폭 `0.5`의 사본이 합산되는지 버퍼 오프라인 처리로 검증. `early_ref_mix=0`일 때 출력이 tap 없는 경우와 완전히 동일(디지털 무음 diff)한지 검증(무음 주입 테스트, `DSP_RULES.md`의 3대 법칙 감사용).

- **B-5** `rust/src/audio/mixer.rs`
  - `SpatialGarbage::EarlyReflectionTaps` variant, `AudioMixer.channel_pan_deg`/`channel_early_ref_taps` 필드, `new()` 초기화.
  - `process()`의 DBAP 루프에 1.2절 회전식 삽입(정확히 `dx/dy` 계산 직전).
  - 완료 기준: `cargo test`로 `pan_deg=0`일 때 기존 DBAP 가중치 출력과 100% 동일(회귀 스냅샷), `pan_deg≠0`일 때 Σ(pan_ratio²)=1이 부동소수 오차(`1e-5`) 이내로 유지되는지 검증(1.3절 증명의 실측).

- **B-6** `rust/src/audio/engine.rs`
  - `UpdateSpatialConfig` 핸들러에 `early_reflection_taps` 이관 + `spatial_gc_tx` 전송 로직 추가. `SetChannelPanDeg`/`SetChannelEarlyRefMix` match arm 추가.
  - 완료 기준: `cargo check`/`cargo clippy` 0 warning. 오디오 스레드 측 코드에 `Vec::new`/`vec![]`/`.clone()`이 새로 추가되지 않았음을 `grep`으로 확인(코드 리뷰 체크리스트, `code-reviewer` 담당).

- **B-7** `rust/src/api/simple.rs`
  - `api_update_spatial_config_json()` 내부에 tap 계산 루프 삽입(2.4절). `api_set_channel_pan_deg()`, `api_set_channel_early_ref_mix()` 신규 함수 추가.
  - 완료 기준: `cargo check` 전체 통과. 이후 `flutter_rust_bridge_codegen generate` 실행하여 `frb_generated.rs`/Dart 바인딩 재생성, `git diff`로 신규 함수 3개(`apiUpdateSpatialConfigJson` 시그니처 불변 확인, `apiSetChannelPanDeg`, `apiSetChannelEarlyRefMix` 신규)만 추가됐는지 확인.

의존관계: B-4·B-5는 서로 다른 파일(병행 가능하나 한 사람이 순차 진행 권장) → B-6(두 파일의 상태를 모두 참조하므로 B-4·B-5 완료 후) → B-7(커맨드 전송이므로 B-3·B-6 이후, 그리고 FFI 재생성이 마지막).

### Phase 3 — Flutter 배선 (Front 단독, Phase 2의 B-7 FFI 재생성 완료 후 시작)

- **F-1** `lib/features/exhibition/state/room_zone_state.dart`
  - `_notifyBackend()`의 `room_zones` payload에 `'ear_level': r.earLevel` 추가.
  - `boundary_max.z`를 하드코딩 `2.0` → `r.ceilingHeight`로 수정(D1, 필수).
  - (선택) D2 수정 시 `'transmission_loss'` → `'transmission_loss_db'`.
  - 완료 기준: `flutter analyze` 0 issue. 위젯/상태 테스트로 `RoomZone(ceilingHeight: 4.0, earLevel: 1.5, absorptionCoeff: 0.2)`를 만들었을 때 생성되는 JSON payload에 `boundary_max.z==4.0`, `ear_level==1.5`, `absorption_coeff==0.2`가 정확히 포함되는지 검증(순수 함수 단위 테스트, 실제 FFI 호출 없이 payload 딕셔너리만 검증).

- **F-2** `lib/features/exhibition/state/speaker_layout_state.dart`
  - `updateSpeaker()`에 `apiSetChannelPanDeg(channel, panDeg)`, `apiSetChannelEarlyRefMix(channel, earlyRefMix)` 호출 추가(기존 `apiSetChannelReverbSend` 라인 바로 옆, 동일한 `chIdx`/try-catch 패턴).
  - 완료 기준: `flutter analyze` 0 issue. 기존 `reverbSend` 동기화 테스트가 있다면 동일 패턴으로 `panDeg`/`earlyRefMix`용 테스트 추가(mock FFI 호출 인자 검증).

- **F-3** `lib/features/exhibition/models/speaker_node.dart`
  - `buildChannelPositionsPayload()`의 죽은 키 `'pan_deg'`, `'early_ref_mix'`, `'reverb_send'` 제거(선택, 4장 D 항목과 별개의 정리 — 기능 동작에는 영향 없으나 혼동 방지). **필수 아님, 시간 되면 수행.**
  - 완료 기준: 제거 후에도 `flutter analyze` 0 issue, 기존 payload 스냅샷 테스트(있다면) 업데이트.

- **F-4** `lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart`
  - Speaker Inspector 아코디언에 `_buildControlBox(..., 'Pan Trim', speaker.panDeg, '°', -45.0, 45.0, ... (v) => _updateSpeaker(speaker, pan: v))` 슬라이더 추가(기존 Yaw/Pitch/Dispersion 슬라이더 바로 아래, 302-304줄 패턴 재사용).
  - `_buildReverbSendCard`와 유사한 형태로 Early Reflection Mix 슬라이더/카드 추가(0.0~1.0, 기본 0.2).
  - `_updateSpeaker()` 시그니처에 이미 `pan`/`earlyRefMix` 파라미터가 존재하는지 확인 후(사용자 조사에 따르면 `pan:`은 정의는 있으나 호출부 0건) 없으면 추가.
  - 완료 기준: `flutter analyze` 0 issue. 위젯 테스트로 슬라이더 드래그 시 `speakerLayoutProvider` 상태의 `panDeg`/`earlyRefMix`가 갱신되는지 검증.

의존관계: F-1은 F-2/F-4와 다른 파일이라 병행 가능하나, F-4는 `_updateSpeaker()`가 최종적으로 F-2의 FFI 호출을 트리거하므로 F-2 이후 권장. F-3은 아무 때나(독립).

### Phase 4 — 통합 검증 (test-runner → code-reviewer)

- **T-1** `cargo test -- --nocapture` 전체 통과, 신규 테스트(B-2, B-4, B-5) 포함.
- **T-2** `cargo clippy` 0 warning.
- **T-3** `flutter analyze` 0 issue, `flutter test` 전체 통과.
- **T-4** 1kHz 사인파/디지털 무음 주입: `early_ref_mix=0` 및 `pan_deg=0`일 때 두 기능 도입 전/후 출력 바이너리 diff가 0(회귀 없음)임을 오프라인 렌더 비교로 확인.
- **T-5** `code-reviewer`가 `grep -n "Vec::new\|vec!\|\.clone()\|Box::new" rust/src/audio/engine.rs rust/src/audio/mixer.rs rust/src/audio/dsp.rs`로 오디오 스레드 경로(특히 이번에 수정된 `process()`/`process_commands` 인근)에 신규 힙 할당이 추가되지 않았는지 최종 감사.

---

## 6. 검증 계획 — 기계적 판정 (1차 반사 도달시간 손계산 대조)

`cargo test`에 아래 픽스처를 고정 추가한다(B-2 완료 기준의 일부):

```
가상 방: boundary_min = (0,0,0), boundary_max = (6,4,3)  // 폭 6m × 깊이 4m × 높이 3m
스피커: (1, 1, 2.5)
ear_level = 1.2, 룸 중심 (3, 2) → 리스너 = (3, 2, 1.2)
absorption_coeff = 0.2 → reflect_r = sqrt(0.8) ≈ 0.8944
```

바닥(z=0) 반사 손계산:
```
이미지 소스 = (1, 1, -2.5)
path_len = sqrt((3-1)² + (2-1)² + (1.2-(-2.5))²) = sqrt(4+1+13.69) = sqrt(18.69) ≈ 4.3234 m
delay_ms = 4.3234 / 340 * 1000 ≈ 12.716 ms
gain = 0.8944 / 4.3234 ≈ 0.2069
```

테스트는 `compute_early_reflection_taps()`가 반환한 바닥 탭(6개 중 z=min 평면에 해당하는 인덱스)의 `delay_ms`/`gain`이 위 손계산값과 **±0.01 이내**로 일치하는지 단정한다. 동일한 방식으로 천장(z=3)·4개 벽면도 각각 손계산해 6탭 전부를 개별 단정하여, 오탐(다른 평면끼리 값이 우연히 맞아떨어지는 경우)을 배제한다.

추가로 **직접음 대비 상대성 검증**: 직접음 거리 `sqrt((3-1)²+(2-1)²+(1.2-2.5)²)=sqrt(4+1+1.69)=sqrt(6.69)≈2.586m`보다 모든 반사 경로(≥4.32m)가 항상 길고, 모든 반사 게인이 직접음 게인(`1/2.586≈0.3867`)보다 항상 작음을 단정하는 물리적 타당성 테스트를 추가한다(반사음이 직접음보다 늦고 작아야 한다는 불변식).

---

## 7. Out of Scope / 후속 과제

- 2차 이상 반사(image-source recursion), 벽면별 6밴드(`alpha_octaves`) 주파수 종속 흡음.
- 탭 delay 서브샘플(hermite) 보간.
- D2(`transmission_loss` 키 불일치), D3(`reverb_send` 죽은 필드), D4(`calculate_forward_vector` Y-up 불일치) — 각각 독립 버그 티켓으로 별도 처리 권장.
