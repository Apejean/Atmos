# The Awesome (Atmos Mixer Pro)

몰입형 전시용 PCM 오디오 분배·운영 프로그램. 미리 만든 PCM 오디오를 전시장 출력 채널에 안정적으로 분배·운영하고 스피커를 보정·관리한다. 오브젝트를 실시간으로 배치하는 렌더러가 아니다. 사람이 없는 무인 운영이 기본이고 오디오 끊김은 허용되지 않는다.

- 설계·개발은 macOS, **현장 운영은 Windows**(ASIO/WASAPI). macOS에서 확인한 것을 현장 보증으로 보지 않는다.
- 스택: Flutter(macOS·Windows 데스크톱, Riverpod) + Rust 2021 오디오 엔진(cpal 0.16, rtrb) + flutter_rust_bridge 2.12.0.

## 폴더

| 폴더 | 내용 |
|---|---|
| `lib/` | 화면(Dart) |
| `rust/` | 오디오 엔진(`rust_lib_atmos_mixer_pro`) |
| `supervisor/` | 충돌 뒤 다시 띄우는 감시 프로그램(`atmos_supervisor`) |
| `integration_test/`, `test/`, `rust/tests/` | 자동 테스트 |
| `tool/` | 빌드·법률 문서 생성·Windows 하네스 스크립트 |
| `windows/`, `macos/` | 플랫폼 러너와 설치 파일 |
| `docs/` | 문서 |

## 명령

```bash
cd rust && cargo test -- --nocapture   # 전체 Rust 테스트(디스크 여유 수 GB 필요)
cd rust && cargo clippy
flutter analyze                        # 0건 유지
flutter test
flutter_rust_bridge_codegen generate   # rust/src/api/ 를 바꿨을 때만
```

## 문서 읽는 순서

1. 저장소 루트 `CLAUDE.md`(규칙·작업 방식)
2. `docs/HANDOFF.md`(현재 상태·확정된 결정·남은 일의 기준)
3. 작업과 관련된 문서: `docs/WINDOWS_FIELD_CHECKLIST.md`, `docs/windows/`, `docs/03_Protocols_and_Workflows/OSC_INTERFACE_SPEC.md`, `docs/superpowers/specs/`

## 라이선스

사유 소프트웨어다. 사용 조건은 최종 사용자 라이선스 계약(`windows/legal/`, 지금은 검토용 초안)을 따른다. 제3자 구성요소의 고지는 빌드 때 `tool/legal/generate_third_party_licenses.dart`가 만들어 설치 폴더 `Legal`에 넣는다.
