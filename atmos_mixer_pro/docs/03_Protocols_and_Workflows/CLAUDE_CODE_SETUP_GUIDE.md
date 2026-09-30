# 🚀 Claude Code 전환 및 작업 시작 가이드 (Atmos Mixer Pro)

본 문서는 **Atmos Mixer Pro** 프로젝트 개발 환경을 **Claude Code(CLI)**로 완전히 전환하여 작업할 때 필요한 모든 설정, 자동화 명령어, 그리고 필수 프로토콜을 정리한 공식 가이드입니다.

---

## 1. 📂 사전 구축된 핵심 설정 파일

이미 프로젝트 루트에 다음 두 파일이 완벽하게 생성되어 배포되었습니다:

1. **`CLAUDE.md`** (프로젝트 루트 및 상위 리포지토리)
   - Claude Code가 실행되자마자 프로젝트의 성격(B2B 실시간 3D 공간음향 DAW)을 100% 인지합니다.
   - **오디오 스레드 3대 불변법칙**(Zero-Allocation, Lock-Free, Sample-Accurate Lerp)을 강제합니다.
   - **7단계 자동 검증 루프** 및 빌드/테스트 명령어(`cargo test`, `flutter analyze` 등)를 탑재했습니다.

2. **`.claudeignore`** (프로젝트 루트 및 상위 리포지토리)
   - `build/`, `target/`, `.dart_tool/`, `*.wav` (대용량 오디오), `*.dylib`, 루트의 테스트 바이너리 등을 차단하여 **토큰 낭비를 최대 80% 이상 절감**합니다.

---

## 2. ⚡ Claude Code 시작 및 권장 명령어

### A. 일반 작업 실행 (추천: Claude 3.7 Sonnet)
UI 위젯 개발, 개별 DSP 튜닝, 일반적인 버그 수정 시 사용합니다.
```bash
cd /Users/Allweno/Projects/GitHub/atmos/atmos_mixer_pro

# 기본 실행 (3.7 Sonnet)
claude
```

### B. 대규모 리팩토링 및 7단계 무결점 QA 실행 (추천: Claude Fable 5.1)
새로운 오디오 엔진 모듈 설계, 다채널 라우팅 구조 변경, 수 시간 단위의 자율 검증 시 사용합니다.
```bash
# Fable 5.1 모델 지정 실행
claude --model claude-fable-5-1
```

---

## 3. 🧩 추천 MCP(Model Context Protocol) 확장 도구 세팅

터미널에서 아래 명령어를 한 번씩 실행하여 Claude Code에 유용한 확장 서버를 등록합니다.

```bash
# 1. 복잡한 DSP 수학 및 오디오 파이프라인 심층 추론 도구
claude mcp add sequential-thinking npx -y @modelcontextprotocol/server-sequential-thinking

# 2. 최신 Flutter/Rust 라이브러리 및 공식 문서 실시간 조회
claude mcp add fetch npx -y @modelcontextprotocol/server-fetch

# 3. 로컬 Git 이력 및 브랜치 상태 정밀 파싱
claude mcp add git npx -y @modelcontextprotocol/server-git --repository /Users/Allweno/Projects/GitHub/atmos
```

---

## 4. 🔄 Claude Code 내에서 7단계 워크플로우 운영 지침

기존에는 다중 서브에이전트(`@Front`, `@Back`, `@check`, `@sub`) 간 메시지를 교환했으나, **Claude Code 단일 세션 내에서도 완벽하게 동일한 7단계를 강제**할 수 있습니다.

### Claude Code에게 지시하는 표준 프롬프트 템플릿:
```text
"다음 작업을 진행해 줘: [작업 내용].
작업 완료 전 반드시 CLAUDE.md의 7단계 검증 루프를 수행해:
1. 코드 구현 후 자체 오디오 3대 규칙(Heap할당/락 금지)을 감사해.
2. `cd rust && cargo test -- --nocapture` 및 `flutter analyze`를 실행해서 오류 0건을 확인해.
3. 100% 무결점 증적 로그를 첨부해서 보고해 줘."
```

---

## 5. 🛡️ 오디오 엔지니어링 즉시 검증 단축 스크립트

Claude Code에서 빠르게 DSP 무결성을 검증할 수 있도록 유용한 검증 명령어를 숙지해 두세요:

```bash
# 1. Rust DSP 단위 테스트 전체 실행
cd rust && cargo test -- --nocapture

# 2. 오디오 콜백 힙 할당 / Mutex 사용 여부 정적 검색
grep -rn "Vec::new" rust/src/
grep -rn "vec!\[" rust/src/
grep -rn "Mutex" rust/src/

# 3. Flutter 린트 및 문법 정적 분석
flutter analyze
```

---

## 6. 💡 토큰 및 비용 절감 3대 수칙

1. **세션 분리:** 기능 개발이 하나 끝나면 터미널에서 `/clear` 또는 새 터미널을 열어 이전 대화 컨텍스트를 비워줍니다.
2. **요약 활용:** 긴 대화 도중 컨텍스트가 너무 커지면 `/compact` 명령을 입력해 이전 내용을 요약 압축합니다.
3. **불필요한 바이너리 탐색 방지:** `test_*.rs`나 오디오 바이너리는 직접 경로(`rust/src/...`)를 명시해 주는 것이 탐색 토큰을 아끼는 데 유리합니다.
