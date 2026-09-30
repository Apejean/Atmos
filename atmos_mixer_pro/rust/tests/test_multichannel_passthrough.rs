//! 바이노럴을 끄면 멀티채널 트랙은 파일 채널 그대로 같은 번호의 출력으로 나간다.
//!
//! 사용자 보고(2026-09-28): "바이노럴을 껐는데 멀티트랙이 스피커 위치에 따라 바이노럴처럼
//! 움직인다". 바이노럴을 끄면 스피커 배치는 소리 방향에 관여하지 않는다 — 채널 사이를 섞거나
//! 패닝하지 않는다. 배치가 바꾸는 건 채널마다 따로 걸리는 자동 보정(시간 정렬·거리 게인·EQ,
//! Dart acoustic_sync가 계산해 ApplyAllChannelTunings로 보낸다)뿐이고, 그건 실제 스피커까지의
//! 거리를 상쇄하는 현장 보정이다. 이 테스트는 보정을 뺀 엔진 경로가 위치와 무관함을 지킨다.

use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::common::config::{Point3D, RoomZone};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const FS: u32 = 48_000;
const CH: usize = 12;
const BLOCK: usize = 480;
/// 파일 채널별 순음(0.5초 창에 정수 주기가 들어가는 주파수).
const FREQS: [f32; 4] = [300.0, 500.0, 700.0, 900.0];

