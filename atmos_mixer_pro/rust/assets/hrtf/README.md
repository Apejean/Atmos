# 내장 HRTF 데이터 (MIT KEMAR, normal pinna)

헤드폰 바이노럴 미리듣기(`src/audio/binaural.rs`)가 쓰는 측정 데이터다.

| 파일 | 쓰임 | 크기 | SHA-256 |
|---|---|---|---|
| `mit_kemar_normal_pinna.sofa` | 원본(SOFA, AES69 SimpleFreeFieldHRIR). 제품은 읽지 않는다 — 변환 도구의 입력 | 1,171,764바이트 | `e7035994f5fd754058424c061380ee92b1d5ed58fccef2887a4266916616acdf` |
| `mit_kemar_normal_pinna.hrtfset` | 제품이 `include_bytes!`로 내장해 `audio::hrtf_data`로 읽는다 | 2,916,708바이트 | `2425f195cfd0e704fb12113e2c96d68fb2c5a3bc43df1e0c37a82972bfbcc222` |

## 출처와 사용 조건

- 저작권: Copyright 1994 by the MIT Media Laboratory. 측정: Bill Gardner, Keith Martin(MIT Media Lab).
- 인용: Gardner, W. G., and Martin, K. D. (1995). "HRTF measurements of a KEMAR." Journal of the Acoustical Society of America, 97(6), 3907–3908.
- SOFA 파일에 적힌 사용 조건(전역 속성 `License` 원문):
  > This HRTF data is provided without any usage restrictions. It is requested that you cite Gardner and Martin (1995) when using this data for research or commercial applications.
- MIT 원본 페이지(http://sound.media.mit.edu/resources/KEMAR.html, 2026-10-22 확인): 데이터는 MIT Media Laboratory 저작물이며, 연구·상업 응용에 쓸 때 저자를 인용하는 조건으로 사용 제한 없이 무료로 제공된다고 적혀 있다.
- 앱 안의 고지: Help → Third-party Notices(`lib/features/settings/widgets/third_party_notices.dart`).
- 이 문서와 앱의 고지는 원본의 사용 조건을 옮긴 것이며, 그보다 넓은 권리를 주장하지 않는다.

## 변환 이력

1. MIT Media Lab의 원본 KEMAR 측정(1994, http://sound.media.mit.edu/resources/KEMAR.html).
2. Piotr Majdak, Michael Mihocic(Acoustics Research Institute, Austrian Academy of Sciences)가 SOFA로 변환
   (파일의 `History`: "Converted from the MIT database"; `SOFAConventions` SimpleFreeFieldHRIR 1.0,
   `APIName` ARI SOFA API for Matlab/Octave 1.1.1, `DateCreated` 1999-11-16 20:01:52, `DateModified` 2020-03-24 13:46:08).
3. Atmos 프로젝트가 자체 형식(`.hrtfset`, 형식은 `src/audio/hrtf_data.rs` 머리 주석)으로 추가 변환(2026-10-22).
   값은 바꾸지 않았다. 앱은 실행 중에 엔진 샘플레이트에 맞춰 보정한다(`src/audio/hrtf_eq.rs`).

## 변환 도구와 검증

- 도구: `tool/hrtf_convert`(개발 전용, 제품·CI·설치 파일에 들어가지 않음). sofa-reader 0.1.6(GPL-3.0-or-later)의
  `strict_load`로 읽은 값을 비트 그대로 쓰고, 쓴 파일을 다시 읽어 값마다 비트를 비교한다.
- 실행 기록: 2026-10-22, Windows 11 개발 PC, rustc 1.99.0(b940084d7 2026-09-28)·cargo 1.99.0, `cargo run --release`.
  도구의 의존성 판은 걷어내기 전 제품의 `Cargo.lock`과 같다(48개 패키지). 결과 "위치 2,130개·IR 727,040개 값이
  SOFA 읽기 결과와 비트 단위로 같음". 로그 `D:\dev\logs\2026-10-22\HRTF-A3-convert-retry_064913.log`(개발 PC).
- 내용: M(측정점) 710, R(귀) 2, N(IR 길이) 512, 44,100Hz, 위치는 구면(방위각 도, 고도각 도, 거리 m).
- 걷어내기 전 앱과 같은 결과인지: `tests/test_hrtf_data_parity.rs`(위치·모든 IR·최근접 조회·보정 출력·바이노럴 렌더를
  걷어내기 전 코드의 덤프와 비교).
