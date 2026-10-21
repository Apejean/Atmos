# hrtf_convert (개발 전용)

앱에 내장하는 MIT KEMAR SOFA(`rust/assets/hrtf/mit_kemar_normal_pinna.sofa`)를 앱이 읽는
`HrtfSet` 바이너리(`rust/assets/hrtf/mit_kemar_normal_pinna.hrtfset`)로 한 번 옮기는 도구다.

- **제품과 분리.** 제품 크레이트(`rust/`)의 의존성, CI(`.github/workflows/build_release.yml`),
  설치 파일(`windows/installer.iss`) 어디에도 들어가지 않는다. 자체 `[workspace]`라 다른 크레이트와 엮이지 않는다.
- **sofa-reader 0.1.6(GPL-3.0-or-later)은 이 도구에서만 쓴다.** 제품은 SOFA 파서 없이 결과 파일만 읽는다.
  판은 걷어내기 전 제품이 쓰던 것에 고정했고, `Cargo.lock`도 그때 제품의 잠금 파일에서 시작했다.
- **숫자는 바꾸지 않는다.** `strict_load`가 읽은 f32를 비트 그대로 쓰고, 쓴 파일을 다시 읽어 값마다 비트를
  비교한다. 다르면 실패로 끝난다. 형식은 `rust/src/audio/hrtf_data.rs` 머리 주석.

## 실행 (Windows 개발 PC)

빌드 산출물이 저장소 밖에 생기도록 `CARGO_TARGET_DIR`을 정한다. `env.ps1`이 VS 개발자 환경을 넣는다.

```bash
powershell -NoProfile -ExecutionPolicy Bypass -File D:/dev/Atmos/atmos_mixer_pro/tool/windows/run.ps1 -Gate HRTF-convert -Cwd <저장소>\atmos_mixer_pro\tool\hrtf_convert -Run "set CARGO_TARGET_DIR=D:\dev\wt\target& cargo run --release -- ..\..\rust\assets\hrtf\mit_kemar_normal_pinna.sofa ..\..\rust\assets\hrtf\mit_kemar_normal_pinna.hrtfset"
```

출력: SOFA 전역 속성(출처 기록용), 측정점 수·IR 길이·샘플레이트, 비트 비교 결과. 결과 파일의 크기·SHA-256과
변환 기록은 `rust/assets/hrtf/README.md`에 남긴다.

결과 파일이 걷어내기 전 앱과 같은 결과를 내는지는 제품 쪽 `rust/tests/test_hrtf_data_parity.rs`가 확인한다.
위치·IR 데이터는 늘 비교하고, 최근접 조회·보정·렌더 결과는 같은 PC에서 만든 기준 덤프가 있어야 비교한다
(산술 결과는 OS·CPU마다 마지막 비트가 다를 수 있다).

## 다른 PC에서 기준 덤프 만들기 (예: macOS)

`baseline/diag_hrtf_dump.rs`는 걷어내기 전 코드에서 기준 덤프를 만드는 진단 테스트다(sofa-reader를 쓰므로
지금 제품에서는 컴파일되지 않는다 — 어느 빌드 대상에도 들어 있지 않다).

1. 걷어내기 전 커밋(c928226)을 임시 worktree로 꺼내 이 파일을 `rust/tests/`에 복사한다.
2. 그 worktree의 `rust/`에서 `HRTF_DUMP_DIR=<기준 폴더> cargo test --test diag_hrtf_dump -- --ignored --nocapture`.
3. 이 브랜치의 `rust/`에서 `HRTF_BASELINE_DIR=<기준 폴더> cargo test --test test_hrtf_data_parity -- --nocapture`.
   12개 덤프가 모두 같으면 통과한다.
