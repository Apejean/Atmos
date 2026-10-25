//! 수동 진단: 실제 공연 음원을 엔진과 같은 설정의 피크 리미터에 넣으면 얼마나 걸리는가(2026-10-25).
//!
//! Autoguard(채널마다 3초 평균 음량이 높으면 몰래 낮추던 기능)를 뺀 뒤(98a5bc0), 사람이 귀로 듣기 전에 소리를
//! 내지 않고 신호로 재는 확인이다. 음원 채널을 증폭 없이(0 dB) 출력 하나에 그대로 보낸다고 보고(가장 불리한
//! 경우: 실제 재생은 채널 게인·DBAP 분배로 스피커마다 더 작다) 리미터 출력 피크, 최대 게인 리덕션, 게인
//! 리덕션이 걸린 시간 비율, 1 dB 넘게 걸리기 시작한 횟수를 잰다. 함께 예전 Autoguard가 줄였을 양도 잰다:
//! 예전 코드는 리미터의 3초 간이 LUFS가 -14를 넘으면 넘은 만큼(dB) 낮췄으므로, 그 값이 이번에 커지는 소리다.
//!
//! 자동 테스트가 아니다(#[ignore]). 실행:
//!   DIAG_AUDIO="<파일1>;<파일2>" cargo test --release --test diag_limiter_program -- --ignored --nocapture

use rust_lib_atmos_mixer_pro::audio::limiter::PeakLimiter;
use rust_lib_atmos_mixer_pro::audio::player::SoundData;

const FS: u32 = 48_000;

fn db(v: f32) -> f32 {
    20.0 * v.max(1e-9).log10()
}

#[test]
#[ignore]
fn diag_limiter_on_program_audio() {
    let list = std::env::var("DIAG_AUDIO").expect("DIAG_AUDIO=<파일1>;<파일2>");
    for path in list.split(';').map(str::trim).filter(|p| !p.is_empty()) {
        let data = SoundData::load_from_file(std::path::Path::new(path), FS).expect("파일 디코딩");
        let ch = data.channels as usize;
        let frames = data.samples.len() / ch;
        let minutes = frames as f32 / FS as f32 / 60.0;
        println!("== {path} ({ch}ch, {:.1}s)", frames as f32 / FS as f32);
        for c in 0..ch {
            report(&format!("ch{}", c + 1), frames, minutes, |f| data.samples[f * ch + c]);
        }
        if ch > 1 {
            // 트랙이 output_stereo=false면 mixer.rs가 채널 평균을 출력 하나로 보낸다.
            report("mono avg", frames, minutes, |f| data.samples[f * ch..f * ch + ch].iter().sum::<f32>() / ch as f32);
        }
    }
}

fn report(label: &str, frames: usize, minutes: f32, sample: impl Fn(usize) -> f32) {
    // mixer.rs가 출력 채널마다 만드는 것과 같은 값(공격 1ms, 해제 500ms, 문턱 0.99).
    let mut lim = PeakLimiter::new(FS as f32, 1.0, 500.0, 0.99);
    let (mut in_peak, mut out_peak, mut max_gr) = (0.0f32, 0.0f32, 0.0f32);
    let (mut over_half_db, mut over_3db, mut onsets) = (0usize, 0usize, 0usize);
    let mut engaged = false;
    // 예전 Autoguard: 3초 간이 LUFS가 -14를 넘은 양(dB)만큼 낮췄다(98a5bc0에서 뺌).
    let (mut max_lufs, mut ducked, mut duck_sum, mut max_duck) = (-70.0f32, 0usize, 0.0f64, 0.0f32);
    for f in 0..frames {
        let x = sample(f);
        let y = lim.process(x);
        in_peak = in_peak.max(x.abs());
        out_peak = out_peak.max(y.abs());
        let gr = lim.current_gain_reduction_db();
        max_gr = max_gr.max(gr);
        over_half_db += usize::from(gr > 0.5);
        over_3db += usize::from(gr > 3.0);
        let now = gr > 1.0;
        onsets += usize::from(now && !engaged);
        engaged = now;
        max_lufs = max_lufs.max(lim.short_term_lufs);
        let duck = (lim.short_term_lufs + 14.0).max(0.0);
        if duck > 0.0 {
            ducked += 1;
            duck_sum += duck as f64;
            max_duck = max_duck.max(duck);
        }
    }
    println!(
        "  {label}: in peak {:.2} dBFS, out peak {:.2} dBFS, max GR {:.2} dB, GR>0.5dB {:.2}%, GR>3dB {:.3}%, GR>1dB onsets {:.1}/min",
        db(in_peak),
        db(out_peak),
        max_gr,
        100.0 * over_half_db as f32 / frames as f32,
        100.0 * over_3db as f32 / frames as f32,
        onsets as f32 / minutes.max(1e-6)
    );
    println!(
        "       old Autoguard: max short-term {:.1} LUFS, would duck {:.1}% of time, max {:.2} dB, mean {:.2} dB while ducking",
        max_lufs,
        100.0 * ducked as f32 / frames as f32,
        max_duck,
        if ducked > 0 { (duck_sum / ducked as f64) as f32 } else { 0.0 }
    );
}
