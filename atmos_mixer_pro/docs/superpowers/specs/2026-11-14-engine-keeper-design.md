# 엔진 유지 스레드와 상태값 설계 (안정성 ③, 초안)

> 상태: **확정(2026-11-14, Windows Main 작성, 13절 사용자 결정 반영). 구현됨(브랜치 `win/engine-keeper`, 이 PC 빌드·시험 통과). 앱 실행 확인·④ 실기는 남음.**
> 기준 코드: origin/main `d60b8c5`. 행 번호·함수 이름은 이 커밋 기준이다(PR #99·#101이 병합되면 `simple.rs` 행이 조금 밀린다).
> 근거: `docs/HANDOFF.md` 확정된 결정 "안정성 검증 프로젝트와 운용 결정(2026-11-10)", 남은 일 16의 ③, 저장소 밖 `D:\dev\harness\REPORT_v1.3.10_verification.md` 1절(지금 복구 경로의 코드 사실).
> 확인 수준: "지금 동작"은 코드 읽기다. 실행·측정은 하지 않았다.
> 경로는 `atmos_mixer_pro/` 기준.

## 1. 목적과 범위

### 1.1 왜
무인 운영에서 엔진이 내려간 뒤 **사람 없이 돌아와야** 한다. 지금은 자기 재시작(워치독·장치 오류·장치 목록 변화)의 재연결이 한 번뿐이다. 설정 장치와 기본 장치(비상 전환)가 모두 실패하면 아무도 다시 시도하지 않는다. 앱 안의 감시(엔진 스레드의 점검 루프)도 엔진과 함께 사라지고, 감시 프로그램은 `engine_active=false`이면 오디오 멈춤을 판단하지 않는다. 결과는 "사람이 알아차리기 전까지 무음"이다(검증 보고 1.2·1.3).

### 1.2 목표
1. 엔진 수명을 맡는 **단일 소유 스레드(유지자)** 를 둔다. 자기 재시작과 재시도는 모두 유지자가 한다.
2. 설정 장치가 없으면 **상한 30초 간격으로 계속 찾는다**(`WaitingForDevice`). 포기하지 않는다.
3. 엔진 상태를 **값 하나**로 내보낸다: `Running / WaitingForDevice / Failed / StoppedByUser`. 진행 카운터도 함께 내보낸다. 하트비트·상태 방송·OSC 시작 신호와 상태 조회가 같은 값을 쓴다.
4. 엔진 시작 호출이 돌아오지 않으면(ASIO 드라이버 멈춤) 기다리기만 하지 않고 `Failed`로 알린다.
5. 대기·실패 상태에서 들어온 재생 명령은 쌓지 않고 버린다(OSC 응답 `rejected engine_waiting`).
6. 장치가 돌아오면 **끊긴 시간만큼 건너뛴 자리에서** 이어 간다(루프는 길이의 나머지).

### 1.3 하지 않는 것(이 단계 밖)
- **비상 전환(기본 장치 −40dB) 제거**는 ⑤다. 이 단계에서는 지금처럼 둔다(HANDOFF 순서 조건: 유지자가 실기 ④를 통과하기 전에는 지우지 않는다). 그래서 이 단계의 `WaitingForDevice`는 **기본 장치까지 실패했을 때만** 생긴다. ⑤ 뒤에는 장치가 없으면 바로 대기다.
- **감시 프로그램의 재실행 정책**(새 상태값으로 언제 앱을 다시 띄울지)은 ⑥이다. 이 단계는 하트비트에 값을 싣기만 하고, 감시는 지금 규칙 그대로다(`engine_active`도 그대로 싣는다).
- **이어 가지 않을 트랙 규칙**(비루프 남은 10초 이하)은 ⑦이다. 다만 6절의 건너뛰기로 파일 끝을 넘은 비루프는 이어 가지 않는다(넘었으니 끝난 것).
- 실기 시험(USB 뽑기, 늦게 켜기, 끈 채 장시간)은 ④다. 이 문서는 시험 항목만 적는다(10.2).

## 2. 지금 동작

- **시작**: `api_init_audio_system(device)`(`api/simple.rs` 1034행) → `start_audio_system`이 엔진 스레드를 띄우고 `rx_init.recv()`로 결과를 **시간 제한 없이** 기다린다(1424행 근처). 엔진 스레드 안 `AudioEngine::start`(`audio/engine.rs` 123행)가 장치를 **최대 30초** 찾는다(0.5초 간격, `timeout_secs = 30`). 설치되지 않은 ASIO 드라이버 등 나타날 수 없는 이름은 `device_search::reject_before_search`로 바로 실패한다. 첫 시작에서 설정 장치를 못 열고 `may_appear_later`면 기본 장치로 비상 전환한다.
- **점검 루프**(엔진 스레드, 100ms): 세대 변경, `device_needs_reset`, `DeviceNotAvailable`, 워치독(콜백 1초 공백), WASAPI·CoreAudio 장치 목록 변화(ASIO는 보지 않음)면 빠져나온다. 빠져나오면 `take_snapshot` → `drop(engine)` → `ENGINE_ACTIVE=false`, `"Stopped"` 방송.
- **재연결**: 세대가 그대로면(자기 재시작) **일회성 스레드**를 하나 띄운다. 100ms 뒤 `device_for_self_restart`로 장치를 정하고 `init_audio_system` → 실패하면 기본 장치(`is_failover_mode=true`) → 그것도 실패하면 사건 `engine_down retry=none`과 `EngineDown:restart_failed`를 방송하고 **끝난다**. 다시 시도하는 코드가 없다.
- **복귀**: 비상 전환 엔진(WASAPI)의 장치 목록 감시가 목록 변화를 보면 자기 재시작 → 설정 장치가 목록에 있으면 그 이름으로 연다.
- **사용자 정지·장치 변경**: `api_stop_audio_engine` / `api_init_audio_system`이 세대를 올린다. 세대가 바뀐 엔진은 자기 재시작을 하지 않는다.
- **하트비트**: `engine_active`(`ENGINE_ACTIVE`)와 `audio_cb_ms`. 감시는 `engine_active=false`이면 오디오 멈춤을 보지 않는다(`supervisor/src/watch.rs`).
- **재생 명령**: 엔진이 꺼진 동안의 명령은 명령 큐에 남는다. 자기 재시작이면 `take_snapshot`이 큐의 `PlayTrack`을 0초 재개 항목으로 바꾼다(재시작 복원이 살림). 오래 꺼져 있으면 쌓인 명령이 복귀 때 한꺼번에 난다.
- **화면**: 상태 문자열 `Running · Failover · Stopped · HotReloading · EngineDown:<원인>`(`engine_down_banner.dart`). `EngineDown`은 지속 배너다(PR #86·#91).

## 3. 결정 요약

| # | 결정 | 근거 |
|---|---|---|
| D1 | 새 모듈 `core/engine_keeper.rs`의 **유지자 스레드 하나**가 엔진 수명을 맡는다. 일회성 재연결 스레드를 없애고, 엔진 스레드는 자기 재시작이 필요하면 유지자에게 알리기만 한다 | 소유자가 하나여야 "아무도 다시 시도하지 않는" 틈이 없다 |
| D2 | 유지자가 원하는 상태(`Desired::Run(장치) / Desired::Stop`)와 지금 상태(`EngineState`)를 가진다. 사용자 정지는 `Stop`으로, 장치 변경은 `Run(새 장치)`로 바꾼다. 세대 번호(`ENGINE_GENERATION`)는 지금처럼 쓴다 | 사용자 정지와 자기 재시작의 경합을 지금 규칙으로 막는다 |
| D3 | 열기 시도 순서는 지금과 같다: 설정 장치 → 실패하면 기본 장치(비상 전환, ⑤까지 유지). **둘 다 실패하면 `WaitingForDevice`로 두고 다시 시도한다.** 간격은 1·2·4·8·16·30초, 그 뒤 30초마다(U1) | HANDOFF "간격 상한 예 30초" |
| D4 | 다시 시도할 때 장치 찾기 시간을 30초에서 **5초**로 줄인다(유지자의 재시도만). 첫 시작과 사용자 요청은 지금처럼 30초 | 재시도마다 30초씩 막히면 상태·카운터가 느리게 움직인다. 재시도 간격이 찾기를 대신한다 |
| D5 | 엔진 시작 결과를 **시간 제한으로** 기다린다: 찾기 시간 + 15초(첫 시작 45초, 재시도 20초). 넘으면 `Failed(start_hung)`로 두고 더 시도하지 않는다(멈춘 드라이버 호출을 끝낼 방법이 없어 새 시도는 같은 드라이버에서 또 멈춘다). 회복은 앱 재실행이며 그 정책은 ⑥이다 | HANDOFF 앱 재실행 조건 "엔진 시작 호출이 돌아오지 않음" |
| D6 | 장치는 찾았는데 열기(스트림 만들기·시작)가 **연속 3번** 실패하면 `Failed(open_failed)`로 바꾼다. 다시 시도는 30초 간격으로 계속한다(혼자 회복할 수도 있다) | HANDOFF 앱 재실행 조건 "장치는 있는데 열기가 연속 실패". 감시 횟수 규칙(3)과 맞춘다 |
| D7 | 상태값은 `Running / WaitingForDevice / Failed / StoppedByUser` 넷이다. 비상 전환 중은 `Running`이고 따로 `failover` 표시를 둔다. 여는 중(`Starting`)은 상태값으로 따로 두지 않고 직전 상태를 유지한다(전이 중 깜빡임 방지) | OSC 규격 §3.2·§3.6이 이미 네 값을 쓴다 |
| D8 | **진행 카운터** `engine_progress`: 유지자가 열기를 시도할 때마다(성공·실패 모두), 그리고 `WaitingForDevice`·`Failed`에서 기다리는 동안 5초마다 1씩 올린다. `Running`·`StoppedByUser`에서는 멈춰 있어도 된다 | 감시(⑥)가 "유지자가 살아 있다"를 본다. 대기 중 카운터가 늘면 정상 |
| D9 | 하트비트에 `engine_state=<값>`, `engine_progress=<수>` 두 줄을 **더한다**. 기존 다섯 줄은 그대로다. 지금 감시 파서는 모르는 키를 건너뛴다(`supervisor/src/heartbeat.rs` `parse`의 `_ => {}`, d60b8c5 확인) | 감시 정책은 ⑥. 앱·감시가 설치본으로 같이 바뀌어도, 섞인 버전에서 깨지지 않아야 한다 |
| D10 | 상태 방송 문자열: 지금 값을 유지하고 `WaitingForDevice:<재시도 횟수>:<다음 시도까지 초>`와 `Failed:<원인>`을 더한다. `EngineDown:<원인>`은 `Failed:<원인>`과 같은 뜻으로 남긴다(배너 호환) — 구현 때 둘 중 하나로 합칠지 정한다(12절) | 화면은 지속 배너로 보인다(무인 현장에 스낵바는 사라진다) |
| D11 | `WaitingForDevice`·`Failed` 동안의 재생 명령(대시보드·OSC·테마 시작·방 클리어 다음 방)은 **엔진 큐에 넣지 않고** 버린다. OSC는 `rejected engine_waiting`으로 답한다. 정지·비우기 명령은 지금처럼 받는다(정지 순번을 올려 이어 갈 목록에서도 뺀다) | OSC 규격 §3.4·§4 "대기 중 재생 명령은 버린다" |
| D12 | 엔진이 다시 열려 이어 갈 때(장치 복귀와 짧은 자기 재시작 모두, U2) **끊긴 시간만큼 건너뛴다**: 위치 = 스냅샷 위치 + (이어 가는 순간 − 스냅샷 순간, 단조 시계). 루프는 파일 길이의 나머지, 비루프는 끝을 넘으면 이어 가지 않는다. 그룹 시작·같은 길이 맞추기(`align_plan`)는 건너뛴 뒤의 위치로 한다 | 사용자 결정 2026-11-10(장치 복귀), 2026-11-14(U2: 짧은 자기 재시작에도) |
| D13 | OSC 시작 신호(`/awesome/status/boot`)의 엔진 상태 인자와 상태 조회 스냅샷이 D7의 값을 쓴다 | OSC 규격 §3.2·§3.6이 3단계에서 확정하기로 한 부분 |

## 4. 상태와 전이

| 지금 상태 | 일 | 다음 상태 |
|---|---|---|
| (앱 시작) | `api_init_audio_system(장치)` → 유지자에 `Run(장치)` | 성공 `Running`, 실패(비상 전환까지) `WaitingForDevice` |
| `Running` | 점검 루프가 자기 재시작 사유를 봄(워치독·장치 오류·목록 변화) | 스냅샷 → 유지자가 곧바로 다시 엶. 성공 `Running`, 실패 `WaitingForDevice` |
| `WaitingForDevice` | 간격(D3)마다 설정 장치 → 기본 장치 순으로 시도 | 성공 `Running`(+6절 이어 가기). 장치는 찾았는데 열기 3연속 실패 `Failed(open_failed)` |
| `Failed(open_failed)` | 30초마다 계속 시도 | 성공 `Running` |
| 아무 상태 | 시작 호출이 시간 제한(D5)을 넘김 | `Failed(start_hung)`, 더 시도 안 함 |
| 아무 상태 | 사용자 정지(`api_stop_audio_engine`) | `StoppedByUser`, 재시도·재개 항목 취소 |
| `StoppedByUser`·`Failed` | 사용자가 시작·장치 변경·리스캔 | 처음 시작과 같음 |

- 유지자는 시도마다 세대를 확인한다. 기다리는 동안 사용자가 정지·장치 변경을 하면 남은 대기를 버리고 새 원하는 상태를 따른다(대기는 짧게 쪼개 `Condvar`로 깨운다).
- `Failed(start_hung)`에서는 멈춘 엔진 스레드가 `ENGINE_THREAD`에 남는다. 새 `start_audio_system`은 그 스레드를 `join`하므로(1094행) 같이 멈춘다. 그래서 사용자 요청도 바로 실패로 돌려준다(화면: "오디오 드라이버가 응답하지 않습니다. 앱을 다시 시작해야 합니다").

## 5. 구조

### 5.1 `core/engine_keeper.rs`
- `pub enum EngineState { Running, WaitingForDevice, Failed(FailCause), StoppedByUser }`, `FailCause { StartHung, OpenFailed }`. 문자열 변환은 이 모듈 하나에 둔다(하트비트·방송·OSC가 같은 함수를 쓴다).
- 전역: 상태(`Mutex<KeeperState>` — 오디오 스레드가 읽지 않는다), `ENGINE_PROGRESS: AtomicU64`, 상태 숫자 사본 `ENGINE_STATE_CODE: AtomicU8`(하트비트·OSC가 잠금 없이 읽음).
- `pub fn request(desired: Desired)`: 원하는 상태를 바꾸고 유지자를 깨운다. 유지자 스레드가 없으면 만든다(앱에서 한 번).
- `pub fn report_self_restart(gen, reason)`: 엔진 스레드가 점검 루프를 빠져나와 자기 재시작이 필요할 때 부른다(지금 일회성 스레드를 띄우던 자리).
- 유지자 루프: 원하는 상태가 `Run`이고 엔진이 없으면 열기 시도 → 결과로 상태를 정하고 간격을 기다린다. 시도는 지금 함수(`init_audio_system`)를 그대로 부르되 찾기 시간(D4)과 결과 대기 시간 제한(D5)을 인자로 넘긴다.
- **시험용 주입 지점**: 열기 함수를 `fn(Option<String>, StartOptions) -> Result<(), StartError>` 형태의 교체 가능한 값으로 둔다(기본은 실제 `init_audio_system`). 시험은 "N번 없음 뒤 성공", "찾았는데 열기 실패", "돌아오지 않음"을 흉내 낸다. 시간은 간격 표를 인자로 받아 시험에서 ms 단위로 줄인다.

### 5.2 `api/simple.rs`
- `start_audio_system`: `rx_init.recv()` → `recv_timeout(D5)`. 찾기 시간을 인자로 받아 `AudioEngine::start`에 넘긴다(`engine.rs`의 `timeout_secs = 30`을 인자로). 오류를 "찾지 못함(`NotFound`)·열기 실패(`OpenFailed`)·시간 초과(`Hung`)"로 나눠 돌려준다(지금은 문자열 하나 — 안쪽은 `StartError` 열거형, FRB로 나가는 `AtmosError` 문구는 그대로).
- 일회성 재연결 스레드(1371~1418행)를 `engine_keeper::report_self_restart(gen, stop_reason)`로 바꾼다. `device_for_self_restart`·비상 전환·`set_short_startup_ramp`·`resume_after_resync`의 순서는 유지자 안으로 옮기고 그대로 지킨다.
- `api_init_audio_system`·`api_stop_audio_engine`은 시그니처 그대로다. 안에서 `engine_keeper::request`를 부르고, 첫 시도의 결과를 지금처럼 돌려준다(화면의 첫 시작 흐름 유지). 실패해도 유지자는 계속 시도한다.
- 재생 진입점(`play_track_from_in`, `api_theme_start`, `clear_room_and_advance`의 다음 방, `play_group_*`): 상태가 `WaitingForDevice`·`Failed`면 `Err(engine_waiting)`을 돌려주고 명령을 보내지 않는다(D11). 판단은 `engine_keeper::accepts_play()` 하나로.

### 5.3 하트비트·상태 방송·OSC
- `core/app_signals.rs` `HeartbeatValues`에 `engine_state: String`, `engine_progress: u64`를 더하고 `render`에 두 줄을 뒤에 붙인다.
- 감시의 파서(`supervisor/src/heartbeat.rs`)는 모르는 키를 건너뛴다(코드 확인). 그래서 이 단계에서 감시 코드는 바꾸지 않는다(정책은 ⑥). 시험으로 고정만 한다(10.1).
- `osc/reply.rs`: 시작 신호 인자 2와 상태 조회의 엔진 상태를 D7 값으로. 재생 거절 이유 `engine_waiting`을 쓴다(규격 §3.4에 이미 있음).
- 화면(`engine_down_banner.dart`): `WaitingForDevice:…`이면 지속 배너 "오디오 장치를 기다리는 중 — n번 시도, 다음 m초 뒤"(주황), `Failed:…`이면 원인별 문구(빨강). `Running`이 오면 배너를 내린다.

### 5.4 3법칙
오디오 콜백은 바뀌지 않는다. 유지자·엔진 스레드·API는 모두 오디오 스레드 밖이다. 콜백이 읽는 값은 늘리지 않는다.

## 6. 장치 복귀 때 이어 가기(D12)

- `take_snapshot`이 스냅샷 순간의 단조 시각(`GLOBAL_STATE.monotonic_ms()`, PR #101)을 함께 남긴다.
- 유지자가 다시 열기에 성공하면 `resume_after_resync` 전에 재개 항목의 위치를 `+ 끊긴 시간`으로 옮긴다. 순수 함수 `skip_outage(entries, outage_s, file_length_of) -> Vec<ResumeEntry>`로 둔다(시험하기 쉽게).
  - 루프: `(위치 + 끊긴 시간) mod 길이`. 길이를 모르면(파일을 못 읽음) 건너뛰지 않는다.
  - 비루프: `위치 + 끊긴 시간 >= 길이`면 뺀다.
- 같은 길이 맞추기(`align_plan`)와 그룹 시작은 옮긴 위치로 한다 — 같은 순간에 같은 시간을 더하므로 그룹 안 상대 위치가 지켜진다.
- 비상 전환 엔진으로 넘어간 경우(기본 장치에서 계속 재생)는 시간이 흘러가며 재생됐으므로 건너뛸 것이 없다(지금 그대로).

## 7. 바뀌지 않는 것
- 비상 전환의 동작·−40dB·복귀 트리거(⑤까지).
- 감시 프로그램의 판정 규칙(⑥까지). 다만 `engine_active=false`인 대기 상태에서 감시가 앱을 다시 띄우지 않는 지금 동작이 이 단계에서는 맞다(유지자가 앱 안에서 찾는다).
- 사용자 정지·장치 변경·리스캔의 화면 흐름과 FRB 시그니처.
- 재시작 복원(`restart_resume`)의 큐·정지 순번·재동기화(Dart ack) 규칙. 6절의 위치 옮기기만 더한다.

## 8. 화면 문구(제안, U4)
- 대기: "오디오 장치를 기다리는 중입니다 — 3번 시도, 다음 시도 30초 뒤. 인터페이스 전원·USB를 확인하세요."
- 열기 실패: "오디오 장치는 보이지만 열지 못했습니다 — 30초마다 다시 시도합니다. 다른 프로그램이 장치를 쓰는지 확인하세요."
- 시작 멈춤: "오디오 드라이버가 응답하지 않습니다. 앱을 다시 시작해야 합니다."

## 9. 사건 로그
- 유지자 전이마다 `사건: engine_state from=… to=… cause=… attempt=n`. 대기 중 재시도 실패는 같은 원인이면 1분에 한 번만(로그가 장시간 쌓이지 않게).
- 기존 `engine_down`(retry=none) 사건은 `retry=keeper`로 바꾼다.

## 10. 시험

### 10.1 단위·통합(이 PC, 장치 없음)
`rust/tests/test_engine_keeper.rs`(주입한 열기 함수, 간격 ms):
- 없음 3번 → 있음: `WaitingForDevice` → `Running`, 카운터 4 이상, 간격이 표대로 늘어난다.
- 설정 장치·기본 장치 모두 실패 → `WaitingForDevice`이고 시도가 멈추지 않는다(지금 버그의 회귀 시험).
- 찾았는데 열기 실패 3연속 → `Failed(open_failed)`, 계속 시도, 성공하면 `Running`.
- 돌아오지 않음 → 시간 제한 뒤 `Failed(start_hung)`, 더 시도하지 않음, 사용자 요청도 바로 실패.
- 대기 중 사용자 정지 → `StoppedByUser`, 남은 대기 버림, 재시도 없음. 대기 중 장치 변경 → 새 장치로 바로 시도.
- 대기 중 재생 요청 → `Err(engine_waiting)`, 명령 큐가 비어 있다. OSC 리스너로 보내면 `rejected engine_waiting`(실제 리스너 시험 방식).
- 하트비트 렌더에 두 줄이 붙고, 감시 파서가 새 하트비트를 읽는다(`supervisor` 시험에 한 건). 옛 감시 파서가 모르는 키를 건너뛰는지도 시험으로 고정.
- `skip_outage`: 루프 나머지, 비루프 끝 넘으면 뺌, 길이 모름이면 그대로, 그룹 상대 위치 유지.
- 기존: `test_restart_resume`(+_races), `test_watchdog_restart_no_deadlock`(소스 감사 — 재연결 블록 표식이 바뀌므로 감사 대상을 유지자로 옮긴다), `test_failover_return_device`, `test_engine_drop_releases_stream`, `test_device_search`, `test_app_signals`, `test_osc_reply*`, 전체 `cargo test`·`flutter test`·`flutter analyze`·`cargo clippy`(새 경고 0).

### 10.2 실기(④, Windows, RME ASIO MADIface USB — 소리가 나는 시험이라 장치·음량 확인 뒤)
1. 재생 중 USB 뽑기 → 지금은 비상 전환(기본 장치)으로 간다. 기본 장치를 사용자가 Windows에서 꺼 둔 상태로도 해 본다 → `WaitingForDevice` 배너·하트비트·카운터 → USB 다시 꽂기 → `Running`, 끊긴 시간만큼 건너뛴 위치에서 이어 감(공연 상태 파일로 확인).
2. 인터페이스를 끈 채 앱 시작 → 대기 → 켜면 이어 감.
3. 인터페이스를 끈 채 몇 시간 → 메모리·핸들 증가 없음(monitor.ps1), ASIO 드라이버 반복 불러오기 이상 없음.
4. 대기 중 OSC 재생 신호 → 거절 응답, 복귀 때 한꺼번에 나지 않음.

## 11. 구현 순서
브랜치 `win/engine-keeper`(구현 시점 origin/main에서). Draft PR을 먼저 연다.
1. `StartError`·찾기 시간·시작 대기 시간 제한(5.2 앞부분). → 기존 시험 그대로.
2. `engine_keeper` 모듈과 주입 지점, 상태·카운터, 유지자 루프. 일회성 재연결 스레드를 유지자로. → 10.1 유지자 시험.
3. 재생 거절(D11)과 OSC 응답. → OSC 시험.
4. 하트비트·상태 방송·OSC 상태값, 화면 배너. → 하트비트·감시 파서·Dart 시험.
5. 이어 가기 건너뛰기(6절). → `skip_outage` 시험.
6. 전체 시험·clippy → Release 빌드 → 앱 실행 확인(정상 시작·사용자 정지·리스캔) → ④ 실기는 사용자와 날을 잡는다.
7. 문서: HANDOFF(확정된 결정의 지금 동작, 남은 일 16 ③), OSC 규격 §3.2·§3.6·§3.4 구현 현황, 이 문서 상태 줄, `WINDOWS_FIELD_CHECKLIST.md`에 10.2.

## 12. 구현 때 정할 것
- `EngineDown:<원인>`과 `Failed:<원인>`을 하나로 합칠지(화면·사건 로그·OSC가 같은 문자열을 쓰게).
- 유지자 기다림의 구현(`Condvar` 또는 짧은 sleep 반복). 단, 사용자 정지에 100ms 안에 반응해야 한다.
- `StartError` 분류의 근거 문자열(엔진 쪽 오류 문구를 열거형으로 바꾸는 범위).

## 13. 사용자 결정(2026-11-14, "응 네가지 제안 진행해")

| # | 내용 | 결정 |
|---|---|---|
| U1 | 장치를 다시 찾는 간격 | 1·2·4·8·16·30초, 그 뒤 30초마다 |
| U2 | 끊긴 시간 건너뛰기를 **짧은 자기 재시작**(워치독·장치 오류, 보통 1초 안팎)에도 적용할지 | 적용(공연 시간과 다른 장비의 시간이 계속 맞는다). 지금은 짧은 재시작에서 건너뛰지 않는다(HANDOFF 실측 "재시작 동안 앞으로 건너뛰지 않음") — 바뀌는 동작이다. 앱 재실행·로그인 이어 가기(`show_state`)는 이 결정 밖이라 그대로다 |
| U3 | 열기 실패를 "실패"로 볼 연속 횟수 | 3번 |
| U4 | 배너 문구 | 8절 |

## 14. 위험

| 위험 | 대비 |
|---|---|
| 유지자가 사용자 정지·장치 변경과 경합해 엉뚱한 장치를 다시 연다 | 세대 확인을 시도마다, 대기를 깨우는 구조(4절), 시험 10.1 |
| 30초마다 ASIO 드라이버를 불러 몇 시간 뒤 누수·멈춤 | 재시도 찾기 5초, 실기 10.2-3(인터페이스를 끈 장시간) |
| 시작 대기 시간 제한이 느린 정상 장치를 "멈춤"으로 오판 | 찾기 시간 + 15초 여유, 사건 로그에 실제 걸린 시간, 실기에서 값 확인 |
| 대기 중 재생 거절이 짧은 자기 재시작(수백 ms)까지 거절한다 | 거절은 `WaitingForDevice`·`Failed`에서만. 자기 재시작 중(`Running` 유지)에는 지금처럼 큐에 넣고 재시작 복원이 살린다 |
| 하트비트 줄 추가로 감시가 파싱에 실패해 앱을 다시 띄운다 | 지금 파서는 모르는 키를 건너뛴다(코드 확인). 10.1의 파서 시험으로 고정 |
