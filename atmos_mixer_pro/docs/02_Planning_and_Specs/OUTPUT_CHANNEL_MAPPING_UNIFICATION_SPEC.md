# 출력 채널 매핑 통합 설계 명세 (Output Channel Mapping Unification)

- 작성: @Architect
- 상태: Draft for review
- 관련 이슈: 환경설정 트랙 매핑 vs 메인화면 Ext. Out 라벨 불일치 (4ch 파일이 2ch 장치에서 다르게 표시됨)

## 0. 배경 및 원칙

단일 진실 원천은 **현재 연결된 오디오 인터페이스가 실제로 노출하는 출력 채널 목록(개수+이름)**이다.
이 앱은 디스크리트 다채널 시스템이므로 물리 출력 채널 = 개별 스피커이며, 모든 UI는
"연결되어 있고 실제로 소리가 나는 채널"만 동일하게 보여줘야 한다.

현재 3개의 서로 다른 출처가 공존한다:

1. `_channelNames` — `preferences_modal.dart`가 `GlobalDeviceCache.channels`에서 자체 로드
2. `hardwareChannelsProvider` (`lib/core/state/global_state.dart:186`) — `FutureProvider`, 실패 시 `[]` 반환
3. `engineState.outputChannelCount` — Rust 엔진이 실제로 연 오디오 스트림의 채널 수 (`active_device_channels`, `rust/src/audio/engine.rs:262`)

그리고 라우팅 항목 생성 로직이 두 갈래로 갈라져 있다:

- 환경설정 트랙 매핑(`preferences_modal.dart:1193~1320`)은 **Output Config 그룹**(`monoConfigs`/`stereoConfigs`/`multiConfigs`)을 소스로 사용
- 메인화면 Ext. Out(`track_card.dart:109~190`)은 Output Config를 완전히 무시하고 **하드웨어 채널 수를 순회**해서 매 채널마다 항목을 새로 만듦 (`f6b5ece`에서 도입)

이 설계는 이 두 갈래를 하나로 합치고, 나머지 4개 지점(FX, 스피커 노드, 스피커 인스펙터)의 채널 수 소스를 통일한다.

## 1. 단일 채널 Provider 계약

### 1.1 교체 대상
`hardwareChannelsProvider`(`FutureProvider<List<String>>`)를 **`outputChannelsProvider`**로 교체한다.
`preferences_modal.dart`의 `_channelNames` 필드와 그 로딩 로직(`_loadChannelNames` 계열, L272~L305)은 삭제하고
`ref.watch(outputChannelsProvider)`로 대체한다. `engineState.outputChannelCount`를 채널 개수 산정에
사용하는 모든 폴백(`track_card.dart:121`, `tuning_modal.dart:1514`, `speaker_inspector_panel.dart:201`)도 제거한다.

`engineState.outputChannelCount`는 채널 "목록"의 대체재가 아니라 **엔진이 실제로 스트림을 연 채널 수**를
보여주는 진단값으로 역할을 좁힌다(예: 상태바에 "엔진: 8ch 활성" 표시용). UI가 항목을 생성하는 데는 더 이상
쓰지 않는다.

### 1.2 노출 타입

```dart
enum OutputChannelsStatus { loading, ready, noDevice, error }

class OutputChannelsState {
  final OutputChannelsStatus status;
  final String? deviceName;
  final List<String> channelNames; // 0-based 인덱스 = 실제 hw 채널
  final String? errorMessage;
  // ready 여부와 무관하게 "마지막으로 성공한 목록"을 함께 들고 있어
  // 순간적인 재조회 실패로 전체 UI가 비어버리지 않게 한다.
  final List<String>? lastKnownGoodChannelNames;
}

final outputChannelsProvider =
    NotifierProvider<OutputChannelsNotifier, OutputChannelsState>(...);
```

- `loading`: 최초 조회 중 또는 디바이스 전환 중. `channelNames`는 `lastKnownGoodChannelNames`를 유지(있다면)해서
  드롭다운이 순간적으로 비지 않게 한다.
- `noDevice`: `config.deviceName == null` 이거나 장치가 시스템에서 사라짐. `channelNames = []`.
- `error`: `apiGetDeviceChannelNames`/`apiGetDeviceChannelCount` 호출이 예외를 던짐. **현재처럼 조용히 `[]`을
  반환하지 않는다.** `errorMessage`를 채우고 `lastKnownGoodChannelNames`를 유지한다.
- `ready`: 정상. `channelNames`가 곧 진실.

