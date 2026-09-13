# 바이노럴(HRTF) 공간 렌더링 재설계 명세서

- 작성일: 2026-09-13
- 작성자: Claude (Sonnet 5), 사용자 검토 요청에 따른 코드 추적 기반 작성
- 조사 방법: `rust/src/audio/binaural.rs`, `rust/src/audio/mixer.rs`(pan_deg 피벗 회전, 룸 바인딩), `rust/src/audio/acoustic.rs`(리스너 기준점 관례), `rust/src/osc/listener.rs`(`/hrtf/tracking`) 전수 대조.
- 전제: `PAN_DEG_AND_EARLY_REFLECTIONS_SPEC.md`가 확립한 좌표계(Z-up)·리스너 기준점(RoomZone 중심 + ear_level)·피벗 회전 공식을 그대로 재사용한다. 새 규약을 발명하지 않는다.

---

## 0. 지금 뭐가 문제인가 (코드로 확인한 사실)

`VirtualMixRoomBinaural::process_interleaved()`(`binaural.rs:394`)를 추적한 결과:

1. 이 함수가 유일하게 읽는 위치 정보는 `GLOBAL_STATE.hrtf_yaw/pitch/roll`뿐이다. 이 값은 `rust/src/osc/listener.rs:201`의 `/hrtf/tracking` 엔드포인트로만 갱신되며, **외부 헤드 트래커 장치가 보내는 리스너의 머리 방향**이다.
2. 이 머리 방향 하나로 HRIR 한 쌍을 계산해(`interp_ir_left`/`interp_ir_right`, `binaural.rs:428`), **믹스에 있는 모든 물리 출력 채널에 동일하게 적용**한다(`binaural.rs:442`, `for ch in &mut self.channels { ch.update_hrtf(...) }`).
3. `mixer.rs`, `dsp.rs`, `core/state.rs` 전체를 grep했으나 채널의 실제 스피커 위치(`channel_positions`)나 오브젝트 트랙 위치가 바이노럴 계산에 들어가는 경로는 **하나도 없다**.

결과: 헤드폰으로 들으면 머리를 돌릴 때 믹스 전체의 색깔이 똑같이 바뀌긴 하지만, "1번 채널은 앞, 6번 채널은 뒤"처럼 스피커별로 다른 방향에서 들리는 공간감 자체가 재현되지 않는다. 궤적 오토메이션이 완성되어 소리가 실제로 움직여도, 이 단계가 그 움직임을 반영할 방법이 없다.

이건 P1-6/7(더미 HRTF)이 이미 해결한 것과는 다른 층위의 문제다. SOFA 실측 데이터를 쓰는 것(P1-6/7)과 그 데이터를 "어느 방향"에 적용할지를 아는 것(이 문서)은 별개다. 전자는 됐고 후자가 안 됐다.

---

## 1. 핵심 결정 요약

