//! 재생 중 무음 경고(core::show_state). 전역 상태를 쓰므로 테스트는 하나다.
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::app_signals::SILENT_WARNINGS;
use rust_lib_atmos_mixer_pro::core::show_state::{check_silence, SilenceWatch};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;

fn track(id: &str, is_loop: bool) -> TrackConfig {
    TrackConfig {
        id: id.into(),
        name: id.into(),
        file_path: format!("/{id}.wav"),
        volume: 1.0,
        is_loop,
        is_streaming: false,
        output_channel: 0,
        output_stereo: false,
        play_osc_address: String::new(),
        stop_osc_address: String::new(),
    }
}

fn set_peak(channel: usize, value: f32) {
    GLOBAL_STATE.vu_levels[channel].store(value.to_bits(), Ordering::Relaxed);
}

#[test]
fn 루프_재생_중_출력이_1분_넘게_0이면_경고_횟수를_올린다() {
    *GLOBAL_STATE.config.write().unwrap() = Some(AppConfig {
        rooms: vec![RoomConfig {
            id: "a".into(),
            name: "a".into(),
            color_hex: "#ffffff".into(),
            volume: 1.0,
            volume_osc_address: String::new(),
            clear_osc_address: String::new(),
            tracks: vec![track("la", true), track("oa", false)],
        }],
        ..AppConfig::default()
    });
    GLOBAL_STATE.clear_playing_tracks();
    GLOBAL_STATE.active_device_channels.store(2, Ordering::Relaxed);
    set_peak(0, 0.0);
    set_peak(1, 0.0);
    GLOBAL_STATE.master_mute.store(false, Ordering::Relaxed);
    let before = SILENT_WARNINGS.load(Ordering::Relaxed);
    let mut w = SilenceWatch::default();

    // 단발만 돌면 경고하지 않는다(단발 사이 정적은 정상)
    GLOBAL_STATE.add_playing_track(1, "oa".into());
    assert_eq!(check_silence(&mut w, 0), None);
    assert_eq!(check_silence(&mut w, 120_000), None);

    // 루프가 도는데 출력이 0
    GLOBAL_STATE.add_playing_track(2, "la".into());
    assert_eq!(check_silence(&mut w, 125_000), None, "막 무음이 시작됐다");
    assert_eq!(check_silence(&mut w, 185_000), Some(before + 1), "60초 무음");
    assert_eq!(SILENT_WARNINGS.load(Ordering::Relaxed), before + 1, "하트비트가 알릴 횟수");
    assert_eq!(check_silence(&mut w, 190_000), None, "10분 안에는 다시 경고하지 않는다");

    // All Mute면 경고하지 않고, 풀린 뒤 다시 잰다
    GLOBAL_STATE.master_mute.store(true, Ordering::Relaxed);
    assert_eq!(check_silence(&mut w, 300_000), None);
    GLOBAL_STATE.master_mute.store(false, Ordering::Relaxed);
    assert_eq!(check_silence(&mut w, 305_000), None);
    assert_eq!(check_silence(&mut w, 365_000), Some(before + 2));

    // 소리가 나면 처음부터 다시 잰다
    set_peak(1, 0.02);
    assert_eq!(check_silence(&mut w, 370_000), None);
    set_peak(1, 0.0);
    assert_eq!(check_silence(&mut w, 375_000), None);
    assert_eq!(check_silence(&mut w, 435_000), Some(before + 3));

    GLOBAL_STATE.clear_playing_tracks();
    GLOBAL_STATE.active_device_channels.store(0, Ordering::Relaxed);
}
