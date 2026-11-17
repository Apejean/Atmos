//! 루프 더킹 설정(켜기·끄기, 줄이는 양). 사용자 결정 2026-11-16 "더킹은 의도된 동작" — 기본은 지금과 같이 켜짐·0.3배.
//! - 설정 파일: 칸이 없으면 기본값(켜짐, 20·log10(0.3)dB)으로 읽고, 옛 버전은 새 칸을 무시한다(롤백 안전).
//! - 기본 설정에서 더킹 무게는 예전 코드(0.3 상수, 0.7 폭)와 비트 단위로 같다.
//! - 더킹 중에 끄거나 양을 바꿔도 무게가 한 번에 뛰지 않는다(내림 150ms·올림 300ms 기울기, 3법칙 3).
//! - 전시 모드는 늘 더킹하지 않는다. 화면 표시는 오디오 밖에서 원자값을 보고 갱신한다.

use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::common::config::AppConfig;
use rust_lib_atmos_mixer_pro::core::ducking;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

const FS: u32 = 48_000;
const CH: usize = 8;
const BLOCK: usize = 512;

/// GLOBAL_STATE를 함께 쓰므로 이 파일의 시험은 하나씩 돈다.
static SERIAL: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

fn config(enabled: bool, db: f32) -> AppConfig {
    AppConfig { ducking_enabled: enabled, ducking_reduction_db: db, ..AppConfig::default() }
}

