//! 현장 시나리오 회귀 테스트: 테마 1(15x20m, 천장 15m, 귀높이 1.6m)에서 가까운 서브(화면 CH1,
//! 5.8m)와 천장 가까이 매단 먼 메인(화면 CH2, 13.8m). 튜닝은 앱 자동 계산값 그대로.
//!
//! 사용자 보고(2026-09-28): "CH2로 튼 사운드의 저음이 서브에서 안 나온다", "저음 데시벨이 너무
//! 낮다". 원인은 셋이었다. ① 베이스 매니지먼트가 메인의 채널 DSP 뒤에서 저역을 갈라 메인의
//! 로우컷·쉘프가 서브 저역에 걸림 ② 헤드폰 미리듣기가 거리(지연·감쇠)를 흉내 내지 않아 서브의
//! 시간 정렬 23ms가 그대로 남아 크로스오버 부근이 지워짐 ③ CH2를 두 방에 써서 다른 방 기준
//! 튜닝(극성 반전·로우컷)이 걸림. 고친 뒤에는 헤드폰에서 메인 사운드의 저역이 중역과 같은 크기로
//! 들려야 한다(예전: 70Hz -15.9dB, 30~80Hz 대부분 -5~-13dB).
//!
//! 진단 표(수동): cargo test --release --test test_field_scenario_bass_balance -- --ignored --nocapture
//!   DIAG_TUNING_JSON=<앱의 tuning_state를 내보낸 JSON 파일>로 앱의 현재 튜닝을 넣어 볼 수 있다.

use rust_lib_atmos_mixer_pro::audio::acoustic::compute_early_reflection_taps;
use rust_lib_atmos_mixer_pro::audio::bass_route::compute_bass_route;
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::common::config::{EqBand, EqType, Point3D, RoomZone};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const FS: u32 = 48_000;
const CH: usize = 12;
const BLOCK: usize = 512;
const AMP: f32 = 0.05;
const SUB: usize = 0; // 화면 CH1
const MAIN: usize = 1; // 화면 CH2
const ROOM: u32 = 1;

#[derive(Clone, Copy)]
struct Scenario {
    binaural: bool,
    sub_on: bool,
    lfe_boost: bool,
    tuning: bool,
    /// 헤드폰 방 시뮬레이션(초기반사). 순음으로 재면 반사 간섭 무늬가 크게 섞여 켜지 않는다.
    er: bool,
}