모든 소비 위젯은 `status == error`일 때 인라인 경고(예: "채널 인식 실패 — 마지막으로 확인된 목록 표시 중")를
보여줘야 한다(무음으로 실패하지 않는다는 요구사항 1을 충족).

### 1.3 갱신 트리거
- `configProvider`의 `deviceName` 변경
- `deviceEventStreamProvider`(핫플러그 이벤트) 수신 시 재조회
- 내부 캐시는 `GlobalDeviceCache`를 그대로 재사용하되, **캐시 read/write는 이 provider 내부로만 캡슐화**한다.
  다른 위젯이 `GlobalDeviceCache.channels`를 직접 읽는 코드(`speaker_node_widget.dart:193~222`)는 제거한다.

## 2. 라우팅 항목 생성기 단일화

신규 파일 `lib/core/utils/channel_routing.dart`에 순수 함수(부작용 없음, 테스트 용이)로 만든다.

```dart
class ChannelRoutingItem {
  final String value;       // ChannelDropdownValueHelper 인코딩
  final String label;
  final int realChannel0;   // 0-based hw 채널 (시작 채널)
  final bool isPartialOutput;
}

List<ChannelRoutingItem> buildChannelRoutingItems({
  required List<String> channelNames, // outputChannelsProvider.channelNames
  required AppConfig config,          // monoConfigs/stereoConfigs/multiConfigs
  required int? fileChannels,         // null=아직 메타데이터 로딩 전
});
```

`preferences_modal.dart`의 "트랙별 출력 채널 매핑" 블록(L1193~L1320)과 `track_card.dart`의 Ext. Out 블록
(L109~L190)은 **둘 다 이 함수만 호출**한다. FX(`tuning_modal.dart`)와 스피커 레이아웃/인스펙터는 그룹핑이
필요 없으므로 이 함수를 쓰지 않고 `channelNames`를 직접 순회한다(요구사항 3).

### 2.1 기본값(그룹 미설정) 규칙 — 하위호환
`monoConfigs`/`stereoConfigs`/`multiConfigs`가 모두 비어 있으면 (Rust `enabled_channels`가 "전부 켬"으로
폴백하는 것과 동일하게) 매 하드웨어 채널에 대해 Mono 항목을, 인접 쌍마다 Stereo 항목을, 그리고 파일 채널 수가
들어갈 수 있는 모든 시작 위치에 Multi 항목을 **자동 생성**한다. 이는 현재 메인화면의 동작이자 신규 설치 시
Output Config를 아직 건드리지 않은 사용자를 위한 기본 동작이다.

### 2.2 그룹 설정됨 — Output Config가 지배
하나라도 그룹이 설정되면, 이후로는 **Output Config에 명시적으로 열린 그룹만** 후보가 된다(Ableton 채널
구성처럼). 이는 "어느 쪽 규칙으로 수렴하는가" 질문에 대한 결정이다: 어제 메인화면을 하드웨어 채널 수
기준으로 바꾼 규칙(`f6b5ece`)이 아니라, **환경설정(Output Config) 규칙으로 수렴**한다. 이유: 사용자가
Output Config에서 채널을 명시적으로 열고 이름 붙인 순간, 그 의도가 모든 화면의 상한이 되어야 하며, 반대로
하면(메인화면이 Output Config를 무시) 지금 겪고 있는 버그가 재발한다.

- **Mono**: `monoConfigs`의 key(1-based)마다 `realCh1=key-1`, `realCh2=key`(0-based) 두 항목을 만든다
  (기존 "모노 1개가 L/R 페어를 연다"는 설계를 유지). 라벨: `Mono Ch-{realCh1+1}` / `Mono Ch-{realCh2+1}`,
  `customName`이 있으면 뒤에 `(name L)`/`(name R)` 접미.
- **Stereo**: `stereoConfigs`의 key마다 `realCh=key-1`, 항목 1개. 라벨: `Stereo Ch-{key}/Ch-{key+1}`.
  `realCh+1 >= channelNames.length`면(짝 채널이 없음) 생성하지 않는다.
- **Multi**: `multiConfigs`의 key마다 `startCh=key-1`(0-based) 항목 1개.
  - `requestedEnd = startCh + (fileChannels ?? 2) - 1`
  - `availableEnd = channelNames.length - 1`
  - `actualEnd = min(requestedEnd, availableEnd)`
  - `isPartialOutput = requestedEnd > availableEnd`
  - 라벨: `N-Ch Ch-{startCh+1}~{actualEnd+1}`, partial이면
    `· 부분 출력 {actualEnd-startCh+1}/{fileChannels}ch (나머지 무시됨)` 접미.
  - `startCh >= channelNames.length`면 생성하지 않는다(요구사항: 하드웨어 상한을 넘는 구성은 표시 안 함).

