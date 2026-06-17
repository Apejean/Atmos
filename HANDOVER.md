# Atmos Mixer Pro - Agent Handover Document

이 문서는 Mac 환경에서 작업하던 기존 Antigravity 에이전트가 Windows 환경에서 새로 시작될 Antigravity 에이전트에게 프로젝트의 문맥(Context), 아키텍처, 그리고 작업 히스토리를 인계하기 위한 문서입니다.

**새로운 에이전트에게:** 이 문서를 읽었다면, 당신은 기존에 완료된 모든 버그 수정 및 시스템 아키텍처를 이해한 상태에서 작업을 이어나가야 합니다.

## 1. 프로젝트 개요
* **이름:** Atmos Mixer Pro
* **목적:** 방 탈출 및 테마파크용 다중 룸 오디오 믹싱 및 제어 시스템
* **기술 스택:** Flutter (Frontend) + Rust (Backend, Audio Engine) + `flutter_rust_bridge` (FFI)

## 2. 주요 아키텍처 및 상태 관리
* **오디오 엔진 (`rust/src/audio/mixer.rs`):**
  * CoreAudio(Mac) / WASAPI(Windows)를 통해 다채널 오디오 인터페이스(예: Scarlett 6i6)로 오디오를 출력합니다.
  * 오디오 렌더링 스레드 내부에서는 절대 `Mutex` 락을 오랫동안 잡고 있으면 안 됩니다 (UI 렉 및 버튼 씹힘 현상 방지).
  * 트랙 상태 변경은 큐(Channel)가 아닌 `StreamSink`를 통해 다이렉트로 메모리에 주입되는 방식을 사용합니다.
* **상태 동기화 (`rust/src/core/state.rs`):**
  * `GLOBAL_STATE` 객체가 싱글톤으로 시스템 상태(재생 중인 트랙, 활성 룸, 설정 등)를 관리합니다.
  * 프론트엔드는 Optimistic UI 업데이트를 하지 않으며, 오직 Rust 백엔드에서 쏘아주는 상태 스트림만 구독(Listen)하여 UI를 렌더링합니다. (Single Source of Truth)
* **네트워크 통신 (`rust/src/osc/listener.rs`):**
  * 아두이노 등 외부 장치로부터 OSC(Open Sound Control) 프로토콜을 통해 트리거 신호(테마 시작, 룸 클리어, 시스템 리셋 등)를 수신합니다.
  * 데드락 방지를 위해 락(Lock)을 획득하는 스코프(Scope)를 최소화했습니다.

## 3. 완료된 주요 디버깅 내역 (3-Pass Deep Debugging)
이전 에이전트는 다음과 같은 치명적 버그 및 엣지 케이스들을 모두 수정했습니다. 절대 이 수정 사항들을 원복(Regression)시키지 마십시오.
* **버튼 씹힘 현상:** 렌더링 스레드의 락 경합을 해결하여 OSC 및 UI 버튼 클릭 무시 현상 제거.
* **BGM 중복 재생:** `api_clear_room`에 Idempotency(멱등성) 락을 구현하여 룸 클리어 중복 트리거 방어.
* **렌더링 글리치:** 룸 이름 등 `TextField` 입력 시 `onChanged` 대신 `FocusNode` 및 `onSubmitted`를 사용하여 디스크 I/O 과부하 해결.
* **패닉(Crash) 방어:** 
  * 존재하지 않는 오디오 채널 번호(Out-of-bounds) 할당 시 강제 종료 방어.
  * 환경설정(`config.json`) 파싱 실패 시 데이터를 날리지 않고 `.corrupted.json`으로 백업하는 로직 추가.
  * 일부 오디오 파일 누락 시에도 앱 전체 로딩이 마비되지 않도록 예외 처리.

## 4. 진행 상황 및 윈도우 환경 주의사항
* 크로스 플랫폼 빌드를 위해 GitHub Actions(`build_release.yml`)가 세팅되어 있습니다.
* Windows 환경에서는 `wasapi` 백엔드(cpal)를 통해 오디오가 렌더링됩니다. Mac 환경의 CoreAudio와는 디바이스 탐색 및 채널 할당 방식이 미세하게 다를 수 있으므로, 하드웨어 디바이스 인식 관련 버그가 발생한다면 `rust/src/api/simple.rs`의 디바이스 조회 로직을 주의 깊게 살펴보십시오.

## 5. 새로운 에이전트의 행동 지침
1. 사용자가 Windows 환경의 터미널 및 파일 경로(`C:\...`)를 제시할 것입니다. Windows 문맥에 맞게 명령어 및 경로를 해석하십시오.
2. 기능 추가나 디버깅 요청이 들어오면, 이 문서에 기재된 아키텍처 원칙(락 최소화, StreamSink 사용, 프론트엔드 상태 추종)을 반드시 지켜서 코드를 작성하십시오.
3. 준비가 되었다면 사용자에게 "이전 에이전트의 인수인계 문서를 확인했으며, Windows 환경에서 작업을 이어나갈 준비가 되었습니다"라고 보고하십시오.
