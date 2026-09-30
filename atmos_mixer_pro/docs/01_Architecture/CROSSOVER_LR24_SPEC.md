# Linkwitz-Riley 24dB/oct (LR4) Crossover Architecture

## 1. Overview
To support Bass Management and Subwoofer (LFE) integration in the 3D Atmos Mixer, we need to implement Linkwitz-Riley 24dB/octave (LR4) crossover filters in the Rust DSP backend.

## 2. DSP Implementation (Rust)
The LR4 filter is formed by cascading two 2nd-order Butterworth filters.
- **Low-Pass Filter (LPF) for Subwoofer:** `Butterworth_LPF(fc) -> Butterworth_LPF(fc)`
- **High-Pass Filter (HPF) for Main Speakers:** `Butterworth_HPF(fc) -> Butterworth_HPF(fc)`

When these two outputs are summed acoustically, the magnitude response is completely flat (0dB bump at crossover frequency), and the phase response of both bands remains in-phase (360-degree difference = 0-degree).

## 3. Bass Management Routing (Dart -> Rust) — 방별(2026-09)
- **서브 지정은 스피커 속성**(`SpeakerNode.isSubwoofer`, 방마다 하나). 공간 설정 payload의
  `is_subwoofer` + `room_id`로 엔진이 라우팅 표를 만든다(`rust/src/audio/bass_route.rs`).
- **크로스오버 주파수**와 **LFE +10dB**는 모든 방 공통(`crossoverFrequency`, 기본 80Hz).
- **메인 채널:** 자기 방에 서브가 있으면 LR4로 가르고, 고역만 자기 채널에 남긴다. 서브가 없는
  방은 풀레인지(다른 방 서브로 보내지 않는다).
- **분할 위치(표준 순서):** 채널 DSP(시간 정렬 딜레이·EQ·게인·극성·초기반사) **앞에서** 가른다.
  각 스피커의 보정은 그 스피커가 실제로 내는 소리에만 걸리고, 서브로 넘어간 저역은 서브 채널의
  보정만 받는다. (예전에는 DSP 뒤에서 갈라 메인의 로우컷·게인이 서브 저역에 한 번 더 걸렸고,
  극성이 뒤집힌 메인의 저역이 서브에서 다른 메인의 같은 저역을 지웠다.)
- **서브 채널 출력:** `LPF120(자기 신호 = .1 LFE 트랙) × (LFE +10dB 토글) + 같은 방 메인에서
  넘어온 저역` → 서브 채널 DSP(딜레이·EQ·게인·극성). 서브는 초기반사·리버브를 쓰지 않는다.
- **극성:** 서브가 있는 방의 메인은 위치만 보고 자동으로 뒤집지 않는다(크로스오버에서 서브와
  반대 극성이 되면 그 대역이 지워진다).
- **헤드폰 미리듣기:** 바이노럴은 스피커→청취 지점 전파(거리/c 지연, 기준거리/거리 감쇠)를
  흉내 낸다. 그래야 자동 튜닝의 시간 정렬·거리 게인이 현장처럼 상쇄되어 서브와 메인이 제 시각·
  제 크기로 합쳐진다(`binaural.rs` Propagation).

## 4. UI Integration
- Add a "Bass Management" toggle in the Tuning Modal.
- Add a Crossover Frequency slider (60Hz ~ 120Hz).
