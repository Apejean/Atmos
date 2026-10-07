---
paths:
  - "atmos_mixer_pro/rust/**"
  - "atmos_mixer_pro/supervisor/**"
---

# Rust 엔진 규칙

- 오디오 스레드 3법칙(CLAUDE.md)을 리뷰 때 직접 확인한다. 핫루프 안의 `vec!`·`push`·`clone`·`Box::new`·`String`·`format!`·`lock()`·`println!`/`eprintln!`을 grep으로 본다. `docs/HANDOFF.md` 남은 일 8의 "기존 3법칙 위반" 목록은 알려진 것이니 새로 늘리지 않는다.
- 파라미터 변경은 샘플 단위 보간(lerp·스무딩). 순간 변경은 딸깍 잡음이 난다.
- DBAP는 `Σg² = 1`, 크로스오버는 80Hz LR24(확정, 다시 제안하지 않음).
- `rust/src/api/`를 바꾸면 Dart 바인딩 재생성이 필요하고 API 계약 변경이다. Sub는 Main에 먼저 알린다.
- Windows 전용 경로(ASIO, `#[cfg(windows)]`)는 macOS에서 컴파일·실행이 확인되지 않는다. 바꾸면 HANDOFF의 "⚠️ Windows 미검증"에 남긴다.
- 테스트는 `rust/tests/`에 파일 단위로 둔다. 수동 진단(`diag_*`, `#[ignore]`)과 조사용 바이너리(`cargo run --bin mixer_bench`, `probe_cpal`)는 자동 테스트가 아니다.
- 전체 `cargo test`는 디스크 여유가 적으면 ENOSPC로 실패한다. 먼저 `--test <이름>`으로 좁혀 돌린다.
