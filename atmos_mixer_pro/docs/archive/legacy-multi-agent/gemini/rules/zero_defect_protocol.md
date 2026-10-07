# [개발 지침서] 상용급 고신뢰성 오디오/UI 시스템 결함 제로(Zero-Defect) 검증 규격서

## 1. 🎯 기본 철학 (Core Philosophy)
"컴파일 성공(Syntax Check)"은 절대로 "기능 완성"이 아니다. `cargo check` 및 `flutter analyze` 통과는 1단계 문법 검사일 뿐이며, 실제 런타임 데이터 인입 -> 연산 -> FFI 전달 -> UI 출력 파이프라인의 전체 통합 테스트(E2E)를 통과하기 전까지는 "완성"이라 칭하지 않는다.

오픈소스 및 외부 자료 이식 시 "단독 격리 검증" 후 결합한다. 깃허브/구글 등에서 가져온 외부 기술은 메인 아키텍처에 바로 결합하지 않고, 독립된 단위 테스트 환경에서 먼저 기계적 입출력 스펙을 정밀 검증한다.

---

## 2. 🚀 검증된 오픈소스 & 학술 기술 연동 고도화 로드맵 (Open-Source Core Integration)

본 프로젝트(Atmos Mixer Pro)의 완성도를 세계 최고급(Dolby Atmos / IEM Graz / L-Acoustics Soundvision 레벨)으로 끌어올리기 위해 아래 검증된 오픈소스 및 학술 라이브러리를 표준 모듈로 채택한다.

### ① EBU R128 / ITU-R BS.1770-4 라우드니스 메터링 (`ebur128` Crate)
* **검증 출처:** C `libebur128` 포팅 Rust 크레이트 (EBU R128 국제 방송 음량 표준)
* **적용 기능:**
  - Integrated Loudness (LUFS / LKFS - 장시간 통합 음량)
  - Short-Term (3초) / Momentary (400ms) 동적 음량 시각화
  - True Peak (dBTP - 오버샘플링 트루 피크 감지)
* **효과:** 전시장 및 현장에서 과도한 소리 크기로 인한 스피커 파손 차단 및 방송/전시 표준 LUFS 음량 자동 맞춤.

### ② SOFA (AES69) 표준 3D 바이노럴 헤드폰 모니터링 (`libmysofa` / `SAF`)
* **검증 출처:** AES69 SOFA (Spatially Oriented Format for Acoustics) 학술 표준 및 SAF (Spatial Audio Framework)
* **적용 기능:**
  - 3D 공간 음향 현장 세팅 전, 작업자가 헤드폰만 끼고 **3D 공간 입체 정위감을 실시간 바이노럴(Binaural) 청음 모니터링**.
  - KEMAR / CIPIC HRTF 프로필 연동 및 임의의 SOFA 측정 파일 로딩 지원.

### ③ IEM Graz / DBAP & HOA 3D 스피커 패닝 (`IEM Plug-in Suite`)
* **검증 출처:** 오스트리아 그라츠 음악공연예술대학교 음향연구소 (IEM Graz - Institute of Electronic Music and Acoustics)
* **적용 기능:**
  - 8채널 ~ 64채널 다채널 불규칙 스피커 배열을 위한 3D DBAP (Distance-Based Amplitude Panning) 연산.
  - High-Order Ambisonics (HOA) 3D 입체 음향 렌더링 엔진 완비.

### ④ True-Peak Oversampling Lookahead Limiter (`fundsp` / `Cytomic`)
* **검증 출처:** Cytomic ZDF 및 `fundsp` Lookahead Dynamics Engine
* **적용 기능:**
  - 4x / 8x FIR 인터폴레이션 오버샘플링 적용.
  - 24채널 합산 출력 시 발생하는 Inter-sample Peak(샘플 간 피크) 왜곡 100% 방지.

---

## 3. 🔬 5단계 정밀 무결성 검증 공정 (5-Step Quality Assurance Pipeline)

### [1단계] 오픈소스 알고리즘 격리 검증 (Isolated Unit Testing)
- **목적**: 외부 라이브러리(`ebur128`, `rustfft`, `rtrb`, `rosc` 등) 및 외부 코드 이식 시 발생할 수 있는 독자적 오류 차단.
- **이행 지침**:
  - 메인 믹서/엔진 코드에 결합하기 전, `tests/test_isolated_feature.rs` 독립 테스트 파일 작성.
  - 오리지널 알고리즘의 입력값 대비 출력 수치를 기계적으로 검증 (`assert_eq!`, `assert_near!`).
  - 알고리즘 내부의 엣지 케이스(예: 0.0, NaN, ∞, 무한소 Denormal) 발생 여부를 독립 환경에서 선제 격리 및 수선.