이 규칙이 현재 버그를 고친다: 4ch 파일 + 2ch 장치 + `multiConfigs={1,2}` 조합에서 두 항목은 여전히
따로 존재하지만(`Ch-1~2 · 부분출력 2/4ch`, `Ch-2~2 · 부분출력 1/4ch`), 이제 라벨이 "왜 둘 다 거의 같아
보이는지"를 명시적으로 설명하므로 요구사항 5(부분 출력을 무음이 아니라 명확히 알림)를 만족한다.

### 2.2 라벨 형식 통일
메인화면 형식(`Mono (Ch-1)` / `Stereo (Ch-1/Ch-2)` / `N-Ch (Ch-1~4)`)을 표준으로 채택한다(이미 더
간결하고 Output Config 탭 라벨과 충돌하지 않음). 환경설정 트랙 매핑은 이 형식으로 맞춘다. 단, `customName`이
있으면 끝에 `(name)`을 덧붙이고 부분 출력이면 `· 부분 출력 X/Yc h` 접미를 붙인다.

## 3. 0-based / 1-based 규약 정리

| 위치 | 규약 | 비고 |
|---|---|---|
| `config.json`의 `mono_configs`/`stereo_configs`/`multi_configs` 맵 **키** | 1-based | 레거시 설계, 변경 안 함 |
| `TrackConfig.outputChannel`, `SpeakerNode.channel` | 0-based | `config.json`에 그대로 저장되는 다른 필드지만, 이 필드 자체는 항상 0-based hw 채널 |
| `ChannelDropdownValueHelper` 인코딩 값(`mono_N`/`stereo_N`/`multi_N`) | 0-based | Dart 드롭다운 전용 문자열 인코딩 |
| `GLOBAL_STATE.enabled_channels[]`, CPAL `hw_ch` 인덱스, `channelNames[]` 배열 인덱스 | 0-based | Rust/CPAL 표준 |

**변환 지점은 딱 두 곳으로만 좁힌다:**
1. Dart: `lib/core/utils/channel_routing.dart` — `*Configs` 맵의 1-based 키를 읽어 0-based로 변환하는
   유일한 장소(Output Config 탭 자체 편집기는 예외로, 그 위젯은 맵의 키를 직접 다루므로 1-based 그대로 둔다).
2. Rust: `rust/src/api/simple.rs`의 `api_get_config`/`api_save_config` 내부, `enabled_channels` 계산과
   튜닝 적용(`ch as usize - 1`) 부분. 현재 이 변환이 두 함수에 걸쳐 6곳 가까이 중복돼 있으므로, 본 작업에서
   `fn compute_enabled_channels(config: &AppConfig, hw_len: usize) -> Vec<bool>` 순수 함수로 추출해
   `api_get_config`/`api_save_config`가 공유하게 한다(카파시 원칙상 최소 변경이지만, 검증 계획의 자동
   테스트를 위해 필요한 최소 리팩터).