fn instance(id: u64, is_loop: bool, channel: usize) -> SoundInstance {
    let samples = vec![0.05f32; FS as usize * 6];
    let data = Arc::new(SoundData { samples, channels: 1, sample_rate: FS });
    let mut inst = SoundInstance::new(
        id,
        id as u32,
        1,
        format!("t{id}"),
        Some(data),
        None,
        FS,
        1,
        is_loop,
        1.0,
        channel,
        false,
        None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.fade_weight = 1.0;
    inst
}

/// 루프 하나만 나는 믹서. `sfx`이면 비루프 트랙도 하나 더 난다(더킹 조건).
fn mixer(sfx: bool) -> AudioMixer {
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    let mut m = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    m.startup_ramp.current_gain = 1.0;
    m.instances[0] = Some(instance(1, true, 2));
    if sfx {
        m.instances[1] = Some(instance(2, false, 3));
    }
    m
}

fn render(m: &mut AudioMixer, blocks: usize) {
    let mut buf = vec![0.0f32; CH * BLOCK];
    for _ in 0..blocks {
        buf.fill(0.0);
        m.process(&mut buf, CH);
    }
}

fn reset_state() {
    GLOBAL_STATE.is_exhibition_mode.store(false, Ordering::SeqCst);
    ducking::apply_settings(&AppConfig::default());
    GLOBAL_STATE.ducking_engaged.store(false, Ordering::SeqCst);
}

/// 프레임마다 더한 f32의 반올림이 블록(512프레임) 동안 쌓이는 만큼의 여유. 한 번에 뛰면 이보다 수십 배 크다.
const ACCUM_TOL: f32 = 1e-4;

/// 한 블록에서 무게가 움직일 수 있는 최대 폭(올림 300ms 기울기, 내림은 이보다 빠르다).
fn max_up_per_block() -> f32 {
    BLOCK as f32 * 0.7 / (FS as f32 * 0.3) + ACCUM_TOL
}

#[test]
fn default_config_keeps_current_ducking() {
    let c = AppConfig::default();
    assert!(c.ducking_enabled, "기본은 지금처럼 켜짐");
    assert_eq!(c.ducking_reduction_db, ducking::DEFAULT_REDUCTION_DB);
    assert!((ducking::DEFAULT_REDUCTION_DB - 20.0 * 0.3f32.log10()).abs() < 1e-4);
    assert_eq!(ducking::floor_gain(ducking::DEFAULT_REDUCTION_DB).to_bits(), 0.3f32.to_bits());
}

#[test]
fn old_config_without_fields_reads_defaults() {
    let mut v = serde_json::to_value(config(false, -3.0)).expect("직렬화");
    let obj = v.as_object_mut().expect("객체");
    assert!(obj.remove("ducking_enabled").is_some());
    assert!(obj.remove("ducking_reduction_db").is_some());
    let c: AppConfig = serde_json::from_value(v).expect("옛 설정 읽기");
    assert!(c.ducking_enabled);
    assert_eq!(c.ducking_reduction_db, ducking::DEFAULT_REDUCTION_DB);
}

#[test]
fn unknown_fields_are_ignored_so_older_versions_can_read_new_files() {
    // 옛 버전(1.3.12)의 AppConfig도 deny_unknown_fields가 없다. 새 칸을 모르는 쪽이 읽어도 실패하지 않는지 같은 규칙으로 본다.
    let mut v = serde_json::to_value(AppConfig::default()).expect("직렬화");
    v.as_object_mut().expect("객체").insert("future_field".into(), serde_json::json!(42));
    let _: AppConfig = serde_json::from_value(v).expect("모르는 칸 무시");
}

#[test]
fn floor_gain_converts_and_clamps() {
    assert!((ducking::floor_gain(-6.0) - 0.501_187).abs() < 1e-5);
    assert_eq!(ducking::floor_gain(0.0), 1.0);
    assert_eq!(ducking::floor_gain(5.0), 1.0, "0dB 위는 0dB");
    assert!((ducking::floor_gain(-100.0) - 0.01).abs() < 1e-6, "−40dB 아래는 −40dB");
    assert_eq!(ducking::floor_gain(f32::NAN).to_bits(), 0.3f32.to_bits(), "숫자가 아니면 기본");
}

#[test]
fn default_weight_sequence_is_bit_identical_to_old_code() {
    let down = (FS as f32 * 0.15) as usize;
    let up = (FS as f32 * 0.3) as usize;
    let (mut old, mut new) = (1.0f32, 1.0f32);
    for frame in 0..(FS as usize * 2) {
        let ducking_now = frame < FS as usize; // 1초 더킹, 1초 풀기
        // 예전 mixer.rs 코드 그대로.
        if ducking_now {
            if old > 0.3 {
                old -= 0.7 / down as f32;
            }
            if old < 0.3 {
                old = 0.3;
            }
        } else {
            if old < 1.0 {
                old += 0.7 / up as f32;
            }
            if old > 1.0 {
                old = 1.0;
            }
        }
        new = ducking::step_weight(new, ducking_now, 0.3, down, up);
        assert_eq!(new.to_bits(), old.to_bits(), "프레임 {frame}");
    }
}

#[test]
fn enabled_by_default_ducks_loops_and_raises_indicator() {
    let _g = lock();
    reset_state();
    let mut m = mixer(true);
    render(&mut m, 20); // 약 0.21초 > 150ms
    assert_eq!(m.ducking.ducking_weight.to_bits(), 0.3f32.to_bits());
    assert!(GLOBAL_STATE.ducking_engaged.load(Ordering::SeqCst));
}

#[test]
fn disabled_setting_never_ducks() {
    let _g = lock();
    reset_state();
    ducking::apply_settings(&config(false, ducking::DEFAULT_REDUCTION_DB));
    let mut m = mixer(true);
    render(&mut m, 20);
    assert_eq!(m.ducking.ducking_weight, 1.0);
    assert!(!GLOBAL_STATE.ducking_engaged.load(Ordering::SeqCst));
    reset_state();
}

#[test]
fn exhibition_mode_never_ducks_even_when_enabled() {
    let _g = lock();
    reset_state();
    GLOBAL_STATE.is_exhibition_mode.store(true, Ordering::SeqCst);
    let mut m = mixer(true);
    render(&mut m, 20);
    assert_eq!(m.ducking.ducking_weight, 1.0);
    reset_state();
}

#[test]
fn turning_off_while_ducked_ramps_up_without_jump() {
    let _g = lock();
    reset_state();
    let mut m = mixer(true);
    render(&mut m, 20);
    assert_eq!(m.ducking.ducking_weight.to_bits(), 0.3f32.to_bits());
    ducking::apply_settings(&config(false, ducking::DEFAULT_REDUCTION_DB));
    let mut prev = m.ducking.ducking_weight;
    for _ in 0..40 {
        render(&mut m, 1);
        let w = m.ducking.ducking_weight;
        assert!(w >= prev && w - prev <= max_up_per_block(), "한 블록에 {prev} → {w}");
        prev = w;
    }
    assert_eq!(prev, 1.0, "약 0.43초 뒤에는 원래 크기");
    assert!(!GLOBAL_STATE.ducking_engaged.load(Ordering::SeqCst));
    reset_state();
}

#[test]
fn changing_amount_while_ducked_moves_gradually_to_new_floor() {
    let _g = lock();
    reset_state();
    let mut m = mixer(true);
    render(&mut m, 20);
    // 덜 줄이기(−6dB): 0.3 → 약 0.501로 천천히 올라간다.
    ducking::apply_settings(&config(true, -6.0));
    let target = ducking::floor_gain(-6.0);
    let mut prev = m.ducking.ducking_weight;
    for _ in 0..30 {
        render(&mut m, 1);
        let w = m.ducking.ducking_weight;
        assert!(w >= prev && w - prev <= max_up_per_block(), "한 블록에 {prev} → {w}");
        assert!(w <= target + 1e-6);
        prev = w;
    }
    assert_eq!(prev, target);
    // 더 줄이기(−20dB): 0.1까지 내림 기울기로 내려간다.
    ducking::apply_settings(&config(true, -20.0));
    let target = ducking::floor_gain(-20.0);
    let max_down = BLOCK as f32 * 0.7 / (FS as f32 * 0.15) + ACCUM_TOL;
    for _ in 0..30 {
        render(&mut m, 1);
        let w = m.ducking.ducking_weight;
        assert!(w <= prev && prev - w <= max_down, "한 블록에 {prev} → {w}");
        assert!(w >= target - 1e-6);
        prev = w;
    }
    assert_eq!(prev, target);
    reset_state();
}

/// 감시 스레드(apply_settings가 띄움)도 같은 함수를 부르므로, 누가 먼저 옮겼든 결과만 본다.
#[test]
fn indicator_follows_engaged_state_and_skips_when_unchanged() {
    let _g = lock();
    reset_state();
    for engaged in [true, false] {
        GLOBAL_STATE.ducking_engaged.store(engaged, Ordering::SeqCst);
        let _ = ducking::sync_indicator();
        assert_eq!(GLOBAL_STATE.is_ducking.load(Ordering::SeqCst), engaged);
        assert!(!ducking::sync_indicator(), "같으면 알리지 않는다");
    }
}

#[test]
fn indicator_watch_thread_updates_screen_state_without_caller() {
    let _g = lock();
    reset_state(); // apply_settings가 감시 스레드를 띄운다.
    GLOBAL_STATE.ducking_engaged.store(true, Ordering::SeqCst);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !GLOBAL_STATE.is_ducking.load(Ordering::SeqCst) {
        assert!(std::time::Instant::now() < deadline, "감시 스레드가 표시를 켜지 않았다");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    GLOBAL_STATE.ducking_engaged.store(false, Ordering::SeqCst);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while GLOBAL_STATE.is_ducking.load(Ordering::SeqCst) {
        assert!(std::time::Instant::now() < deadline, "감시 스레드가 표시를 끄지 않았다");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
