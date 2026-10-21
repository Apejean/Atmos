//! 내장 HRTF 데이터와 바이노럴 출력이 sofa-reader를 걷어내기 전과 비트 단위로 같아야 한다.
//!
//! 기준은 걷어내기 전 코드(origin/main c928226)에서 같은 덤프를 만든 것이다(기준 worktree 전용 진단
//! `tests/diag_hrtf_dump.rs`, 2026-10-22). 덤프를 만드는 부분은 그 파일과 같고, 다른 곳은 맨 위
//! "데이터 접근" 함수와 맨 아래 비교뿐이다.
//!
//! - 데이터(위치·IR·측정점별 IR 조회)는 산술이 없어 어느 플랫폼에서든 비트가 같다 → 늘 기준값과 비교한다.
//! - 최근접 조회·보정·렌더는 sin/cos/atan2·FFT 결과라 OS·CPU마다 마지막 비트가 다를 수 있다 →
//!   같은 PC에서 만든 기준 덤프 폴더를 `HRTF_BASELINE_DIR`로 주면 파일마다 바이트 단위로 비교한다.
//!   Windows 개발 PC(2026-10-22)에서는 12개 모두 같았다(D:\dev\logs\2026-10-22\HRTF-B2-compare_065059.txt).
//!
//! HRTF_DUMP_DIR을 정하면 덤프 파일과 manifest.txt도 쓴다. HRTF 데이터·보정(hrtf_eq)·바이노럴 렌더링을
//! **일부러** 바꾸면 기준도 다시 만들어야 한다.

use std::path::Path;
use std::sync::atomic::Ordering;

use rust_lib_atmos_mixer_pro::audio::binaural::VirtualMixRoomBinaural;
use rust_lib_atmos_mixer_pro::audio::hrtf_data::{HrtfPosition, HrtfSet};
use rust_lib_atmos_mixer_pro::audio::hrtf_eq;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;

// ---- 데이터 접근(교체: audio::hrtf_data, 앱과 같은 내장 파일) ----

type Set = HrtfSet;

fn load_set() -> Set {
    HrtfSet::from_bytes(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/hrtf/mit_kemar_normal_pinna.hrtfset"
    )))
    .expect("HrtfSet::from_bytes")
}

fn set_meta(db: &Set) -> (f32, usize, usize) {
    (db.sample_rate, db.num_measurements, db.ir_length)
}

fn set_positions(db: &Set) -> Vec<[f32; 3]> {
    db.positions.iter().map(|p| [p.azimuth, p.elevation, p.distance]).collect()
}

fn set_irs(db: &Set) -> &[f32] {
    &db.impulse_responses
}

fn set_slices(db: &Set, idx: usize) -> Option<([f32; 3], &[f32], &[f32])> {
    db.get_hrtf_slices(idx).map(|(p, l, r)| ([p.azimuth, p.elevation, p.distance], l, r))
}

fn set_nearest(db: &Set, t: [f32; 3]) -> (usize, f32) {
    db.find_nearest(&HrtfPosition::new(t[0], t[1], t[2]))
}

fn set_equalized(db: &Set, rate: f32) -> (f32, usize, Vec<f32>) {
    let mut e = db.clone();
    hrtf_eq::equalize(&mut e, rate);
    (e.sample_rate, e.ir_length, e.impulse_responses)
}

// ---- 덤프(기준 diag_hrtf_dump.rs와 같아야 한다) ----

const RATES: [f32; 3] = [44_100.0, 48_000.0, 96_000.0];
const RENDER_CH: usize = 6;
const RENDER_OUT_CH: usize = 8;
const RENDER_BLOCK: usize = 512;
const RENDER_BLOCKS: usize = 48;

/// 리틀 엔디언 바이트 버퍼. f32는 비트 그대로 쓴다.
#[derive(Default)]
struct Bytes(Vec<u8>);

impl Bytes {
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_bits().to_le_bytes());
    }
    fn f32s(&mut self, vs: &[f32]) {
        for &v in vs {
            self.f32(v);
        }
    }
}

/// FNV-1a 64비트. 덤프가 같은지 빠르게 보는 지문(암호용 아님, 파일 비교는 SHA-256으로 따로 한다).
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// 결정적 의사 난수(입력 신호·임의 조회 위치용). [0, 1).
struct Lcg(u64);

impl Lcg {
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 40) as f32) / ((1u64 << 24) as f32)
    }
}

