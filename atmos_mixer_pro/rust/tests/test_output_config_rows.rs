//! 출력 설정 = 줄마다 따로(Ableton처럼, 사용자 결정 2026-11-22 "이전처럼 에이블톤처럼 따로 하자",
//! 설계 docs/superpowers/specs/2026-11-22-output-config-separate-rows-design.md).
//! 트랙은 자기 출력 종류의 줄(모노 CHk·스테레오 k/k+1·멀티 CHk 시작)이 켜져 있어야 소리를 낸다. 1.3.15·1.3.16은 세 줄을
//! "열린 채널" 하나로 묶어, 모노 CH1·CH2만 켜도 스테레오 1/2 자리가 맞았고 멀티 CHk는 CHk 하나만 열었다.
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, ChannelSetting, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::output_groups::{
    apply_row_tables, open_channels_with, required_group, route_open_in, row_enabled, GroupKind,
};
use rust_lib_atmos_mixer_pro::core::processing_channels::processing_channel_count;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const FS: u32 = 48_000;
const BLOCK: usize = 480;
const OUT: usize = 8;

/// 전역 줄 표·게이트를 건드리는 시험은 차례로 돈다(한 실행 파일 안 시험은 동시에 돈다).
static GLOBAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_global() -> std::sync::MutexGuard<'static, ()> {
    GLOBAL_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn setting(enabled: bool) -> ChannelSetting {
    ChannelSetting {
        enabled,
        custom_name: String::new(),
        delay_ms: 0.0,
        eq_bands: Vec::new(),
        position: None,
        phase_invert: false,
        gain_db: 0.0,
    }
}

fn track(file_path: &str, output_channel: u32, output_stereo: bool) -> TrackConfig {
    TrackConfig {
        id: file_path.into(),
        name: file_path.into(),
        file_path: file_path.into(),
        volume: 1.0,
        is_loop: true,
        is_streaming: false,
        output_channel,
        output_stereo,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
        routes: vec![],
    }
}

fn with_tracks(mut config: AppConfig, tracks: Vec<TrackConfig>) -> AppConfig {
    config.rooms.push(RoomConfig {
        id: "r".into(),
        name: "r".into(),
        color_hex: String::new(),
        volume: 1.0,
        clear_osc_address: String::new(),
        tracks,
    });
    config
}

/// 시험용 파일 채널 수: 경로 이름이 채널 수.
fn fake_channels(path: &str) -> Option<usize> {
    path.trim_end_matches(".wav").parse().ok()
}

#[test]
fn 켠_줄이_닿는_채널을_합쳐_채널_게이트를_만든다() {
    let mut config = AppConfig::default();
    config.mono_configs.insert(3, setting(true)); // CH3
    config.stereo_configs.insert(5, setting(true)); // CH5·CH6
    config.multi_configs.insert(1, setting(true)); // CH1부터 4채널 트랙만큼
    config.multi_configs.insert(8, setting(false)); // 꺼짐
    config.mono_configs.insert(0, setting(true)); // 0번 키는 쓰지 않는다
    config.mono_configs.insert(255, setting(true)); // 장치 밖
    let config = with_tracks(config, vec![track("4.wav", 0, true), track("2.wav", 0, true)]);
    assert_eq!(
        open_channels_with(&config, 8, &fake_channels),
        vec![true, true, true, true, true, true, false, false],
        "멀티 CH1 시작은 그 자리의 4채널 트랙만큼 CH1~4를 연다(모노 줄을 켜지 않아도)"
    );
    assert_eq!(open_channels_with(&AppConfig::default(), 4, &fake_channels), vec![false; 4], "켠 것이 없으면 모두 닫힘");
}

#[test]
fn 멀티_줄은_그_자리_트랙이_없거나_모노로_합치면_시작_채널_하나만_연다() {
    let mut config = AppConfig::default();
    config.multi_configs.insert(2, setting(true)); // CH2 시작, 그 자리 트랙 없음
    config.multi_configs.insert(5, setting(true)); // CH5 시작, 스테레오 출력을 끈 6채널 트랙(믹서가 모노로 합친다)
    let config = with_tracks(config, vec![track("6.wav", 4, false), track("24.wav", 0, true)]);
    assert_eq!(
        open_channels_with(&config, 8, &fake_channels),
        vec![false, true, false, false, true, false, false, false],
        "CH1의 24채널 트랙은 멀티 CH1 줄이 꺼져 있어 열지 않는다"
    );
}

#[test]
fn 파일_채널_수와_스테레오_출력으로_자리_종류를_정한다() {
    // 화면 buildChannelRoutingItems와 같은 규칙: 다채널 파일은 멀티 자리, 모노 파일은 모노 자리(새 트랙 기본값
    // "스테레오 출력 켬"이어도 같은 채널의 모노), 스테레오 파일은 스테레오 출력이면 스테레오·아니면 모노 자리.
    assert_eq!(required_group(24, true), GroupKind::Multi);
    assert_eq!(required_group(4, false), GroupKind::Multi);
    assert_eq!(required_group(1, false), GroupKind::Mono);
    assert_eq!(required_group(1, true), GroupKind::Mono);
    assert_eq!(required_group(2, true), GroupKind::Stereo);
    assert_eq!(required_group(2, false), GroupKind::Mono);
}

