//! 재시작 복원의 경쟁 상황. 전역 상태를 쓰므로 테스트는 하나다.
//! - 여러 스레드(OSC, 재개 스레드, FRB 워커)가 동시에 재생해도 인스턴스 번호가 겹치지 않는다.
//!   번호는 재생 목록과 커서 표의 열쇠라 겹치면 트랙 하나가 사라지거나 남의 위치로 재개된다.
//! - 재개가 도는 도중 전체 정지(Emergency)를 누르면 정지 뒤에 재생이 남지 않는다.
use rust_lib_atmos_mixer_pro::api::simple::{api_play_track, api_stop_all};
use rust_lib_atmos_mixer_pro::audio::engine::ENGINE_GENERATION;
use rust_lib_atmos_mixer_pro::audio::playback_cursor::CURSOR_TABLE;
use rust_lib_atmos_mixer_pro::audio::player::SoundData;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::restart_resume;
use rust_lib_atmos_mixer_pro::core::state::{next_instance_id, GLOBAL_STATE};
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

fn drain() -> Vec<AudioCommand> {
    let mut v = Vec::new();
    while let Ok(c) = GLOBAL_STATE.command_receiver.try_recv() {
        v.push(c);
    }
    v
}

#[test]
fn 동시_재생은_각자_번호를_받고_재개_도중_전체_정지하면_정지_뒤에_재생이_남지_않는다() {
    let fs = 48_000u32;
    let data = Arc::new(SoundData { samples: vec![0.1; fs as usize / 10], channels: 1, sample_rate: fs });
    GLOBAL_STATE.preloaded_sounds.write().unwrap().insert("/sfx.wav".into(), data);
    let mut config = AppConfig::default();
    config.rooms.push(RoomConfig {
        id: "r1".into(),
        name: "R1".into(),
        color_hex: "#ffffff".into(),
        volume: 1.0,
        volume_osc_address: String::new(),
        clear_osc_address: String::new(),
        tracks: vec![TrackConfig {
            id: "sfx".into(),
            name: "sfx".into(),
            file_path: "/sfx.wav".into(),
            volume: 1.0,
            is_loop: false,
            is_streaming: false,
            output_channel: 1,
            output_stereo: false,
            play_osc_address: String::new(),
            stop_osc_address: String::new(),
        }],
    });
    *GLOBAL_STATE.config.write().unwrap() = Some(config);
    GLOBAL_STATE.clear_playing_tracks();
    drain();

    // 1) 인스턴스 번호는 전역 카운터에서 받는다. 8개 스레드가 동시에 받아도 겹치지 않고, 커서 표의 빈 칸
    //    표시인 0도 아니다. 예전처럼 시계(나노초)를 쓰면 macOS 해상도가 1µs라 동시 재생에서 겹친다.
    let ids: Vec<u64> = (0..8)
        .map(|_| std::thread::spawn(|| (0..10_000).map(|_| next_instance_id()).collect::<Vec<u64>>()))
        .collect::<Vec<_>>()
        .into_iter()
        .flat_map(|h| h.join().unwrap())
        .collect();
    let unique: HashSet<u64> = ids.iter().copied().collect();
    assert_eq!(unique.len(), ids.len(), "동시에 받은 인스턴스 번호 {}개가 겹쳤다", ids.len() - unique.len());
    assert!(!unique.contains(&0), "인스턴스 번호 0(커서 표의 빈 칸 표시)이 나왔다");
    // 재생 API도 같은 카운터를 쓴다
    let before = next_instance_id();
    api_play_track("r1".into(), "sfx".into()).unwrap();
    let after = next_instance_id();
    let id = *GLOBAL_STATE.playing_track_ids.read().unwrap().keys().next().expect("재생 목록이 비었다");
    assert!(before < id && id < after, "재생 인스턴스 번호 {id}가 카운터({before}~{after})에서 오지 않았다");
    drain();

    // 2) 재개(300개)가 도는 도중 전체 정지: 정지 명령 뒤에 재생 명령이 없고 재생 목록이 비어야 한다
    let generation = ENGINE_GENERATION.load(Ordering::SeqCst);
    for round in 0..20u64 {
        GLOBAL_STATE.clear_playing_tracks();
        CURSOR_TABLE.clear_all();
        drain();
        for i in 0..300u64 {
            let inst = 10_000 + i;
            GLOBAL_STATE.add_playing_track(inst, "sfx".into());
            CURSOR_TABLE.publish(i as usize, inst, 0.01);
        }
        restart_resume::take_snapshot(generation);
        assert_eq!(restart_resume::pending_entries().len(), 300);
        let resume = std::thread::spawn(restart_resume::resume_pending);
        std::thread::sleep(Duration::from_micros(200 + round * 100));
        api_stop_all().unwrap();
        resume.join().unwrap();
        let cmds = drain();
        let last_stop = cmds
            .iter()
            .rposition(|c| matches!(c, AudioCommand::StopAll))
            .expect("전체 정지 명령이 큐에 없다");
        let plays_after = cmds[last_stop + 1..]
            .iter()
            .filter(|c| matches!(c, AudioCommand::PlayTrack { .. }))
            .count();
        assert_eq!(plays_after, 0, "{round}회차: 전체 정지 뒤에 재생 {plays_after}개가 남았다");
        assert!(
            GLOBAL_STATE.playing_track_ids.read().unwrap().is_empty(),
            "{round}회차: 전체 정지 뒤에 재생 목록이 남았다"
        );
        assert!(restart_resume::pending_entries().is_empty());
    }
}