| 항목 | 결정 | 근거 |
|---|---|---|
| 렌더링 단위 | **오브젝트별이 아니라 물리 출력 채널별.** 각 채널을 "그 위치에 실제 스피커가 있다"고 보고 독립적으로 HRTF 렌더링한 뒤 합산한다 | `process_interleaved`는 이미 DBAP로 믹스가 끝난 *이후*의 물리 채널 신호에서 동작한다(오브젝트 신호도 이미 여러 채널에 분산되어 있음). 이 시점에서 오브젝트/베드를 다시 분리할 수 없고, 분리할 필요도 없다 — "룸에 진짜 스피커가 있다면 이렇게 들린다"를 재현하는 것이 목표이므로, 이미 실제 설치 위치에 맞게 분배된 채널 신호를 그대로 그 위치에서 렌더링하는 것이 정확하다 |
| 채널 기준 방위각 소스 | `channel_positions[ch]`(하드웨어 채널의 실제 3D 위치, `mixer.rs`가 DBAP에 쓰는 것과 동일 데이터) | 이미 있는 데이터를 재사용한다. 신규 좌표계·신규 배관 불필요 |
| 리스너(수음점) 기준 | 채널이 바인딩된 `RoomZone`의 수평 중심 + `ear_level` | `PAN_DEG_AND_EARLY_REFLECTIONS_SPEC.md` §2.1이 이미 확립한 관례와 동일. 초기반사음·pan_deg가 쓰는 것과 같은 기준점을 쓰지 않으면 "왜 여기는 다른 중심을 쓰냐"는 정합성 문제가 생긴다 |
| 채널 미바인딩 시 | 정면(0°) 취급, 렌더링에서 제외하지 않음 | 무음/제외보다 "일단 앞에서 나는 것으로 취급"이 안전하다(초기반사음의 "바인딩 안 되면 게인 0" 관례와는 다름 — 이유는 §2.3 참고) |
| 좌표계 | Z-up, X=폭, Y=깊이(프로젝트 전역 규약) | `PAN_DEG...SPEC.md` §1.1과 동일. `acoustic.rs::calculate_forward_vector()`(Y-up 가정, 기존 결함)는 재사용 금지 — 이미 그 문서가 내린 결정을 그대로 따른다 |
| 방위각 "정면(0°)" 방향 정의 | **신규 결정**: RoomZone 좌표계에서 +Y 방향을 0°(정면), +X 방향을 +90°(오른쪽)로 정의 | 코드베이스 어디에도 "월드 프레임에서 방위각 0°가 어느 쪽인가"를 정의해둔 곳이 없다(기존 코드는 오브젝트 위치 없이 머리 방향만 다뤘으므로 이 정의가 필요 없었다). 방·카메라 관례상 Y=깊이 축을 정면으로 삼는 것이 room_zone 좌표계와 가장 자연스럽게 맞는다 |
| 헤드 회전 결합 | `relative_azimuth[ch] = base_azimuth[ch] - head_yaw_deg` | 기존 단일-소스 코드의 `azimuth_deg = -yaw.to_degrees()`(정면 고정 소스, `base_azimuth=0`인 특수 사례)를 그대로 포함하는 일반화. 머리를 오른쪽으로 돌리면(+yaw) 고정된 소리는 상대적으로 왼쪽(음의 방향)으로 이동해야 하므로 부호를 유지한다 |
| SOFA 방위각 부호 관례 | **미확정 — 실기 검증 필요** | MIT KEMAR SOFA 데이터셋이 양의 azimuth를 왼쪽/오른쪽 어느 쪽으로 두는지 코드베이스에서 확인할 수 없었다. 기존 코드는 헤드 요각 하나만 다뤄서 이 부호가 틀려도 "머리를 돌리면 좌우 어느 쪽이든 필터가 바뀐다"는 것만 보장되면 티가 안 났다. 채널별 위치가 들어가면 "오른쪽 채널이 실제로 오른쪽 귀에서 크게 들리는가"가 명확히 검증 가능해지므로, §5 검증 계획에 실기 확인 절차를 명시한다 |
| 고도각(elevation) | v1 범위 밖. 항상 0°로 고정(기존 단일-소스 코드와 동일) | `SourcePosition::new(azimuth_deg, 0.0, distance)`가 이미 고도각을 0.0으로 하드코딩하고 있고, `azimuth_buckets`도 방위각 360방향으로만 인덱싱한다(고도각 무시). 이번 작업은 그 기존 제약을 유지한 채 "방위각만이라도 실제 위치를 쓰게" 만드는 것이 범위다. 천장 스피커가 있는 레이아웃에서는 높이 차이가 반영되지 않는다는 한계가 남는다 |
| 연산 위치 | 오디오 스레드 내부, `process_interleaved()` 안 | 이미 그 함수가 오디오 콜백에서 실행 중이고(Law 1/2 준수 상태), 채널당 최근접-3-탐색 + IR 보간은 채널 수 × IR 길이 규모의 가벼운 연산이라(예: 12채널 × 256탭 ≈ 3072 float 연산/콜백) 콜백 주기 안에서 무리 없다. 새 힙 할당 없이 기존 스크래치 버퍼(`interp_ir_left/right`)를 채널 루프 안에서 재사용한다 |
| `channel_base_azimuth` 갱신 시점 | `UpdateSpatialConfig` 커맨드 처리 시(오디오 스레드, 포인터 교체만) | `channel_positions`/`room_zones`가 바뀔 때만 재계산하면 되고, atan2 계산 자체는 채널 수만큼의 가벼운 루프라 오디오 스레드에서 직접 해도 Law 1 위반이 아니다(힙 할당 없음, 이미 있는 `Vec<f32>`에 덮어쓰기만). 초기반사음 탭 계산처럼 비-오디오 스레드로 옮길 필요는 없다 |

