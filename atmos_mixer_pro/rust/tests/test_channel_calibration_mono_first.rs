//! 채널 보정값(지연·EQ·위상·게인·위치)은 그 채널의 모노 줄 값이 이긴다(2026-11-22 사용자 "8,9번은 지금 진행").
//!
//! 출력 설정의 스테레오·멀티 줄을 새로 켜면 그 항목은 기본값(지연 0·게인 0·위치 없음)으로 생긴다. 엔진은 세 맵을 모노 →
//! 스테레오 → 멀티 순서로 덮어 적용해, 새로 켠 줄의 기본값이 그 채널의 모노 보정값을 지웠다(엔진 시작·설정 불러오기).
//! 스테레오·멀티 줄 값은 모노 줄이 없는 채널에만 쓴다(예전 설정 호환).
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, ChannelSetting, EqBand, EqType, Point3D};
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;

/// 전역 설정·명령 통로를 건드리는 시험은 차례로 돈다.
static GLOBAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_global() -> std::sync::MutexGuard<'static, ()> {
    GLOBAL_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn row(delay_ms: f32, gain_db: f32, phase_invert: bool, position: Option<Point3D>, eq_bands: Vec<EqBand>) -> ChannelSetting {
    ChannelSetting {
        enabled: true,
        custom_name: String::new(),
        delay_ms,
        eq_bands,
        position,
        phase_invert,
        gain_db,
    }
}

/// 출력 설정 창이 새로 켠 줄(`setOutputRow`의 기본 항목)과 같다.
fn fresh_row() -> ChannelSetting {
    row(0.0, 0.0, false, None, Vec::new())
}

fn point(x: f32) -> Point3D {
    Point3D { x, y: 2.0, z: 1.5, yaw_rotation: 0.0, pitch_tilt: 0.0, dispersion_angle: 0.0, size: 0.0 }
}

fn bell(freq: f32, gain: f32) -> EqBand {
    EqBand { enabled: true, freq, gain, q_factor: 1.0, filter_type: EqType::Bell, slope_db_per_oct: 12 }
}

/// 모노 CH1·CH2는 보정됨, 그 위에 스테레오 1/2와 멀티 CH1 시작을 새로 켬. CH5·CH6은 모노 줄 없이 스테레오 5/6 값만 있음
/// (예전 설정).
fn calibrated_config() -> AppConfig {
    let mut config = AppConfig::default();
    config.mono_configs.insert(1, row(5.0, -6.0, true, Some(point(1.0)), vec![bell(250.0, -4.0)]));
    config.mono_configs.insert(2, row(3.0, -3.0, false, Some(point(2.0)), Vec::new()));
    config.stereo_configs.insert(1, fresh_row());
    config.multi_configs.insert(1, fresh_row());
    config.stereo_configs.insert(5, row(7.0, -9.0, false, Some(point(5.0)), Vec::new()));
    config
}

fn linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

#[test]
fn engine_start_keeps_mono_row_calibration_under_new_stereo_and_multi_rows() {
    let _guard = lock_global();
    *GLOBAL_STATE.config.write().unwrap_or_else(|e| e.into_inner()) = Some(calibrated_config());
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    let mixer = AudioMixer::new(48_000, 8, 512, gc_tx, None);
    *GLOBAL_STATE.config.write().unwrap_or_else(|e| e.into_inner()) = None;

    let ch1 = &mixer.channel_dsp[0];
    assert_eq!(ch1.target_delay_ms, 5.0, "CH1 지연은 모노 줄 값");
    assert!((ch1.target_gain_linear - linear(-6.0)).abs() < 1e-6, "CH1 게인은 모노 줄 값");
    assert!(ch1.phase_invert, "CH1 위상 반전은 모노 줄 값");
    assert!(
        ch1.target_bands.iter().any(|b| b.enabled && b.freq == 250.0 && b.gain == -4.0),
        "CH1 EQ는 모노 줄 값(250Hz -4dB)"
    );
    assert_eq!(mixer.channel_positions[0].as_ref().map(|p| p.x), Some(1.0), "CH1 위치는 모노 줄 값");

    let ch2 = &mixer.channel_dsp[1];
    assert_eq!(ch2.target_delay_ms, 3.0, "CH2 지연은 모노 줄 값");
    assert!((ch2.target_gain_linear - linear(-3.0)).abs() < 1e-6, "CH2 게인은 모노 줄 값");
    assert_eq!(mixer.channel_positions[1].as_ref().map(|p| p.x), Some(2.0), "CH2 위치는 모노 줄 값");

    // 모노 줄이 없는 채널은 예전처럼 스테레오 줄 값을 쓴다.
    for ch in [4, 5] {
        assert_eq!(mixer.channel_dsp[ch].target_delay_ms, 7.0, "CH{} 지연은 스테레오 5/6 값", ch + 1);
        assert!((mixer.channel_dsp[ch].target_gain_linear - linear(-9.0)).abs() < 1e-6);
        assert_eq!(mixer.channel_positions[ch].as_ref().map(|p| p.x), Some(5.0));
    }
}

#[test]
fn loading_config_sends_mono_row_calibration_last() {
    let _guard = lock_global();
    let dir = std::env::temp_dir().join(format!("atmos_calib_mono_first_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.json");
    std::fs::write(&path, serde_json::to_string(&calibrated_config()).expect("json")).expect("write config");
    while GLOBAL_STATE.command_receiver.try_recv().is_ok() {}

    let _ = rust_lib_atmos_mixer_pro::api::simple::api_get_config(path.to_string_lossy().into_owned());

    // 믹서는 받은 순서대로 적용한다(engine.rs ApplyAllChannelTunings) — 채널마다 마지막 값이 남는다.
    let mut last: std::collections::HashMap<usize, (f32, f32, bool)> = std::collections::HashMap::new();
    while let Ok(command) = GLOBAL_STATE.command_receiver.try_recv() {
        if let AudioCommand::ApplyAllChannelTunings { tunings } = command {
            for (channel, delay_ms, _bands, phase_invert, gain_db) in tunings {
                last.insert(channel, (delay_ms, gain_db, phase_invert));
            }
        }
    }
    *GLOBAL_STATE.config.write().unwrap_or_else(|e| e.into_inner()) = None;
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(last.get(&0), Some(&(5.0, -6.0, true)), "CH1은 모노 줄 보정값으로 끝나야 한다");
    assert_eq!(last.get(&1), Some(&(3.0, -3.0, false)), "CH2는 모노 줄 보정값으로 끝나야 한다");
    assert_eq!(last.get(&4), Some(&(7.0, -9.0, false)), "모노 줄이 없는 CH5는 스테레오 5/6 값");
}