#[test]
fn 자리는_자기_종류의_줄이_켜져_있어야_한다() {
    // 켠 줄: 모노 CH1·CH2, 스테레오 3/4, 멀티 CH5 시작.
    let row_on = |kind: GroupKind, start: usize| match kind {
        GroupKind::Mono => start == 0 || start == 1,
        GroupKind::Stereo => start == 2,
        GroupKind::Multi => start == 4,
    };
    assert!(route_open_in(row_on, GroupKind::Mono, 1, OUT), "모노 CH2");
    assert!(!route_open_in(row_on, GroupKind::Stereo, 0, OUT), "모노 CH1·CH2를 켜도 스테레오 1/2 줄은 꺼져 있다");
    assert!(route_open_in(row_on, GroupKind::Stereo, 2, OUT), "스테레오 3/4");
    assert!(!route_open_in(row_on, GroupKind::Mono, 2, OUT), "스테레오 3/4를 켜도 모노 CH3 줄은 꺼져 있다");
    assert!(route_open_in(row_on, GroupKind::Multi, 4, OUT), "멀티 CH5 시작");
    assert!(!route_open_in(row_on, GroupKind::Multi, 0, OUT), "모노 CH1을 켜도 멀티 CH1 시작 줄은 꺼져 있다");
    assert!(!route_open_in(row_on, GroupKind::Mono, 1, 1), "장치 밖");

    // 스테레오는 화면의 짝(1/2·3/4…)만, 짝 채널도 장치 안이어야 한다.
    let all_on = |_: GroupKind, _: usize| true;
    assert!(!route_open_in(all_on, GroupKind::Stereo, 1, OUT), "2/3은 스테레오 짝이 아니다");
    assert!(!route_open_in(all_on, GroupKind::Stereo, 2, 3), "CH4가 장치 밖");
}

#[test]
fn 설정의_줄을_종류별_표로_옮긴다() {
    let mut config = AppConfig::default();
    config.mono_configs.insert(2, setting(true));
    config.stereo_configs.insert(1, setting(false));
    config.stereo_configs.insert(3, setting(true));
    config.multi_configs.insert(1, setting(true));
    assert!(row_enabled(&config, GroupKind::Mono, 1));
    assert!(!row_enabled(&config, GroupKind::Mono, 0));
    assert!(!row_enabled(&config, GroupKind::Stereo, 0), "꺼진 줄");
    assert!(row_enabled(&config, GroupKind::Stereo, 2));
    assert!(row_enabled(&config, GroupKind::Multi, 0));

    let _guard = lock_global();
    apply_row_tables(&config);
    let on = |table: &Vec<std::sync::atomic::AtomicBool>, i: usize| table[i].load(Ordering::SeqCst);
    assert!(on(&GLOBAL_STATE.mono_rows_open, 1) && !on(&GLOBAL_STATE.mono_rows_open, 0));
    assert!(!on(&GLOBAL_STATE.stereo_rows_open, 0) && on(&GLOBAL_STATE.stereo_rows_open, 2));
    assert!(on(&GLOBAL_STATE.multi_rows_open, 0) && !on(&GLOBAL_STATE.multi_rows_open, 1));
    restore_all_rows();
}

#[test]
fn 처리_폭은_멀티_자리_트랙의_파일_채널_수까지_본다() {
    // 메모리에 읽은 음원이면 파일을 다시 읽지 않고 그 채널 수를 쓴다.
    let path = "memory://test_output_config_rows/24ch.wav";
    GLOBAL_STATE.preloaded_sounds.write().unwrap_or_else(|e| e.into_inner()).insert(
        path.to_string(),
        Arc::new(SoundData { samples: vec![0.0; 24], channels: 24, sample_rate: FS }),
    );
    let mut config = AppConfig::default();
    config.mono_configs.insert(1, setting(true));
    config.mono_configs.insert(2, setting(true));
    config.multi_configs.insert(1, setting(true));
    let config = with_tracks(config, vec![track(path, 0, true)]);
    assert_eq!(processing_channel_count(&config, 94), 24, "멀티 CH1 시작의 24채널 트랙 → CH1~24");
    assert_eq!(processing_channel_count(&config, 12), 12, "장치 채널 수를 넘지 않는다");
}

/// 다른 시험이 기본값(모두 열림)을 기대하므로 줄 표를 되돌린다.
fn restore_all_rows() {
    for table in [&GLOBAL_STATE.mono_rows_open, &GLOBAL_STATE.stereo_rows_open, &GLOBAL_STATE.multi_rows_open] {
        for slot in table.iter() {
            slot.store(true, Ordering::SeqCst);
        }
    }
}

fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0.0f32, |m, v| m.max(v.abs()))
}

