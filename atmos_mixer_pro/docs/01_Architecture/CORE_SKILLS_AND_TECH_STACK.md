# 🛠️ Atmos Mixer Pro: 핵심 기술 스택 및 엔지니어링 스킬셋 (Core Skills & Tech Stack)

본 문서는 **Atmos Mixer Pro** 개발에 적용된 핵심 오디오 DSP 알고리즘, 실시간 시스템 설계, 3D 그래픽스, FFI 바인딩, 그리고 무결점 검증 프로토콜을 체계적으로 집대성한 기술 문서입니다. Claude Code 및 모든 개발자가 프로젝트의 엔지니어링 표준을 한눈에 파악할 수 있도록 작성되었습니다.

---

## 1. 🎧 프로 오디오 DSP & 수학적 신호 처리 (Audio DSP & Math Skills)

### ① 3D 공간 음향 패닝 (DBAP - Distance-Based Amplitude Panning)
- **적용 기술:** 불규칙 다채널(8ch ~ 64ch) 스피커 배치를 위한 3D 거리 역제곱 기반 진폭 패닝.
- **수학적 모델:** 스피커 $i$와 가상 음원 간의 유클리드 거리 $d_i = \sqrt{(x - x_i)^2 + (y - y_i)^2 + (z - z_i)^2}$를 계산하고, 롤오프 계수($a$)를 적용하여 에너지 보존 법칙($\sum g_i^2 = 1$)에 따라 게인을 정규화:
  $$g_i = \frac{d_i^{-a}}{\sqrt{\sum_{k=1}^N d_k^{-2a}}}$$
- **구현 특징:** 스피커 이동 및 음원 이동 시 지퍼 노이즈를 방지하기 위해 샘플 단위 선형 보간(Linear Smoothing) 적용.

### ② SOFA (AES69) 표준 3D 바이노럴 HRTF 렌더링
- **적용 라이브러리:** `sofa-reader`, `rubato` (샘플레이트 변환)
- **적용 기술:** 3D 공간 음향 현장 시공 전, 작업자가 헤드폰만으로 3D 공간 입체 정위감을 실시간 바이노럴(Binaural) 청음할 수 있도록 지원.
- **구현 특징:** KEMAR / CIPIC 측정 데이터 및 임의의 SOFA 파일 로딩, 수평각(Azimuth) 및 고도각(Elevation) 최근접/삼선형 보간 필터링.

### ③ Linkwitz-Riley 24dB/oct (LR4) 크로스오버 & 베이스 매니지먼트
- **적용 기술:** 2차 Butterworth 필터 2개를 직렬 종속(Cascaded) 연결하여 24dB/octave 슬로프 형성.
  - 새틀라이트 스피커(1~N채널): $f_c$ (60Hz ~ 120Hz) High-Pass 필터링
  - 서브우퍼(LFE, .1채널): **같은 방** 메인 채널의 저역 성분 합산 후 $f_c$ Low-Pass 필터링(방별 서브, 채널 DSP 앞에서 분할 — `CROSSOVER_LR24_SPEC.md` §3)
- **위상 특성:** 크로스오버 주파수에서 음압 평탄도 $0\text{dB}$ 범프 유지 및 360도(동위상 $0^\circ$) 위상 정합 보장.

### ④ EBU R128 / ITU-R BS.1770-4 라우드니스 & 트루 피크 미터링
- **적용 라이브러리:** `ebur128`
- **측정 항목:**
  - Integrated Loudness (LUFS / LKFS - 장시간 통합 음량)
  - Short-Term (3초) / Momentary (400ms) 동적 음량
  - True-Peak (dBTP - 4x/8x FIR 오버샘플링을 통한 인터샘플 클리핑 감지)

### ⑤ 3D 소리 궤적(Trajectory) 오토메이션 & 스플라인 보간
- **적용 알고리즘:** Catmull-Rom Spline (등속도 곡선 보간)
- **실시간 연산:** 3D 웨이포인트 노드 사이를 지날 때 이동 시간(Duration)과 곡선에 맞춰 매 오디오 버퍼마다 현재 $(X, Y, Z)$ 좌표를 $0\%$ 지연으로 도출하여 DBAP 엔진에 전달.

---

## 2. ⚡ 실시간 시스템 & 오디오 스레드 엔지니어링 (Real-time Audio Skills)

### ① 실시간 3대 불변법칙 (Real-time Safety)
1. **Zero-Allocation:** 실시간 콜백 루프(`process()`, `process_interleaved()`) 내 `Vec::new()`, `vec![]`, `Box::new()`, `String` 등 모든 동적 힙 메모리 할당 원천 금지. 모든 버퍼는 `new()` 생성자에서 사전 할당(Pre-allocation).
2. **Lock-Free Concurrency:** 오디오 스레드 내 `Mutex::lock()`, `RwLock`, `println!()`, 디스크 파일 I/O 절대 금지.
3. **Parameter Smoothing (Lerp):** 60fps UI/OSC 파라미터 유입 시 즉시 스냅 금지, 오디오 버퍼 길이에 맞춘 지수/선형 감쇠 스무딩 적용.