/// 최근접 조회 위치: 격자 + 경계 + 임의 위치.
fn lookup_targets(nominal: f32) -> Vec<[f32; 3]> {
    let mut t = Vec::new();
    for d in [nominal, 0.5, 3.0] {
        for ei in 0..30 {
            let el = -50.0 + 5.0 * ei as f32;
            for ai in 0..145 {
                t.push([-180.0 + 2.5 * ai as f32, el, d]);
            }
        }
    }
    for e in [
        [180.0, 0.0, nominal],
        [-180.0, 0.0, nominal],
        [360.0, 0.0, nominal],
        [-360.0, 0.0, nominal],
        [720.0, 0.0, nominal],
        [0.0, 90.0, nominal],
        [0.0, -90.0, nominal],
        [45.0, 45.0, 0.0],
        [0.0, 0.0, 0.0],
        [10.0, 10.0, 1e-6],
        [179.9, -40.0, nominal],
        [-179.9, 89.9, nominal],
    ] {
        t.push(e);
    }
    let mut rng = Lcg(0x5EED_0001);
    for _ in 0..1000 {
        let az = rng.next_f32() * 720.0 - 360.0;
        let el = rng.next_f32() * 180.0 - 90.0;
        let d = rng.next_f32() * 5.0;
        t.push([az, el, d]);
    }
    t
}

fn set_angles(bin: &mut VirtualMixRoomBinaural, az: &[f32], el: &[f32]) {
    let (a, e) = bin.channel_base_angles_mut();
    a.copy_from_slice(az);
    e.copy_from_slice(el);
    bin.mark_base_azimuth_dirty();
}

/// 앱 경로 전체: 엔진이 쓰는 바이노럴(내장 데이터 로드 → 보정 → 최근접 3점 보간 → 합성곱).
/// 중간에 스피커 각도를 바꾸고(교차 페이드) 머리를 돌린다(요각).
fn render(rate: f32) -> Vec<u8> {
    let mut b = Bytes::default();
    let mut bin = VirtualMixRoomBinaural::new_with_rate(RENDER_CH, RENDER_BLOCK, rate);
    bin.enabled = true;
    match bin.hrtf_sample_rate() {
        Some(s) => {
            b.u32(1);
            b.f32(s);
        }
        None => b.u32(0),
    }
    set_angles(&mut bin, &[0.0, 30.0, -30.0, 110.0, -110.0, 180.0], &[0.0, 0.0, 10.0, 20.0, -30.0, 60.0]);
    let mut rng = Lcg(0x5EED_0002);
    for block in 0..RENDER_BLOCKS {
        if block == 16 {
            set_angles(&mut bin, &[0.0, 75.0, -30.0, 110.0, -110.0, 180.0], &[0.0, 0.0, -20.0, 20.0, -30.0, 60.0]);
        }
        if block == 32 {
            GLOBAL_STATE.hrtf_yaw.store(0.7f32.to_bits(), Ordering::Relaxed);
        }
        let mut buf = vec![0.0f32; RENDER_BLOCK * RENDER_OUT_CH];
        for f in 0..RENDER_BLOCK {
            for ch in 0..RENDER_CH {
                let impulse = if block == 0 && f == 0 { 1.0 } else { 0.0 };
                buf[f * RENDER_OUT_CH + ch] = impulse + 0.25 * (rng.next_f32() * 2.0 - 1.0);
            }
        }
        bin.process_interleaved(&mut buf, RENDER_OUT_CH);
        for f in 0..RENDER_BLOCK {
            b.f32(buf[f * RENDER_OUT_CH]);
            b.f32(buf[f * RENDER_OUT_CH + 1]);
        }
    }
    GLOBAL_STATE.hrtf_yaw.store(0.0f32.to_bits(), Ordering::Relaxed);
    b.u32(bin.debug_hrir_rebuild_count() as u32);
    b.0
}

/// 고도 링 인덱스 조회와 전수 조회의 최근접 3점(인덱스, 거리).
fn nearest3() -> Vec<u8> {
    let bin = VirtualMixRoomBinaural::new_with_rate(1, RENDER_BLOCK, 48_000.0);
    let mut b = Bytes::default();
    for ei in 0..29 {
        let el = -45.0 + 5.0 * ei as f32;
        for ai in 0..73 {
            let az = -180.0 + 5.0 * ai as f32;
            for (idx, dist) in bin.debug_nearest_hrirs(az, el) {
                b.u32(idx as u32);
                b.f32(dist);
            }
            for (idx, dist) in bin.debug_nearest_hrirs_brute_force(az, el) {
                b.u32(idx as u32);
                b.f32(dist);
            }
        }
    }
    b.0
}