fn p(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

fn four_channel_file() -> Arc<SoundData> {
    let n = FS as usize * 3;
    let mut samples = Vec::with_capacity(n * FREQS.len());
    for i in 0..n {
        for f in FREQS {
            samples.push(0.1 * (std::f32::consts::TAU * f * i as f32 / FS as f32).sin());
        }
    }
    Arc::new(SoundData { samples, channels: FREQS.len() as u16, sample_rate: FS })
}

/// 4채널 파일을 CH1부터 N:N(앱의 "스테레오 켬 + CH1")으로, 바이노럴 끄고 튼다.
/// CH1·CH2 스피커를 `positions` 자리에 둔다. 1초 뒤 0.5초 동안 CH1~CH5 출력을 모은다.
fn render(positions: [Point3D; 2]) -> Vec<Vec<f32>> {
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    let mut m = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    m.startup_ramp.current_gain = 1.0;
    let mut inst = SoundInstance::new(
        1, 1, 1, "multi".to_string(), Some(four_channel_file()), None, FS, 4, false, 1.0, 0, true, None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.fade_weight = 1.0;
    m.instances[0] = Some(inst);

    m.room_zones = vec![RoomZone {
        room_id: 1,
        boundary_min: p(0.0, 0.0, 0.0),
        boundary_max: p(15.0, 20.0, 15.0),
        ear_level: 1.6,
        ..Default::default()
    }];
    m.listener_position = Some(p(7.5, 10.0, 1.6));
    m.channel_positions = vec![None; CH];
    m.channel_room_ids = vec![None; CH];
    for (ch, pos) in positions.into_iter().enumerate() {
        m.channel_positions[ch] = Some(pos);
        m.channel_room_ids[ch] = Some(1);
    }
    m.binaural.enabled = false;
    m.set_binaural_room(Some(1));
    m.recalculate_binaural_channel_azimuths();
    m.recalculate_spatial_dsp();

    let mut buf = vec![0.0f32; CH * BLOCK];
    let mut out = vec![Vec::new(); 5];
    for i in 0..(FS as usize * 3 / 2) / BLOCK {
        buf.fill(0.0);
        m.process(&mut buf, CH);
        if i >= FS as usize / BLOCK {
            for f in 0..BLOCK {
                for (c, o) in out.iter_mut().enumerate() {
                    o.push(buf[f * CH + c]);
                }
            }
        }
    }
    out
}

/// `x`에 담긴 `freq` 순음의 진폭(창 길이에 정수 주기가 들어가야 정확하다).
fn tone_amplitude(x: &[f32], freq: f32) -> f32 {
    let (mut re, mut im) = (0.0f64, 0.0f64);
    for (i, v) in x.iter().enumerate() {
        let ph = std::f64::consts::TAU * freq as f64 * i as f64 / FS as f64;
        re += *v as f64 * ph.cos();
        im += *v as f64 * ph.sin();
    }
    (2.0 * (re * re + im * im).sqrt() / x.len() as f64) as f32
}

/// `file`을 `out_ch`부터 틀고(스테레오 켬이면 N:N), CH1·CH2에 서로 다른 채널 FX를 건다.
/// 1초 뒤 0.5초 동안의 (CH1, CH2) 출력.
fn render_with_fx(file: Arc<SoundData>, out_ch: usize, stereo: bool) -> (Vec<f32>, Vec<f32>) {
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    let mut m = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    m.startup_ramp.current_gain = 1.0;
    let (sr, chans) = (file.sample_rate, file.channels);
    let mut inst = SoundInstance::new(
        1, 1, 1, "t".to_string(), Some(file), None, sr, chans, false, 1.0, out_ch, stereo, None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.fade_weight = 1.0;
    m.instances[0] = Some(inst);
    m.binaural.enabled = false;
    // 스피커 배치가 만든 자동 FX라고 치고 채널마다 다르게 건다(시간 정렬·거리 게인·EQ).
    for (ch, delay, gain) in [(0usize, 3.0f32, -6.0f32), (1, 7.0, 4.0)] {
        let dsp = &mut m.channel_dsp[ch];
        dsp.update_delay_target(delay);
        dsp.set_gain_db(gain);
        dsp.update_eq_targets(
            &[rust_lib_atmos_mixer_pro::common::config::EqBand {
                enabled: true,
                freq: 2000.0,
                gain: -3.0,
                q_factor: 1.0,
                filter_type: rust_lib_atmos_mixer_pro::common::config::EqType::Bell,
                slope_db_per_oct: 12,
            }],
            FS as f32,
        );
    }
    let mut buf = vec![0.0f32; CH * BLOCK];
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for i in 0..(FS as usize * 3 / 2) / BLOCK {
        buf.fill(0.0);
        m.process(&mut buf, CH);
        if i >= FS as usize / BLOCK {
            for f in 0..BLOCK {
                a.push(buf[f * CH]);
                b.push(buf[f * CH + 1]);
            }
        }
    }
    (a, b)
}

/// 채널별 순음 파일(`freqs`의 0은 무음 채널).
fn tone_file(freqs: &[f32]) -> Arc<SoundData> {
    let n = FS as usize * 3;
    let mut samples = Vec::with_capacity(n * freqs.len());
    for i in 0..n {
        for &f in freqs {
            samples.push(if f > 0.0 { 0.1 * (std::f32::consts::TAU * f * i as f32 / FS as f32).sin() } else { 0.0 });
        }
    }
    Arc::new(SoundData { samples, channels: freqs.len() as u16, sample_rate: FS })
}

#[test]
fn 모노_스테레오_멀티_트랙_모두_같은_채널이면_그_채널의_fx를_똑같이_받는다() {
    // 사용자 질문(2026-09-29): "FX는 스피커 채널별로 걸리니, 모노·스테레오도 그 채널 FX대로
    // 들려야 하지 않나?" — 맞다. 채널 FX는 트랙 종류와 무관하게 그 출력 채널의 모든 소리에 걸린다.
    let (_, mono_ch2) = render_with_fx(tone_file(&[500.0]), 1, false);
    let (st_ch1, st_ch2) = render_with_fx(tone_file(&[300.0, 500.0]), 0, true);
    let (mu_ch1, mu_ch2) = render_with_fx(tone_file(&[300.0, 500.0, 0.0, 0.0]), 0, true);

    let max_diff = |a: &[f32], b: &[f32]| a.iter().zip(b).fold(0.0f32, |m, (x, y)| m.max((x - y).abs()));
    assert!(max_diff(&mono_ch2, &st_ch2) < 1e-6, "모노와 스테레오의 CH2 처리가 다르다");
    assert!(max_diff(&st_ch2, &mu_ch2) < 1e-6, "스테레오와 멀티의 CH2 처리가 다르다");
    assert!(max_diff(&st_ch1, &mu_ch1) < 1e-6, "스테레오와 멀티의 CH1 처리가 다르다");

    // FX가 실제로 걸렸는지: CH2 +4dB(0.1 → 약 0.158). 2kHz 벨은 500Hz에 거의 영향이 없다.
    let a = tone_amplitude(&mono_ch2, 500.0);
    assert!(a > 0.13 && a < 0.17, "CH2 FX(+4dB)가 모노 트랙에 걸리지 않았다: {a}");
}

#[test]
fn 바이노럴을_끄면_멀티채널_트랙은_채널_그대로_나가고_스피커를_옮겨도_움직이지_않는다() {
    let here = render([p(8.74, 0.2, 2.61), p(13.03, 15.03, 2.44)]);
    let moved = render([p(2.0, 18.0, 5.0), p(12.0, 3.0, 9.0)]);

    for c in 0..FREQS.len() {
        let diff = here[c].iter().zip(&moved[c]).fold(0.0f32, |m, (a, b)| m.max((a - b).abs()));
        assert!(diff < 1e-6, "CH{}: 스피커를 옮겼더니 출력이 {diff} 만큼 바뀌었다", c + 1);

        for (k, &f) in FREQS.iter().enumerate() {
            let a = tone_amplitude(&here[c], f);
            if k == c {
                assert!((a - 0.1).abs() < 0.002, "CH{}: 파일 {}번 채널이 {a}로 나왔다(0.1이어야 한다)", c + 1, c + 1);
            } else {
                assert!(a < 1e-5, "CH{}에 파일 {}번 채널이 {a} 섞였다", c + 1, k + 1);
            }
        }
    }
    let leak = here[4].iter().fold(0.0f32, |m, v| m.max(v.abs()));
    assert!(leak < 1e-6, "4채널 파일인데 CH5로 {leak}가 나갔다");
}