---

## 2. 상세 설계

### 2.1 채널 기준 방위각 계산

각 채널 `ch`가 위치 `pos = channel_positions[ch]`를 가지고, `RoomZone` `zone`에 바인딩되어 있다면(바인딩 판정은 `mixer.rs:365`의 기존 point-in-room 로직 재사용):

```
listener_x = (zone.boundary_min.x + zone.boundary_max.x) / 2
listener_y = (zone.boundary_min.y + zone.boundary_max.y) / 2
// listener_z(ear_level)는 방위각 계산에 쓰지 않는다 — 고도각 미지원(0.1절)

dx = pos.x - listener_x
dy = pos.y - listener_y

base_azimuth_deg = atan2(dx, dy).to_degrees()   // +Y=0°(정면), +X=+90°(오른쪽)
```

바인딩된 Zone이 없으면 `base_azimuth_deg = 0.0`(정면 취급, §1 표 참고).

이 값을 채널별로 보관: `AudioMixer`(또는 `VirtualMixRoomBinaural` 자신)에 `channel_base_azimuth: Vec<f32>` 신설. `UpdateSpatialConfig` 핸들러가 `channel_positions`/`room_zones`를 갱신할 때 함께 채운다(정확한 삽입 지점은 `engine.rs`의 기존 `UpdateSpatialConfig` 분기 — `channel_early_ref_taps` 이관과 나란히 추가).

### 2.2 오디오 콜백에서의 적용 (`process_interleaved` 재작성 스케치)

```rust
// 헤드 방향이 바뀌었을 때만 재계산하던 기존 가드는 유지하되,
// "채널 위치가 바뀌었다"는 신호도 재계산 조건에 추가해야 한다
// (새 필드 channel_base_azimuth_dirty: bool 또는 세대 카운터로 감지).
if head_changed || channel_base_azimuth_dirty {
    for ch in 0..num_ch {
        let target_azimuth = self.channel_base_azimuth[ch] - yaw.to_degrees();
        let target = SourcePosition::new(target_azimuth, 0.0, self.nominal_distance);
        let nbrs = Self::find_three_nearest_indexed(db, &self.azimuth_buckets, &target);

        // 기존 3-근접 보간 로직 그대로, target만 채널별로 바뀐다.
        // interp_ir_left/right 스크래치 버퍼를 채널마다 덮어써서 재사용(Law 1: 새 할당 없음).
        ...채널별 보간...

        self.channels[ch].update_hrtf(&self.interp_ir_left, &self.interp_ir_right);
    }
}
```

그 뒤의 de-interleave → `process_block` → 합산 로직은 기존 그대로다. 바뀌는 것은 "모든 채널에 같은 IR"에서 "채널마다 자기 위치에 맞는 IR"로 바뀌는 것뿐이다.

### 2.3 오브젝트 모드와의 관계 — 왜 별도 처리가 필요 없는가

DBAP가 오브젝트를 여러 물리 채널에 거리 가중치로 이미 분산시켜 놓았으므로, 위 방식은 오브젝트 소리도 자동으로 올바른 방향으로 재현한다. 그 채널들이 실제로 그 방향에 있다고 보고 렌더링하기 때문이다.

**알아둬야 할 특성 (결함 아님, 문서화 필요):** 오브젝트가 두 스피커 사이(예: Ch-1과 Ch-2 사이)에 있으면, DBAP가 두 채널에 에너지를 나눠 보내고, 바이노럴 단계는 그 두 채널을 각자의 실제 방향에서 오는 것으로 렌더링한다. 그 결과 헤드폰으로는 "두 방향에서 동시에 들리는" 것처럼 느껴질 수 있다. 이것은 실제 그 방에 스피커 두 대를 두고 서 있어도 동일하게 들리는 현상이므로 정확한 재현이지 결함이 아니다. 다만 사용자가 "왜 오브젝트가 한 점이 아니라 두 방향에서 들리지"라고 오해할 수 있으므로 UI 도움말이나 문서에 미리 적어두는 것을 권한다.

바인딩되지 않은 채널(§1의 "정면 취급" 결정)이 오브젝트 채널일 경우, 특히 오브젝트 모드는 `channel_positions[ch]`가 `Some`인 게 보통이므로(오브젝트 트랙은 `current_position`이 DBAP 가중치 계산에 이미 쓰이고 있음) 실질적으로 바인딩 안 되는 경우는 드물다. 순수 베드 채널 중 어떤 RoomZone에도 속하지 않는 경우(설정 누락)에 주로 해당한다.

