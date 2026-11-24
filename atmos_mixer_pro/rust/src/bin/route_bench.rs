//! 매트릭스 라우팅 부하 벤치(설계 `docs/superpowers/specs/2026-11-14-matrix-routing-design.md` 8.4). 조사용이다(자동 시험 아님).
//!
//!   cargo run --release --bin route_bench -- [가|나|다|라|all] [반복 횟수]
//!
//! 믹서 폭 94, 블록 1024프레임, 48kHz. 블록 하나를 처리하는 시간(평균·최대, µs)을 잰다.
//! (가) 표 없는 스테레오 3트랙(매핑) (나) 같은 3트랙을 매핑과 같은 표로 (다) 모노 3트랙을 각 12출력으로
//! (라) 모노 1트랙을 94출력으로. (가)는 구현 전 커밋에서도 같은 코드로 잰다(표 쪽 코드는 그 커밋에 없다).

use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::Arc;
use std::time::Instant;

const FS: u32 = 48_000;
const WIDTH: usize = 94;
const BLOCK: usize = 1024;
const FRAMES: usize = 48_000; // 1초 루프

fn data(channels: usize) -> Arc<SoundData> {
    let mut samples = Vec::with_capacity(FRAMES * channels);
    for i in 0..FRAMES {
        for c in 0..channels {
            samples.push(0.1 * ((i as f32) * 0.01 * (c as f32 + 1.0)).sin());
        }
    }
    Arc::new(SoundData { samples, channels: channels as u16, sample_rate: FS })
}

fn instance(id: u32, d: Arc<SoundData>, out: usize, stereo: bool) -> SoundInstance {
    let ch = d.channels;
    SoundInstance::new(
        id as u64, id, 1, format!("t{id}"), Some(d), None, FS, ch, true, 1.0, out, stereo, None,
        GLOBAL_STATE.enabled_channels.len(),
    )
}

fn with_table(mut i: SoundInstance, cells: &[(u16, u16)]) -> SoundInstance {
    let entries: Vec<_> = cells
        .iter()
        .map(|&(src, out)| rust_lib_atmos_mixer_pro::audio::route::RouteEntry { src, out, gain: 1.0 })
        .collect();
    i.set_initial_routes(&entries);
    i
}

fn build(case: &str) -> Option<AudioMixer> {
    let (gc_tx, _gc_rx) = crossbeam_channel::bounded(4096);
    let mut m = AudioMixer::new(FS, WIDTH, BLOCK, gc_tx, None);
    m.startup_ramp.current_gain = 1.0;
    match case {
        "가" => {
            for k in 0..3 {
                m.instances[k] = Some(instance(k as u32 + 1, data(2), k * 2, true));
            }
        }
                "나" => {
            for k in 0..3 {
                let o = (k * 2) as u16;
                m.instances[k] = Some(with_table(instance(k as u32 + 1, data(2), k * 2, true), &[(0, o), (1, o + 1)]));
            }
        }
                "다" => {
            for k in 0..3 {
                let cells: Vec<(u16, u16)> = (0..12).map(|j| (0, (k * 12 + j) as u16)).collect();
                m.instances[k] = Some(with_table(instance(k as u32 + 1, data(1), 0, false), &cells));
            }
        }
                "라" => {
            let cells: Vec<(u16, u16)> = (0..WIDTH).map(|o| (0, o as u16)).collect();
            m.instances[0] = Some(with_table(instance(1, data(1), 0, false), &cells));
        }
        _ => return None,
    }
    Some(m)
}

fn run(case: &str, iterations: usize) {
    let Some(mut m) = build(case) else {
        println!("{case}: 이 빌드에 없는 경우");
        return;
    };
    let mut out = vec![0.0f32; BLOCK * WIDTH];
    for _ in 0..200 {
        out.fill(0.0);
        m.process(&mut out, WIDTH);
    }
    let mut total = 0u128;
    let mut worst = 0u128;
    for _ in 0..iterations {
        out.fill(0.0);
        let t = Instant::now();
        m.process(&mut out, WIDTH);
        let us = t.elapsed().as_micros();
        total += us;
        worst = worst.max(us);
    }
    println!(
        "{case}: 블록 {iterations}개 평균 {:.1}µs · 최대 {worst}µs (예산 {:.1}µs)",
        total as f64 / iterations as f64,
        BLOCK as f64 / FS as f64 * 1e6
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let which = args.get(1).cloned().unwrap_or_else(|| "all".to_string());
    let iterations: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2000);
    println!(
        "SoundInstance 크기 {}바이트(+ 표 배열 미리 잡기: {}바이트)",
        std::mem::size_of::<SoundInstance>(),
        route_capacity_bytes()
    );
    let cases: Vec<&str> = if which == "all" { vec!["가", "나", "다", "라"] } else { vec![which.as_str()] };
    for c in cases {
        run(c, iterations);
    }
}

fn route_capacity_bytes() -> usize {
    3 * rust_lib_atmos_mixer_pro::audio::route::MAX_TRACK_ROUTES
        * std::mem::size_of::<rust_lib_atmos_mixer_pro::audio::route::RouteEntry>()
}

