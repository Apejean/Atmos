//! 진단(수동, 자동 테스트 아님): sofa-reader를 걷어내기 **전** 기준값을 파일로 덤프한다.
//!
//! 기준 worktree(origin/main c928226, 제품 코드 그대로)에서만 쓴다. 교체 뒤에는
//! `tests/test_hrtf_data_parity.rs`가 같은 형식으로 덤프하고, 두 덤프를 바이트 단위로 비교한다.
//! 덤프를 만드는 부분은 두 파일이 같아야 하므로, 다른 곳은 맨 위 "데이터 접근" 함수뿐이다.
//!
//! 실행: HRTF_DUMP_DIR=<폴더> cargo test --test diag_hrtf_dump -- --ignored --nocapture

use std::path::Path;
use std::sync::atomic::Ordering;

use rust_lib_atmos_mixer_pro::audio::binaural::VirtualMixRoomBinaural;
use rust_lib_atmos_mixer_pro::audio::hrtf_eq;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use sofa_reader::{SofaFile, SourcePosition};

// ---- 데이터 접근(기준: sofa-reader) ----

type Set = SofaFile;

fn load_set() -> Set {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/hrtf/mit_kemar_normal_pinna.sofa");
    SofaFile::strict_load(&path).expect("SOFA strict_load")
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
    db.find_nearest(&SourcePosition::new(t[0], t[1], t[2]))
}

fn set_equalized(db: &Set, rate: f32) -> (f32, usize, Vec<f32>) {
    let mut e = db.clone();
    hrtf_eq::equalize(&mut e, rate);
    (e.sample_rate, e.ir_length, e.impulse_responses)
}

// ---- 덤프(교체 뒤 test_hrtf_data_parity.rs와 같아야 한다) ----

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

#[test]
#[ignore = "수동 진단: HRTF_DUMP_DIR에 기준 덤프를 쓴다"]
fn dump_hrtf_baseline() {
    let dir = std::env::var("HRTF_DUMP_DIR").expect("HRTF_DUMP_DIR을 정해야 한다");
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).expect("덤프 폴더를 만들지 못함");
    let mut manifest = String::new();
    for (name, bytes) in build_dumps() {
        std::fs::write(dir.join(&name), &bytes).expect("덤프를 쓰지 못함");
        let line = format!("{name} {} {:016x}\n", bytes.len(), fnv1a64(&bytes));
        print!("{line}");
        manifest.push_str(&line);
    }
    std::fs::write(dir.join("manifest.txt"), manifest).expect("manifest를 쓰지 못함");
}