### 2.4 초기반사음과의 상호작용

`channel_early_ref_taps`(초기반사음)는 `dsp.rs`의 채널별 DSP 체인에서 이미 처리되어 `out`에 합산된 뒤 `mixer.rs`의 후단으로 넘어간다. 바이노럴 단계(`process_interleaved`)는 그 이후, 채널 DSP를 모두 거친 최종 채널 신호를 입력받는다(§0.2 참고: 이 함수는 `output` 버퍼, 즉 이미 초기반사음이 섞인 신호를 de-interleave한다). 따라서 **추가 배선 없이 초기반사음도 자동으로 바이노럴 렌더링에 포함된다.** 별도 작업 불필요.

---

## 3. 데이터 구조 변경

`rust/src/audio/binaural.rs`의 `VirtualMixRoomBinaural`에 추가:

```rust
pub struct VirtualMixRoomBinaural {
    // ...기존 필드...
    channel_base_azimuth: Vec<f32>,   // len == channel 수, 도 단위, 0°=정면
    base_azimuth_dirty: bool,          // channel_positions/room_zones 갱신 시 true
}
```

`new(num_channels, block_size)`에서 `channel_base_azimuth: vec![0.0; num_channels]`로 초기화(전부 정면 취급 — 안전한 기본값).

`AudioMixer`(`mixer.rs`)에 새 메서드 추가, 기존 `channel_early_ref_taps` 이관과 같은 자리(엔진의 `UpdateSpatialConfig` 핸들러)에서 호출:

```rust
impl AudioMixer {
    pub fn recalculate_binaural_channel_azimuths(&mut self) {
        for ch in 0..self.binaural.channel_base_azimuth.len().min(self.channel_positions.len()) {
            let az = self.channel_positions[ch].as_ref()
                .and_then(|pos| find_bound_zone(pos, &self.room_zones))
                .map(|zone| compute_base_azimuth(pos, zone))
                .unwrap_or(0.0);
            self.binaural.channel_base_azimuth[ch] = az;
        }
        self.binaural.base_azimuth_dirty = true;
    }
}
```

(`find_bound_zone`/`compute_base_azimuth`는 §2.1 공식을 함수로 뺀 것 — `mixer.rs:365`의 기존 point-in-room 루프와 중복 코드가 생기지 않도록, 가능하면 그 루프가 이미 계산한 `bound_room_id`를 재사용하는 리팩터링을 함께 고려할 것. 지금은 두 곳에 유사 로직이 따로 존재하는데, 이번 작업으로 세 번째 자리가 생기는 것이므로 공통 헬퍼로 뽑는 것을 권장한다.)

---

## 4. 범위 밖 (v1에서 하지 않는 것)

- **고도각 기반 HRTF.** 천장 스피커와 바닥 스피커가 방위각은 같고 높이만 다르면 지금 설계로는 구분되지 않는다. SOFA 데이터셋이 고도각 측정을 포함하는지부터 확인해야 하는 별도 조사가 필요하다(현재 `azimuth_buckets`가 애초에 고도각을 버리고 인덱싱하므로, 이 구조 자체를 바꿔야 한다 — v2 과제).
- **거리 기반 게인/스펙트럼 차이.** `SourcePosition`에 `distance` 필드가 있고 `lookup_distance()`가 이미 거리 성분을 반영해 근접 HRIR을 고르지만(§`binaural.rs:380`), 이는 "실측 위치와 가장 가까운 샘플을 고르는" 용도이지 "멀수록 작게" 같은 거리 감쇠를 새로 만드는 것이 아니다. 거리 감쇠는 이미 DBAP 단계(`mixer.rs`)에서 처리되고 있으므로 중복 적용하지 않는다.
- **오브젝트별 개별 HRTF 직접 렌더링.** §2.3에서 설명했듯 채널 기반 렌더링이 이를 자동으로 포함하며, 오브젝트를 DBAP 이전 단계에서 별도로 뽑아 렌더링하면 오히려 이중 처리가 된다.
- **바이노럴 토글 UI 복구(P1-8).** 이 문서는 DSP 계산 로직만 다룬다. UI에서 켜고 끄는 스위치를 다시 연결하는 것은 별개의 작은 작업이며, 이 재설계가 끝난 뒤 함께 처리하는 것을 권한다(지금 연결해봐야 방향이 틀린 렌더링을 켜는 것밖에 안 된다).