fn p(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

fn band(filter_type: EqType, freq: f32, gain: f32, q: f32) -> EqBand {
    EqBand { enabled: true, freq, gain, q_factor: q, filter_type, slope_db_per_oct: 12 }
}

fn zone() -> RoomZone {
    RoomZone {
        room_id: ROOM,
        boundary_min: p(0.0, 0.0, 0.0),
        boundary_max: p(15.0, 20.0, 15.0),
        ear_level: 1.6,
        absorption_coeff: 0.3,
        ..Default::default()
    }
}

/// 앱 자동 계산값(2026-09-28, 테마 1을 보고 있을 때): (게인dB, 딜레이ms, 극성 반전, EQ).
fn auto_tuning(ch: usize) -> (f32, f32, bool, Vec<EqBand>) {
    if ch == SUB {
        (-2.238, 23.338, false, vec![band(EqType::LowCut, 20.0, 0.0, 0.707)])
    } else {
        (
            5.303,
            0.0,
            false,
            vec![
                band(EqType::LowShelf, 300.0, -2.1, 0.707),
                band(EqType::Bell, 111.67, -2.58, 3.12),
                band(EqType::Bell, 720.13, -2.92, 3.42),
                band(EqType::Bell, 50.84, -1.2, 2.83),
                band(EqType::HighShelf, 10000.0, 4.41, 0.707),
            ],
        )
    }
}

fn mixer_for(s: Scenario, tone_hz: f32, tone_ch: usize) -> AudioMixer {
    let n = FS as usize * 6;
    let samples: Vec<f32> = (0..n)
        .map(|i| AMP * (std::f32::consts::TAU * tone_hz * i as f32 / FS as f32).sin())
        .collect();
    mixer_playing(s, Arc::new(SoundData { samples, channels: 1, sample_rate: FS }), tone_ch)
}

/// `data`를 `out_ch` 모노 출력으로 반복 재생하는 믹서(앱의 트랙 재생과 같은 경로:
/// 스테레오 파일은 (L+R)/2로 합쳐 한 채널에 넣는다).
fn mixer_playing(s: Scenario, data: Arc<SoundData>, out_ch: usize) -> AudioMixer {
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    let mut m = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    GLOBAL_STATE.hrtf_yaw.store(0.0f32.to_bits(), Ordering::Relaxed);
    m.startup_ramp.current_gain = 1.0;

    let (sr, chans) = (data.sample_rate, data.channels);
    let mut inst = SoundInstance::new(
        1, 1, 1, "track".to_string(), Some(data), None, sr, chans, true, 1.0, out_ch, false, None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.fade_weight = 1.0;
    m.instances[0] = Some(inst);

    // 공간 설정(엔진이 UpdateSpatialConfig에서 하는 일)
    let z = zone();
    m.room_zones = vec![z.clone()];
    m.listener_position = Some(p(7.5, 10.0, 1.6));
    m.channel_positions = vec![None; CH];
    m.channel_room_ids = vec![None; CH];
    let sub_pos = p(11.47, 14.10, 2.61);
    let main_pos = p(7.02, 14.19, 14.75);
    m.channel_positions[SUB] = Some(sub_pos.clone());
    m.channel_positions[MAIN] = Some(main_pos.clone());
    m.channel_room_ids[SUB] = Some(ROOM);
    m.channel_room_ids[MAIN] = Some(ROOM);
    m.binaural.enabled = s.binaural;
    m.set_binaural_room(Some(ROOM));
    m.recalculate_binaural_channel_azimuths();
    if s.er {
        for (ch, pos) in [(SUB, &sub_pos), (MAIN, &main_pos)] {
            let taps = compute_early_reflection_taps(pos, &z);
            m.apply_early_reflection_taps(ch, &taps);
        }
    }

    // 베이스 매니지먼트
    let is_sub: Vec<bool> = (0..CH).map(|c| s.sub_on && c == SUB).collect();
    let has_speaker: Vec<bool> = m.channel_positions.iter().map(|p| p.is_some()).collect();
    let route = compute_bass_route(&is_sub, &m.channel_room_ids, &has_speaker);
    let _ = m.set_bass_routing(route, is_sub);
    for c in m.crossovers.iter_mut() {
        c.set_target_freq(80.0, FS as f32);
    }
    m.lfe_boost_enabled = s.lfe_boost;

    // 채널 튜닝(ApplyAllChannelTunings와 같은 호출)
    if s.tuning {
        let app = app_tunings();
        for (ch, key) in [(SUB, "1"), (MAIN, "2")] {
            let (gain, delay, invert, bands) = app
                .as_ref()
                .and_then(|t| tuning_from_json(t, key))
                .unwrap_or_else(|| auto_tuning(ch));
            let d = &mut m.channel_dsp[ch];
            d.update_delay_target(delay);
            d.update_eq_targets(&bands, FS as f32);
            d.phase_invert = invert;
            d.set_gain_db(gain);
        }
    }
    m
}

fn app_tunings() -> Option<serde_json::Value> {
    let path = std::env::var("DIAG_TUNING_JSON").ok()?;
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// 앱 tuning_state의 한 채널 → (게인dB, 딜레이ms, 극성 반전, EQ 밴드).
fn tuning_from_json(t: &serde_json::Value, key: &str) -> Option<(f32, f32, bool, Vec<EqBand>)> {
    let c = t.get(key)?;
    let f = |k: &str, i: usize| c.get(k)?.get(i)?.as_f64().map(|v| v as f32);
    let types = [EqType::LowCut, EqType::LowShelf, EqType::Bell, EqType::Notch, EqType::HighShelf, EqType::HighCut];
    let mut bands = Vec::new();
    for i in 0..8 {
        let ty = c.get("bandTypes").and_then(|v| v.get(i)).and_then(|v| v.as_u64()).unwrap_or(2) as usize;
        bands.push(EqBand {
            enabled: c.get("bandEnabled")?.get(i)?.as_bool()?,
            freq: f("freqs", i)?,
            gain: f("gains", i)?,
            q_factor: f("qs", i)?,
            filter_type: types[ty.min(5)],
            slope_db_per_oct: c.get("bandSlopes").and_then(|v| v.get(i)).and_then(|v| v.as_u64()).unwrap_or(12) as u32,
        });
    }
    Some((
        c.get("gainDb")?.as_f64()? as f32,
        c.get("delay")?.as_f64()? as f32,
        c.get("phaseInvert")?.as_bool()?,
        bands,
    ))
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

/// 1.5초 안정화 후 0.5초 측정. 바이노럴이면 (헤드폰 L+R, 0), 아니면 (CH1 출력, CH2 출력).
fn measure(s: Scenario, tone_hz: f32, tone_ch: usize) -> (f32, f32) {
    let mut m = mixer_for(s, tone_hz, tone_ch);
    let mut buf = vec![0.0f32; CH * BLOCK];
    for _ in 0..141 {
        buf.fill(0.0);
        m.process(&mut buf, CH);
    }
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for _ in 0..47 {
        buf.fill(0.0);
        m.process(&mut buf, CH);
        for f in 0..BLOCK {
            a.push(buf[f * CH]);
            b.push(buf[f * CH + 1]);
        }
    }
    if s.binaural {
        let both: Vec<f32> = a.iter().chain(b.iter()).copied().collect();
        (rms(&both), 0.0)
    } else {
        (rms(&a), rms(&b))
    }
}

fn db(x: f32) -> f32 {
    20.0 * x.max(1e-9).log10()
}

const HEADPHONES: Scenario = Scenario { binaural: true, sub_on: true, lfe_boost: false, tuning: true, er: false };

#[test]
fn 테마1_헤드폰에서_메인_사운드의_저역이_중역과_같은_크기로_들린다() {
    let ref_1k = measure(HEADPHONES, 1000.0, MAIN).0;
    for f in [30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 120.0] {
        let d = db(measure(HEADPHONES, f, MAIN).0 / ref_1k);
        assert!(d.abs() <= 3.0, "CH2 사운드 {f}Hz가 1kHz 대비 {d:+.1}dB");
    }
}

#[test]
fn 테마1_서브_자기_신호와_메인에서_넘어온_저역이_같은_크기로_들린다() {
    // LFE +10dB를 끄면 CH1로 튼 사운드(서브 자기 신호)와 CH2로 튼 사운드의 저역이 같아야 한다.
    for f in [40.0, 50.0, 60.0] {
        let own = measure(HEADPHONES, f, SUB).0;
        let folded = measure(HEADPHONES, f, MAIN).0;
        let d = db(folded / own);
        assert!(d.abs() <= 1.5, "{f}Hz: CH2에서 넘어온 저역이 서브 자기 신호보다 {d:+.1}dB");
    }
}

#[test]
fn 테마1_바이노럴을_끄고_정지하면_서브까지_모든_출력이_조용해진다() {
    // 사용자 보고(2026-09-28): 바이노럴을 끄고 CH2 트랙을 재생·정지하면 소리가 멈추지 않고,
    // 한 번 더 재생·정지해야 멈춘다. 엔진의 정지 경로(StopTrack = is_stopping → 300ms 페이드 →
    // 슬롯 비움)가 실제 튜닝·서브 라우팅(CH2 저역 → CH1)에서도 끝까지 조용해지는지 본다.
    let n = FS as usize * 6;
    let samples: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / FS as f32;
            AMP * ((std::f32::consts::TAU * 50.0 * t).sin() + (std::f32::consts::TAU * 1000.0 * t).sin())
        })
        .collect();
    let data = Arc::new(SoundData { samples, channels: 1, sample_rate: FS });
    let mut m = mixer_playing(Scenario { binaural: false, ..HEADPHONES }, data, MAIN);
    let mut buf = vec![0.0f32; CH * BLOCK];
    let peak_of = |buf: &[f32], ch: usize| (0..BLOCK).map(|f| buf[f * CH + ch].abs()).fold(0.0f32, f32::max);

    let (mut sub_peak, mut main_peak) = (0.0f32, 0.0f32);
    for _ in 0..94 {
        buf.fill(0.0);
        m.process(&mut buf, CH);
        sub_peak = sub_peak.max(peak_of(&buf, SUB));
        main_peak = main_peak.max(peak_of(&buf, MAIN));
    }
    assert!(sub_peak > 0.01 && main_peak > 0.01, "재생 중인데 CH1 {sub_peak} / CH2 {main_peak}");

    // 엔진의 StopTrack 처리와 같다(engine.rs process_commands).
    for inst in m.instances.iter_mut().flatten() {
        inst.is_stopping = true;
    }
    for _ in 0..47 {
        buf.fill(0.0);
        m.process(&mut buf, CH);
    }
    assert!(m.instances.iter().all(|s| s.is_none()), "정지한 트랙이 믹서 슬롯에 남았다");
    let mut rest = 0.0f32;
    for _ in 0..47 {
        buf.fill(0.0);
        m.process(&mut buf, CH);
        rest = rest.max(buf.iter().fold(0.0f32, |a, v| a.max(v.abs())));
    }
    assert!(rest < 1e-4, "정지 0.5초 뒤에도 출력에 {:+.1}dBFS가 남았다", db(rest));
}

#[test]
#[ignore]
fn 진단_표() {
    const FREQS: [f32; 12] = [30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 120.0, 150.0, 300.0, 1000.0];
    let table = |label: &str, s: Scenario| {
        let ref_1k = measure(s, 1000.0, MAIN).0;
        println!("\n[{label}] 헤드폰 레벨 (CH2 1kHz 기준 dB)\n  주파수 | 사운드2(CH2) | 사운드1(CH1)");
        for f in FREQS {
            let s2 = measure(s, f, MAIN).0;
            let s1 = measure(s, f, SUB).0;
            println!("  {f:>6.0} | {:>+8.1} | {:>+8.1}", db(s2 / ref_1k), db(s1 / ref_1k));
        }
    };
    table("헤드폰, 반사 끔", HEADPHONES);
    table("헤드폰, 방 시뮬레이션(반사) 켬", Scenario { er: true, ..HEADPHONES });
    table("헤드폰, +10dB 켬", Scenario { lfe_boost: true, ..HEADPHONES });

    let spk = Scenario { binaural: false, ..HEADPHONES };
    println!("\n[스피커 출력] 원음(AMP/√2) 대비 dB\n  주파수 | 사운드2→CH1(서브) | 사운드2→CH2 | 사운드1→CH1(서브)");
    let src = AMP / 2.0f32.sqrt();
    for f in FREQS {
        let (s2_sub, s2_main) = measure(spk, f, MAIN);
        let (s1_sub, _) = measure(spk, f, SUB);
        println!("  {f:>6.0} | {:>+8.1} | {:>+8.1} | {:>+8.1}", db(s2_sub / src), db(s2_main / src), db(s1_sub / src));
    }
}

/// 실제 트랙 파일로 CH1(서브)·CH2 출력 크기를 잰다(수동 진단).
///   DIAG_AUDIO_DIR에 sound1_CH1_guitar.f32 / sound2_CH2_dance.f32(인터리브 f32, 44.1kHz 2채널).
#[test]
#[ignore]
fn 진단_실제_트랙_파일() {
    let dir = std::env::var("DIAG_AUDIO_DIR").expect("DIAG_AUDIO_DIR");
    let load = |name: &str| {
        let bytes = std::fs::read(format!("{dir}/{name}.f32")).expect("파일");
        let samples: Vec<f32> = bytes.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect();
        Arc::new(SoundData { samples, channels: 2, sample_rate: 44_100 })
    };
    let dbfs = |x: f32| 20.0 * x.max(1e-9).log10();
    // 스피커 출력(바이노럴 끔): 10초 재생, 2~10초 구간의 (CH1, CH2) RMS.
    let speaker_rms = |m: &mut AudioMixer| {
        let mut buf = vec![0.0f32; CH * BLOCK];
        let (mut a, mut b, mut n) = (0.0f64, 0.0f64, 0usize);
        let blocks = (FS as usize * 10) / BLOCK;
        for i in 0..blocks {
            buf.fill(0.0);
            m.process(&mut buf, CH);
            if i >= blocks / 5 {
                for f in 0..BLOCK {
                    a += (buf[f * CH + SUB] as f64).powi(2);
                    b += (buf[f * CH + MAIN] as f64).powi(2);
                    n += 1;
                }
            }
        }
        (((a / n as f64).sqrt()) as f32, ((b / n as f64).sqrt()) as f32)
    };
    let spk = Scenario { binaural: false, sub_on: true, lfe_boost: false, tuning: true, er: false };
    let guitar = load("sound1_CH1_guitar");
    let dance = load("sound2_CH2_dance");

    let (sub, main) = speaker_rms(&mut mixer_playing(spk, dance.clone(), MAIN));
    println!("사운드2(댄스)만 CH2 재생 -> CH1(서브) {:+.1} dBFS | CH2 {:+.1} dBFS", dbfs(sub), dbfs(main));
    let (sub, _) = speaker_rms(&mut mixer_playing(spk, guitar.clone(), SUB));
    println!("사운드1(기타)만 CH1 재생 -> CH1(서브) {:+.1} dBFS", dbfs(sub));
    let (sub, main) = speaker_rms(&mut mixer_playing(Scenario { sub_on: false, ..spk }, dance.clone(), MAIN));
    println!("서브 지정 해제, 사운드2 CH2 재생 -> CH1 {:+.1} dBFS | CH2 {:+.1} dBFS", dbfs(sub), dbfs(main));

    let n = FS as usize * 6;
    let tone: Vec<f32> = (0..n).map(|i| 0.1 * (std::f32::consts::TAU * 50.0 * i as f32 / FS as f32).sin()).collect();
    let tone = Arc::new(SoundData { samples: tone, channels: 1, sample_rate: FS });
    let (sub, main) = speaker_rms(&mut mixer_playing(spk, tone, MAIN));
    println!("50Hz 톤(-23dBFS RMS) CH2 재생 -> CH1(서브) {:+.1} dBFS | CH2 {:+.1} dBFS", dbfs(sub), dbfs(main));

    let mut m = mixer_playing(spk, dance, MAIN);
    for c in m.crossovers.iter_mut() {
        c.set_target_freq(120.0, FS as f32);
    }
    let (sub, main) = speaker_rms(&mut m);
    println!("크로스오버 120Hz, 사운드2 CH2 재생 -> CH1(서브) {:+.1} dBFS | CH2 {:+.1} dBFS", dbfs(sub), dbfs(main));
}

/// 헤드폰 미리듣기 음량이 거리 흉내(전파 지연·감쇠) 때문에 얼마나 바뀌었는지 잰다(수동 진단).
///   DIAG_AUDIO_DIR은 위와 같다. 사운드2(댄스)를 CH2로 틀었을 때의 헤드폰 L+R RMS.
#[test]
#[ignore]
fn 진단_미리듣기_음량() {
    let dir = std::env::var("DIAG_AUDIO_DIR").expect("DIAG_AUDIO_DIR");
    let bytes = std::fs::read(format!("{dir}/sound2_CH2_dance.f32")).expect("파일");
    let samples: Vec<f32> = bytes.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect();
    let dance = Arc::new(SoundData { samples, channels: 2, sample_rate: 44_100 });
    let dbfs = |x: f64| 20.0 * x.max(1e-12).log10();
    // 10초 재생, 2~10초 구간 RMS: (헤드폰 L+R 또는 CH1, CH2)
    let run = |m: &mut AudioMixer| {
        let mut buf = vec![0.0f32; CH * BLOCK];
        let (mut a, mut b, mut n) = (0.0f64, 0.0f64, 0usize);
        let blocks = (FS as usize * 10) / BLOCK;
        for i in 0..blocks {
            buf.fill(0.0);
            m.process(&mut buf, CH);
            if i >= blocks / 5 {
                for f in 0..BLOCK {
                    a += (buf[f * CH] as f64).powi(2);
                    b += (buf[f * CH + 1] as f64).powi(2);
                    n += 1;
                }
            }
        }
        ((a / n as f64).sqrt(), (b / n as f64).sqrt())
    };
    let src = {
        let s = &dance.samples;
        let mono: f64 = s.as_chunks::<2>().0.iter().map(|c| (((c[0] + c[1]) * 0.5) as f64).powi(2)).sum::<f64>() / (s.len() / 2) as f64;
        mono.sqrt()
    };
    println!("원본(L+R)/2 RMS {:+.1} dBFS", dbfs(src));

    let (l, r) = run(&mut mixer_playing(HEADPHONES, dance.clone(), MAIN));
    println!("헤드폰(거리 흉내 켬) L {:+.1} / R {:+.1} dBFS", dbfs(l), dbfs(r));

    let mut m = mixer_playing(HEADPHONES, dance.clone(), MAIN);
    for ch in 0..CH {
        m.binaural.clear_channel_propagation(ch);
    }
    let (l, r) = run(&mut m);
    println!("헤드폰(거리 흉내 없음) L {:+.1} / R {:+.1} dBFS", dbfs(l), dbfs(r));

    let (a, b) = run(&mut mixer_playing(Scenario { binaural: false, ..HEADPHONES }, dance, MAIN));
    println!("바이노럴 끔: CH1(서브) {:+.1} / CH2 {:+.1} dBFS", dbfs(a), dbfs(b));
}
