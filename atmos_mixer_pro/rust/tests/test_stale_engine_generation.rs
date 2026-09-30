//! 엔진을 다시 띄운 뒤 옛 스트림이 살아 있어도 명령은 지금 세대 엔진만 받는다.
//!
//! 사용자 보고(2026-09-29): "3번째 트랙을 정지했는데 버튼만 재생으로 바뀌고 소리는 계속 난다".
//! 실기 로그: 워치독 재시작(장치 샘플레이트 44.1k→48k 변경) 뒤에만 정지가 실패했고, 전체 정지를
//! 두 번 눌러도 인스턴스 하나만 멈췄다. 앱 안에는 믹서가 3개 살아 있었다 — cpal 0.15.3(macOS)은
//! 기본 장치가 아닌 장치의 스트림을 drop해도 해제하지 않아(장치 분리 리스너의 순환 참조), 옛
//! 콜백이 계속 돌며 같은 명령 큐를 나눠 먹었다. 재생 명령은 옛 믹서, 정지 명령은 새 믹서로 가면
//! 소리가 멈추지 않는다.

use rust_lib_atmos_mixer_pro::api::simple::build_play_track_command;
use rust_lib_atmos_mixer_pro::audio::engine::{AudioEngine, ENGINE_GENERATION};
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::SoundData;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use std::sync::atomic::Ordering;
use std::sync::Arc;

fn mixer() -> AudioMixer {
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    AudioMixer::new(48_000, 12, 512, gc_tx, None)
}

fn sounding(m: &AudioMixer) -> usize {
    m.instances.iter().flatten().filter(|i| !i.is_stopping).count()
}

#[test]
fn 옛_세대_스트림은_명령을_가져가지_않아_재생과_정지가_같은_믹서로_간다() {
    let (tx, rx) = crossbeam_channel::unbounded::<AudioCommand>();
    let (mut old, mut current) = (mixer(), mixer());
    ENGINE_GENERATION.store(9, Ordering::SeqCst);
    let data = Arc::new(SoundData { samples: vec![0.1; 4800], channels: 1, sample_rate: 48_000 });

    tx.send(build_play_track_command(
        1, 7, 3, "track".into(), Some(data), None, 48_000, 1, true, 1.0, 1.0, 0, false, None,
    ))
    .expect("재생 명령");
    // 옛 세대(2) 콜백이 먼저 돌아도 명령을 가져가면 안 된다.
    assert!(!AudioEngine::begin_callback(&mut old, &rx, 2), "옛 세대 콜백이 일했다");
    assert!(AudioEngine::begin_callback(&mut current, &rx, 9));
    assert_eq!(sounding(&old), 0, "재생 명령이 옛 스트림의 믹서로 갔다");
    assert_eq!(sounding(&current), 1);

    tx.send(AudioCommand::StopTrack { room_id: 7, track_id: 3 }).expect("정지 명령");
    assert!(!AudioEngine::begin_callback(&mut old, &rx, 2));
    assert!(AudioEngine::begin_callback(&mut current, &rx, 9));
    assert_eq!(sounding(&current), 0, "정지 명령이 재생 중인 믹서에 닿지 않았다");
}