### ② 고성능 락-프리 동기화 도구
- **`rtrb`:** 단일 생산자-단일 소비자(SPSC) 락-프리 링 버퍼를 통한 메인 스레드 ⇄ 오디오 스레드 간 무지연 데이터 전송.
- **`Atomic` 프리미티브:** `AtomicU32`, `AtomicBool`을 사용한 원자적 볼륨, 뮤트, 재생 상태 스왑.
- **`cpal` (CoreAudio / ASIO):** macOS 저지연 CoreAudio 및 Windows 저지연 ASIO 네이티브 백엔드 제어.

---

## 3. 🌉 크로스 플랫폼 & 프론트엔드 연동 (Frontend & FFI Skills)

### ① Rust ⇄ Flutter 고속 FFI 브릿지
- **도구:** `flutter_rust_bridge` (v2.12.0)
- **메모리 정렬:** C 호환 바이트 정렬(`#[repr(C)]`) 구조체 설계로 언어 간 메모리 복사 최소화.
- **수명 주기 통일:** 메모리 할당 및 해제 책임을 Rust 엔진에 일원화하여 다트 GC(가비지 컬렉터)와의 충돌 및 메모리 릭 원천 방지.

### ② Flutter 60fps/120Hz 고주파 캔버스 렌더링 최적화
- **상태 관리:** Riverpod를 활용한 세분화된 상태 구독.
- **Decoupled 렌더링:** 빈번한 60fps 애니메이션 프레임(오디오 궤적 이동, 레벨 미터) 시 전체 위젯 트리가 리빌드되지 않도록 `Listenable`, `ValueNotifier`, `RepaintBoundary`를 사용해 페인팅 영역 격리.
- **3D 방 및 스피커 투영 (`CustomPaint` + `Matrix4`):** 모델 파일 없이 코드로 4x4 바닥 격자와 스피커 큐브를 3차원 투영 렌더링.

### ③ 3D 마네킹 헤드 및 Three.js/WebGL 시뮬레이터
- **3D 리스너 모델:** 유광 화이트 마네킹 헤드(`listener_head.glb`, `listener_mannequin.glb`) 렌더링.
- **기준 좌표계:** 방 정중앙 바닥으로부터 인체 표준 좌식 귀 높이 **$1.2\text{m}$** (`W/2`, `D/2`, `1.2m`)에 귀 기준점(`listenerGroup`) 배치.
- **스피커 방사각(Aiming) 자동 연산:** 천장 및 벽면 스피커의 Pitch/Yaw 각도를 $1.2\text{m}$ 귀 위치를 향해 조준하도록 삼각함수 벡터 유도.

---

## 4. 🌐 전시장 하드웨어 및 네트워크 통신 (Protocol Skills)

### ① OSC (Open Sound Control) 실시간 제어
- **적용 라이브러리:** `rosc`, UDP 소켓 통신
- **주요 명령어 명세:**
  - `/track/{id}/play`, `/track/{id}/stop`, `/track/{id}/reset`
  - `/track/{id}/pos {x} {y} {z}` (외부 트래커를 통한 실시간 소리 객체 위치 제어)
  - 미디어 서버(Resolume, TouchDesigner, QLab) 연동 지원.

### ② 데이터 직렬화 및 영속성 (Persistence)
- **설정 파일:** `config.json`
- **구조적 정합성:** Rust의 `AppConfig` 구조체와 Dart 모델 간 1:1 필드 매핑 및 무결점 역직렬화 보장.

---

## 5. 🔬 결함 제로(Zero-Defect) 검증 프로토콜 (QA & Verification Skills)

### ① 기계적 정답 판정 (Mock Signal Injection)
- **1kHz Sine Wave 주입:** 1kHz Peak Bin이 정확히 0dBFS이고 비대상 대역이 -140dBFS 이하인지 `cargo test` 자동 검증.
- **Digital Silence (0.0) 주입:** 무음 입력 시 출력이 -140dBFS로 안정화되고 CPU 데노멀(Denormal) 현상이 없는지 검증.
- **Impulse (1.0, 0.0...) 주입:** 필터의 임펄스 응답 반응 곡선 정합성 검증.
- **허용 오차:** 물리 수치 오차 $0.001\%$ 미만 유지.

### ② 7단계 순환 루프 (Automated 7-Step QA Loop)
1. **구현 (Implementation)**
2. **리뷰 요청 (Review Request)**
3. **코드 감사 (Code Audit against 3 DSP Laws)**
4. **엄격한 테스트 (Testing via `cargo test`, `flutter analyze`)**
5. **감독관 피드백 (Supervision & Issue Ordering)**
6. **무한 수정 루프 (Fix & Loop until 100% flawless)**
7. **최종 보고 (Proof-of-Execution Report)**