/// (파일 이름, 바이트)를 정해진 순서로 만든다.
fn build_dumps() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let db = load_set();
    let (sr, m, n) = set_meta(&db);

    let mut b = Bytes::default();
    b.f32(sr);
    b.u32(m as u32);
    b.u32(n as u32);
    out.push(("raw_meta.bin".to_string(), b.0));

    let positions = set_positions(&db);
    let mut b = Bytes::default();
    for p in &positions {
        b.f32s(p);
    }
    out.push(("raw_positions.bin".to_string(), b.0));

    let mut b = Bytes::default();
    b.f32s(set_irs(&db));
    out.push(("raw_ir.bin".to_string(), b.0));

    // m번(범위 밖)은 None이어야 한다.
    let mut b = Bytes::default();
    for idx in 0..=m {
        match set_slices(&db, idx) {
            Some((p, l, r)) => {
                b.u32(1);
                b.f32s(&p);
                b.u32(l.len() as u32);
                b.f32s(l);
                b.u32(r.len() as u32);
                b.f32s(r);
            }
            None => b.u32(0),
        }
    }
    out.push(("slices.bin".to_string(), b.0));

    let nominal = positions.first().map(|p| p[2]).unwrap_or(1.0);
    let mut b = Bytes::default();
    for t in lookup_targets(nominal) {
        let (idx, dist) = set_nearest(&db, t);
        b.f32s(&t);
        b.u32(idx as u32);
        b.f32(dist);
    }
    out.push(("nearest.bin".to_string(), b.0));

    for rate in RATES {
        let (esr, en, eir) = set_equalized(&db, rate);
        let mut b = Bytes::default();
        b.f32(esr);
        b.u32(m as u32);
        b.u32(en as u32);
        b.f32s(&eir);
        out.push((format!("eq_{}.bin", rate as u32), b.0));
    }

    for rate in RATES {
        out.push((format!("render_{}.bin", rate as u32), render(rate)));
    }
    out.push(("nearest3.bin".to_string(), nearest3()));
    out
}

// ---- 기준과 비교 ----

/// 걷어내기 전 덤프 중 산술이 없는 데이터의 (이름, 바이트 수, FNV-1a 64). 기준 worktree에서 diag_hrtf_dump로
/// 잰 값(로그 D:\dev\logs\2026-10-22\HRTF-A1-baseline-dump_064429.log). 플랫폼과 무관하게 같아야 한다.
const EXPECTED_DATA: &[(&str, usize, u64)] = &[
    ("raw_meta.bin", 12, 0x15a2_5095_c08b_78d6),
    ("raw_positions.bin", 8_520, 0xea9b_7174_52b0_9863),
    ("raw_ir.bin", 2_908_160, 0xb183_b1b9_1c53_b8fd),
    ("slices.bin", 2_925_204, 0xa0b7_e498_ad9d_8a53),
];

#[test]
fn hrtf_data_and_binaural_output_match_the_sofa_reader_baseline() {
    let dumps = build_dumps();
    let dump_dir = std::env::var("HRTF_DUMP_DIR").ok();
    let baseline_dir = std::env::var("HRTF_BASELINE_DIR").ok();
    if let Some(dir) = &dump_dir {
        std::fs::create_dir_all(dir).expect("덤프 폴더를 만들지 못함");
    }
    let mut manifest = String::new();
    let mut mismatches = Vec::new();
    for (name, bytes) in &dumps {
        let hash = fnv1a64(bytes);
        let line = format!("{name} {} {hash:016x}\n", bytes.len());
        print!("{line}");
        manifest.push_str(&line);
        if let Some(dir) = &dump_dir {
            std::fs::write(Path::new(dir).join(name), bytes).expect("덤프를 쓰지 못함");
        }
        if let Some(&(_, expected_len, expected_hash)) = EXPECTED_DATA.iter().find(|e| e.0 == name) {
            if bytes.len() != expected_len || hash != expected_hash {
                mismatches.push(format!(
                    "{name}: {}바이트 {hash:016x} / 기준 {expected_len}바이트 {expected_hash:016x}",
                    bytes.len()
                ));
            }
        }
        if let Some(dir) = &baseline_dir {
            match std::fs::read(Path::new(dir).join(name)) {
                Ok(base) if base == *bytes => {}
                Ok(base) => {
                    let first = base.iter().zip(bytes.iter()).position(|(a, b)| a != b);
                    mismatches.push(format!(
                        "{name}: 기준 덤프와 다름(기준 {}바이트, 지금 {}바이트, 처음 다른 바이트 {first:?})",
                        base.len(),
                        bytes.len()
                    ));
                }
                Err(e) => mismatches.push(format!("{name}: 기준 덤프를 읽지 못함: {e}")),
            }
        }
    }
    if let Some(dir) = &dump_dir {
        std::fs::write(Path::new(dir).join("manifest.txt"), &manifest).expect("manifest를 쓰지 못함");
    }
    for &(expected_name, _, _) in EXPECTED_DATA {
        assert!(dumps.iter().any(|(n, _)| n == expected_name), "{expected_name} 덤프가 없다");
    }
    assert!(mismatches.is_empty(), "기준과 다른 덤프:\n{}", mismatches.join("\n"));
}