---

## 5. 검증 계획

1. **단위 테스트 — 채널별로 다른 방위각이 실제로 다른 IR을 고르는가.** 두 채널에 대칭 위치(예: 리스너 기준 정확히 좌/우 90°)를 주고 `channel_base_azimuth`를 계산, 각각의 `find_three_nearest_indexed` 결과 인덱스 집합이 달라야 한다.
2. **단위 테스트 — 헤드 요각이 전체를 균일하게 시프트하는가.** 모든 채널의 `base_azimuth`를 고정한 채 `yaw`만 바꿨을 때, 모든 채널의 `relative_azimuth`가 정확히 같은 만큼 이동해야 한다.
3. **단위 테스트 — 미바인딩 채널은 크래시 없이 0°로 폴백하는가.** `channel_positions[ch] = None`이거나 어떤 zone에도 속하지 않을 때.
4. **회귀 테스트 — 채널이 1개뿐이고 정면(0°)에 고정이면 기존 단일-소스 동작과 동치인가.** `base_azimuth=0`일 때 `relative_azimuth = -yaw`가 되어 기존 코드와 수식이 일치해야 한다.
5. **실기 검증 (필수, macOS에서 불가능 — 헤드폰 청음이 필요) — SOFA 방위각 부호 확인.** 리스너 기준 정확히 오른쪽(+X 방향)에 스피커 하나를 두고, 그 채널에 테스트 톤을 흘려 헤드폰으로 들었을 때 오른쪽 귀에서 크게 들리는지 확인한다. 반대로 들리면 §1의 부호(`base_azimuth_deg = atan2(dx, dy)`의 부호, 또는 `relative_azimuth = base - yaw`의 부호)를 뒤집어야 한다. **이 단계 전까지는 "구현했다"고 말할 수 없다** — 수학이 맞아 보여도 SOFA 데이터셋 자체의 부호 관례를 코드로는 확인할 수 없었으므로 청음 없이는 검증되지 않는다.
6. **실기 검증 — 헤드 트래커로 회전 시 각 채널이 물리적으로 맞는 방향에서 들리는가.** 여러 채널에 각각 다른 음원(예: 채널마다 다른 숫자를 세는 음성)을 흘리고, 머리를 돌리며 각 음원이 예상한 방향에 남아있는지(월드 프레임에 고정되어 있는지, 머리를 따라 돌지 않는지) 확인.

---

## 6-1. 구현 결과 (2026-09-13)

구현 완료, 실기(헤드폰 + Scarlett 6i6)로 확인 시도 중 발견한 두 개의 실제
결함을 함께 고쳤다. 둘 다 이 스펙 작업 이전부터 있던 잠재 결함이며, 내
per-channel 리팩터링이 만든 게 아니라 그 리팩터링이 처음으로 이 코드 경로를
실제로 실행시켜서 드러난 것이다.