/// 블록 하나를 돌리고 (전체 최대 진폭, CH1의 인접 샘플 간 최대 변화량)을 돌려준다.
fn block(mixer: &mut AudioMixer, buf: &mut [f32], prev_ch1: &mut f32) -> (f32, f32) {
    buf.fill(0.0);
    mixer.process(buf, OUT);
    let mut jump = 0.0f32;
    for frame in 0..BLOCK {
        let y = buf[frame * OUT];
        jump = jump.max((y - *prev_ch1).abs());
        *prev_ch1 = y;
    }
    (peak(buf), jump)
}

#[test]
fn 자리의_줄이_꺼진_트랙은_무음이고_위치는_흐르며_줄을_켜면_딸깍_없이_들린다() {
    // 스테레오 파일을 스테레오 1/2 자리에 둔다. 채널 게이트는 모두 열려 있고(모노 CH1·CH2가 켜진 것과 같다), 스테레오
    // 1/2 줄만 꺼져 있다 — Ableton처럼 모노 줄은 스테레오 자리를 열지 않는다(화면에 안내가 뜬다).
    let _guard = lock_global();
    for slot in GLOBAL_STATE.enabled_channels.iter() {
        slot.store(true, Ordering::SeqCst);
    }
    restore_all_rows();
    GLOBAL_STATE.stereo_rows_open[0].store(false, Ordering::SeqCst);
    let (gc_tx, _gc_rx) = crossbeam_channel::bounded(16);
    let mut mixer = AudioMixer::new(FS, OUT, BLOCK, gc_tx, None);
    mixer.startup_ramp.current_gain = 1.0; // 앱 시작 뮤트 램프는 이 시험의 대상이 아니다
    // 1kHz 코사인(블록 480프레임 = 10주기, 블록 첫 샘플이 늘 마루라 한 번에 뛰면 드러난다).
    let frames = 4800;
    let mut samples = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let v = 0.5 * (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / FS as f32).cos();
        samples.push(v);
        samples.push(v);
    }
    let data = Arc::new(SoundData { samples, channels: 2, sample_rate: FS });
    mixer.instances[0] = Some(SoundInstance::new(
        1, 7, 1, "t".into(), Some(data), None, FS, 2, true, 1.0, 0, true, None,
        GLOBAL_STATE.enabled_channels.len(),
    ));
    let mut buf = vec![0.0f32; BLOCK * OUT];
    let mut prev = 0.0f32;

    // 줄이 꺼진 동안: 처음부터 무음(열린 CH1·CH2로도 새어 나오지 않는다).
    let mut loudest = 0.0f32;
    for _ in 0..200 {
        loudest = loudest.max(block(&mut mixer, &mut buf, &mut prev).0);
    }
    assert!(loudest < 1e-6, "스테레오 1/2 줄이 꺼져 있는데 소리가 났다: {loudest}");

    // 무음이어도 재생 위치는 흐른다(루프 한 바퀴가 10블록이라 7블록만 본다).
    let before = mixer.instances[0].as_ref().map(|i| i.position_seconds()).unwrap_or(0.0);
    for _ in 0..7 {
        block(&mut mixer, &mut buf, &mut prev);
    }
    let after = mixer.instances[0].as_ref().map(|i| i.position_seconds()).unwrap_or(0.0);
    assert!(after != before, "무음인 동안 재생 위치가 멈췄다");

    // 스테레오 1/2 줄을 켠다: 서서히 커진다(CH1에서 한 번에 뛰지 않는다).
    GLOBAL_STATE.stereo_rows_open[0].store(true, Ordering::SeqCst);
    let mut worst_jump = 0.0f32;
    for _ in 0..5 {
        worst_jump = worst_jump.max(block(&mut mixer, &mut buf, &mut prev).1);
    }
    let mut steady_jump = 0.0f32;
    let mut steady_peak = 0.0f32;
    for _ in 0..50 {
        let (p, j) = block(&mut mixer, &mut buf, &mut prev);
        steady_jump = steady_jump.max(j);
        steady_peak = steady_peak.max(p);
    }
    assert!(steady_peak > 0.1, "줄을 켰는데 소리가 나지 않는다: {steady_peak}");
    assert!(worst_jump <= steady_jump * 2.0, "줄을 켤 때 CH1이 튀었다: 전환 {worst_jump}, 정상 {steady_jump}");

    // 다시 끈다: 서서히 줄어 무음이 된다.
    GLOBAL_STATE.stereo_rows_open[0].store(false, Ordering::SeqCst);
    let mut worst_jump = 0.0f32;
    for _ in 0..5 {
        worst_jump = worst_jump.max(block(&mut mixer, &mut buf, &mut prev).1);
    }
    assert!(worst_jump <= steady_jump * 2.0, "줄을 끌 때 CH1이 튀었다: {worst_jump}");
    // 줄어드는 순간 생긴 극저역이 채널 DSP 끝의 직류 차단기(0.5Hz)에서 천천히 빠진다(-100dB 안팎). 3초 뒤의 크기를 본다.
    for _ in 0..300 {
        block(&mut mixer, &mut buf, &mut prev);
    }
    let mut tail = 0.0f32;
    for _ in 0..20 {
        tail = tail.max(block(&mut mixer, &mut buf, &mut prev).0);
    }
    assert!(tail < 1e-6, "줄을 껐는데 소리가 남았다: {tail}");
    restore_all_rows();
}