### 3.1 Rust 측 발견된 결함 (이번 통합의 선행 조건)
`compute_enabled_channels`(현재 인라인 로직, `simple.rs:44~74` 등)는 **`multi_configs`를 전혀 반영하지
않는다.** `mono_configs`/`stereo_configs`가 하나라도 있으면 else 분기로 들어가는데, 이 분기는 mono/stereo
키만 순회한다. 즉 사용자가 Output Config에서 Multi 그룹만 추가로 열어도 그 채널 범위가
`enabled_channels`에서 계속 `false`로 남아 믹서가 조용히 그 출력을 건너뛸 수 있다(요구사항 2 "실제로
소리가 나는가"를 정면으로 위반). 요구사항 4의 의미("Multi는 시작 채널부터 N번까지 연다")에 맞춰
`multi_configs`의 각 활성 키에 대해 `real_ch..hw_len` 구간 전체를 `enabled_channels`에 마킹하도록
고친다. 이는 back-engineer의 선행 태스크다(Dart 작업이 이 값에 의존하지는 않지만, 이 결함을 고치지
않으면 "화면엔 보이는데 소리는 안 남" 문제가 남는다).

## 4. 지점별 적용과 작업 순서

Rust(back)와 Dart(front) 트랙은 서로 다른 언어/파일이라 완전 병렬 가능. Dart 내부는 아래 순서를 지켜
같은 파일을 동시에 건드리지 않는다.

**Track A (back-engineer, Rust, 병렬로 즉시 시작 가능)**
- A1. `rust/src/api/simple.rs`: `compute_enabled_channels()` 추출 + multi_configs 반영 수정,
  `api_get_config`/`api_save_config`가 이를 호출하도록 교체.
- A2. `rust/tests/test_enabled_channels_multi_config.rs`(신규): mono/stereo 없이 multi만 설정된 config,
  mono+stereo+multi 혼합 config에 대해 `enabled_channels`가 기대한 범위를 여는지 검증.

**Track B (front-engineer, Dart, 순차)**
- B1. `lib/core/state/global_state.dart`: `hardwareChannelsProvider` → `outputChannelsProvider` 교체,
  `OutputChannelsState`/`OutputChannelsNotifier` 구현, `deviceEventStreamProvider` 구독 추가.
- B2. `lib/core/utils/channel_routing.dart`(신규): `buildChannelRoutingItems` 구현 (2절 규칙).
- B3 (B1,B2 완료 후, 서로 다른 파일이라 병렬 가능):
  - B3a. `lib/features/settings/widgets/preferences_modal.dart`: `_channelNames` 제거,
    `outputChannelsProvider` + `buildChannelRoutingItems`로 트랙 매핑 블록(L1193~1320) 교체.
    Output Config 탭 자체 그룹 편집기(L560~650, L2090~2230)는 채널 개수만 새 provider에서 받고 로직은 유지.
  - B3b. `lib/features/dashboard/widgets/track_card.dart`: Ext. Out 블록(L109~190)을 동일 함수 호출로 교체.
  - B3c. `lib/features/settings/widgets/tuning_modal.dart`: `hardwareChannelsProvider`/`outputChannelCount`
    참조(L1509~1515)를 `outputChannelsProvider`로 교체(그룹핑 없이 평면 목록).
  - B3d. `lib/features/exhibition/widgets/speaker_node_widget.dart`: `GlobalDeviceCache` 직접 접근(L188~222)
    제거, `outputChannelsProvider`로 교체.
  - B3e. `lib/features/exhibition/widgets/hud/speaker_inspector_panel.dart`: `engineState.outputChannelCount`
    (L201)를 `outputChannelsProvider`로 교체.

권장 순서: A1→A2와 B1→B2를 동시에 시작 → B1,B2 완료 후 B3a~B3e를 파일 단위로 병렬 진행 →
code-reviewer가 3대 DSP 법칙(Rust 쪽은 A1이 `process()` 루프가 아닌 config 로드 경로라 락/할당 제약 완화
대상이지만 `unwrap()` 금지는 유지) 및 diff 충돌 감사 → test-runner가 5절 검증 실행.

## 4-1. 진행 상황 (2026-09-13 기준)

Track A, B 전부 완료. 커밋 순서:

| 작업 | 커밋 | 내용 |
|---|---|---|
| A1, A2 | `d3e3a3f` | `compute_enabled_channels()` 추출, multi_configs 반영, SoundInstance 오프스레드 |
| B1 | `d872fbe` | `outputChannelsProvider` (loading/ready/noDevice/error) |
| B2 | `859a147` | `buildChannelRoutingItems()` 단일 생성기 |
| B3a~e | `6d6dab8` | 5개 UI 전부 마이그레이션, `hardwareChannelsProvider` 삭제 |
| 게이트 | `e5056f4` | 출력 게이트 규칙 통일, 모노→스테레오 업믹스 결함 |
| 감사 반영 | `8237726` | 코드 감사 4건 수정 |

### 구현 중 확정한 설계 결정

**개별 채널 라벨도 단일화했다.** 명세는 라우팅 항목 생성기(`buildChannelRoutingItems`)만
단일화하도록 했지만, 스피커 레이아웃/인스펙터/FX는 그룹핑 없이 개별 채널을 쓰므로 그 생성기를
쓸 수 없다. 그 세 UI가 각자 라벨 템플릿을 갖고 있어 같은 채널이 `Output CH 1`,
`Channel 1 (L) • Out 1`, `Ch-1 Out 1`로 달랐다. `channelDisplayName()`을 추가해
`Ch-N (하드웨어 이름)` 하나로 모았다. FX의 `(L)/(R)` 표기는 짝/홀수 추정일 뿐이고
"개별 채널" 규칙과 어긋나서 제거했다(쌍 편집은 Link L/R이 담당).

**출력 게이트의 "미설정" 판정 기준을 Dart 규칙으로 통일했다.** Rust는 mono/stereo/multi 맵이
`is_empty()`인지만 봤고 Dart는 "활성 항목이 하나라도 있는가"를 봤다. 그래서 항목은 있지만 전부
`enabled: false`인 설정에서 UI는 모든 채널을 선택지로 제시하는데 엔진은 전부 음소거했다.
현재 데이터 모델에는 "설정된 적 있음" 플래그가 없어 "미설정"과 "전부 비활성"을 구분할 수 없으므로
Dart 규칙(활성 그룹 없으면 전체 개방)으로 맞췄다. **동작 변경이다**: 전부 비활성인 설정이
무음이 아니라 전체 개방이 된다. 채널을 실제로 닫는 방법은 다른 그룹을 열어 상대적으로 닫는 것이다.

**저장 전 장치 미리보기를 유지했다.** `outputChannelsProvider`는 저장된 `deviceName`만
구독하므로 마이그레이션만 하면 환경설정에서 장치를 고른 직후 채널 수가 갱신되지 않는다.
모달 안에서만 쓰는 미리보기 조회를 따로 두어 "저장하면 적용됩니다" 문구와 함께 보여준다.

## 5. 검증 계획

1. `cargo test --test test_enabled_channels_multi_config -- --nocapture` (A2)
2. `flutter test test/core/utils/channel_routing_test.dart`(신규): 사용자가 보고한 정확한 재현 케이스
   (4ch 파일, 2ch 장치, multiConfigs keys {1,2})를 표로 검증 — 두 항목이 partial-output 표기와 함께
   구별되게 생성되는지, `realChannel0`이 0/1인지 확인.
3. `flutter test test/core/utils/channel_routing_test.dart`에 **동일 입력을 두 번 호출해 리스트가
   `deepEquals`인지 확인하는 테스트**를 포함시켜 "생성기가 결정적이며 호출부와 무관하게 동일 결과"를
   기계적으로 보장한다.
4. `flutter test test/integration/channel_routing_sync_test.dart`(신규): 하나의 `ProviderContainer`에
   같은 `config`를 주입한 뒤, `preferences_modal`의 트랙 드롭다운에서 값을 변경하는 콜백을 호출하고
   `configProvider` 갱신 후 `track_card`를 재빌드해 두 위젯이 계산하는 라벨/값이 일치하는지 assert —
   "한쪽에서 바꾼 값이 다른 쪽에 반영되는가"의 기계적 검증.
5. `flutter analyze` 0 issues, `cargo clippy` 0 warnings.

### 실제 검증 결과

| 항목 | 계획 | 결과 |
|---|---|---|
| 1 | `cargo test --test test_enabled_channels_multi_config` | 9 passed |
| 2, 3 | `test/core/utils/channel_routing_test.dart` | 19 passed (결정성 테스트 포함) |
| 4 | 두 화면 라벨 일치 통합 테스트 | `test/integration/channel_label_consistency_test.dart` 4 passed |
| 5 | analyze 0 / clippy 0 | **미달.** `flutter analyze` 10 issues로 유지(전부 사전 존재: dashboard_screen 미사용 import 3건, cargokit lint 경로, deprecated activeColor 2건, 테스트 print 4건). 이 작업으로 늘지 않았음 |
| 6 | 실기 수동 QA | **미실행.** 실제 다채널 인터페이스 필요 |

추가로 들어간 검증:
- `rust/tests/test_mono_to_stereo_upmix.rs`: 모노→스테레오 업믹스가 출력 게이트를 지키고
  좌우 레벨이 대칭인지. 두 결함을 각각 독립적으로 잡는 것을 확인함(게인만 되돌리면 시나리오 1
  실패, 게이트만 되돌리면 시나리오 2 실패).
- `rust/tests/test_device_error_message_contract.rs`: 장치 부재 오류 메시지가 Dart 매처와
  어긋나지 않도록 문자열을 고정.
6. 수동 QA: 실제 2ch 장치로 전환 후 저장된 4ch 트랙 매핑을 열어 환경설정/메인화면 두 화면 모두
   동일한 부분 출력 경고가 보이는지, 장치 핫플러그 시 앱 재시작 없이 두 화면이 함께 갱신되는지 확인.