**결함 1 — 방위각 부호가 반대(2단계에 걸쳐 확정, 최종 결론은 "반전
없음").**

1단계: 헤드폰 청음(사용자, 화이트 노이즈)과 수치 검증(`binaural_numeric_check`)
둘 다 "azimuth +90°가 왼쪽 귀에서 크게 들린다"는 같은 결과를 냈다. 즉 SOFA
조회에 넣는 azimuth_deg는 양수=왼쪽 우세다(수학 교과서의 "0°=정면,
+90°=오른쪽" 관례와 반대). 이걸 보정하려고 `dx.atan2(dy)`를 `-dx.atan2(dy)`로
고쳤었다.

2단계(실기에서 발견한 회귀): 실제 스피커 레이아웃 + 헤드폰으로 확인하니
"Ch1이 마네킹 정면(=마네킹 눈이 보는 방향, 사용자가 마네킹 뒤통수 너머로
보는 시점 = 3D 룸의 'Back View' 카메라 프리셋) 기준 오른쪽에 있는데
왼쪽에서 들린다"는 반대 방향 보고가 나왔다. 원인은 1단계 수정이 "월드
+X=화면 오른쪽"이라고 검증 없이 가정했기 때문이다. 실제로는
`assets/3d_simulator/studio_engine.html`이 Dart 채널 x좌표를 Three.js
world X에 반전 없이 그대로 매핑하고(`posX = sp.x - room.width/2`), "Back
View" 카메라(`studio_engine.html:607-610`, `-Z`에서 `+Z`를 바라봄)의
forward×up 벡터를 계산하면 화면 오른쪽이 world **-X**다.

두 사실(SOFA 양수=왼쪽, 화면 오른쪽=world -X)을 합치면 서로 상쇄되어,
**최종적으로는 부호를 전혀 뒤집지 않는 원래의 `dx.atan2(dy)`가 정답**이다.
1단계의 `-dx` 수정을 되돌렸다. 1단계에서 SOFA 쪽 사실(코드로 확인
가능)만 검증하고, 2단계의 실제 룸 좌표 매핑(Dart→JS 배선, 코드 읽기로
확인 가능했는데 하지 않음)은 가정으로 남겨둔 것이 회귀의 원인이었다.

**결함 2 (더 심각, 사전 존재) — 채널 합산이 대입이었다.** `BinauralChannel::
process_block()`이 `out_left[i] = ...`로 스테레오 믹스 버스에 대입했다.
호출부(`process_interleaved`)는 여러 채널을 `mix_left`/`mix_right` 하나에
누적하려고 루프 전에 0으로 채워두는데, 정작 함수는 그 값을 채널마다 통째로
덮어썼다. 그 결과 채널이 2개 이상이면 나중 채널(무음이어도)이 앞선 채널의
값을 지워버려 전체 출력이 항상 무음이 됐다. 기존 테스트가 전부 채널 1개
(`VirtualMixRoomBinaural::new(1, ...)`)로만 돌았거나 전 채널이 무음인
입력만 써서 지금까지 드러나지 않았다. `+=`로 고쳤다.

**결함 3 (부수 발견) — FFT 크기가 실제 콜백 프레임 수와 무관하게
고정이었다.** `AudioMixer::new()`가 바이노럴을 항상 `block_size=8192`로
생성했는데, 실제 하드웨어 콜백은 그보다 훨씬 작다(Scarlett 6i6에서 1024).
오버랩-애드 컨볼루션은 매 콜백 오버랩 테일을 처음부터 다시 만드는 구조라,
FFT 크기가 실제 프레임 수보다 훨씬 크면 이전 블록이 넘겨준 테일 대부분을
버리게 되어 출력이 깨진다. `AudioMixer::new()`에 `block_size: usize` 인자를
추가해 `engine.rs`가 실제 해석된 하드웨어 버퍼 크기를 넘기도록 했다(비-오디오
스레드인 엔진 초기화 시점에 한 번만 정해지므로 Law 1 위반 없음). 이 변경으로
호출부 8곳(테스트/probe 포함)이 함께 바뀌었다.

세 결함 모두 회귀 테스트로 고정했다(`tests/test_binaural_channel_azimuth.rs`,
5건 — 되돌리면 각각 독립적으로 실패하는 것을 확인함). 청음 검증용 도구
`rust/src/bin/binaural_azimuth_probe.rs`도 남겨뒀다(Scarlett 등 실제 장치로
직접 재생해 좌우를 확인하는 수동 청음 스크립트, `cargo run --release --bin
binaural_azimuth_probe`).

## 6. 작업 순서 제안

1. `mixer.rs:365`의 point-in-room 바인딩 로직을 공통 헬퍼로 추출(중복 3곳 방지 — pan_deg 피벗, 초기반사음, 이번 작업).
2. `AudioMixer::recalculate_binaural_channel_azimuths()` 추가, `UpdateSpatialConfig` 핸들러에서 호출.
3. `VirtualMixRoomBinaural`에 `channel_base_azimuth`/`base_azimuth_dirty` 필드 추가.
4. `process_interleaved()`를 §2.2대로 재작성(채널 루프 안으로 근접-탐색+보간 이동).
5. §5의 단위 테스트 1~4 작성 및 통과 확인(macOS에서 가능).
6. 실기(헤드폰 + 스피커 레이아웃 실측)로 §5의 5~6 검증. 부호가 틀리면 5단계 재작성 없이 §2.1의 부호 하나만 뒤집으면 되도록 미리 그 지점을 명확히 분리해서 구현할 것.