### [2단계] 실시간 오디오 스레드 제약 조건 감사 (Real-time Thread Safety Audit)
- **목적**: 48kHz / 128 샘플 (2.67ms) 실시간 오디오 루프 내의 지연(Jitter) 및 소리 끊김(Stutter) 100% 방지.
- **이행 지침 (3대 금기 사항 전수 조사)**:
  - **동적 메모리 할당 절대 금지**: 오디오 콜백 루프 내부에서 `vec![]`, `Box::new()`, `String` 등 힙 할당 금지. (모든 버퍼는 Pre-allocation 기법으로 사전 생성)
  - **스레드 락 블로킹 금지**: 오디오 스레드 내에서 `Mutex::lock()` 사용 금지. (스레드 간 통신은 `rtrb` 락-프리 링버퍼 또는 Atomic Pointers 필수 사용)
  - **I/O 및 출력 디버그 금지**: 오디오 콜백 내부 `println!`, 파일 I/O 연산 배치 금지.

### [3단계] 정밀 모의 신호 주입 테스트 (Mock Signal Injection & Assertion)
- **목적**: 눈대중 검사가 아닌, 컴퓨터에 의한 100% 기계적 정답 판정.
- **이행 지침**:
  - **1kHz Sine Wave (정현파) 주입**: RTA/DSP 모듈에 1kHz 신호 주입 시, 1kHz Peak Bin 수치가 정확히 0dBFS이며 나머지 대역이 -140dBFS 이하로 내려가는지 `cargo test` 자동 판정.
  - **Digital Silence (0.0) 주입**: 입력이 무음일 때 출력이 정확히 -140dBFS로 안정화되고, CPU 점유율이 폭증하는 Denormal Number 현상이 없는지 검증.
  - **Impulse (1.0, 0.0, 0.0…) 주입**: Biquad/SVF 필터 적용 시 임펄스 응답 반응 곡선 연산 검증.

### [4단계] Rust ⇄ Dart FFI 메모리 패킹 검증 (Cross-Language Alignment)
- **목적**: 언어 간 메모리 주소 및 구조체 이종 정렬로 인한 런타임 렉/크래시 방지.
- **이행 지침**:
  - Rust 단 C 연동 구조체에 반드시 `#[repr(C)]` 명시.
  - Rust 구조체의 포인터 크기/바이트 정렬(Byte Alignment)과 Dart FFI `ffi.Struct` 데이터 타입 크기가 100% 일치하는지 FFI 단독 라운드트립 테스트 수행.
  - 메모리 해제(Deallocation) 주체가 오직 Rust 엔진 단에서만 이루어지도록 메모리 라이프사이클 통일.

### [5단계] 런타임 시뮬레이션 로그 및 증적서 제출 (Proof-of-Execution)
- **목적**: "다 됐다"는 구두 보고 배제 및 정직한 증적 기반 완료 선언.
- **이행 지침**:
  - 기능 개발 완료 보고 시, 아래 3가지 증적을 반드시 보고서에 첨부해야 완료로 승인함.
    - `cargo test --workspace` 실행 결과 로그 (0 Failure).
    - `flutter analyze` 실행 결과 로그 (0 Issue).
    - 모의 데이터 주입 시 실제로 산출된 물리 수치/출력 결과표.

---

## 4. 📝 기능 완료 검증 체크리스트 (Definition of Done)
개발자는 각 기능을 완료했다고 선언하기 전, 아래 체크리스트를 100% 충족했는지 서명하여 제출한다.

- [ ] **[단위 테스트]** 오픈소스/외부 알고리즘 단독 격리 테스트 통과 여부
- [ ] **[오디오 스레드]** 오디오 콜백 내 malloc, Mutex, println! 미사용 확인 여부
- [ ] **[오디오 미터링]** EBU R128 LUFS & True-Peak 오버샘플링 검증 여부
- [ ] **[수치 검증]** 모의 신호(Sine/Silence/Impulse) 주입 결과 수치 오차 0.001% 이내 확인 여부
- [ ] **[FFI 정렬]** Rust ⇄ Dart 메모리 구조체 크기 및 바이트 포인터 일치 확인 여부
- [ ] **[통합 테스트]** cargo test 및 flutter analyze 0 에러 / 0 경고 통과 증적 첨부 여부

위 규격서의 절차와 체크리스트를 100% 통과한 코드만이 프로젝트의 프로덕션 브랜치에 병합(Merge) 및 최종 완성으로 인정됩니다.
