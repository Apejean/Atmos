//! 오디오 콜백에서 재생 인스턴스를 해제하지 않는다(HANDOFF 8번 B5, 3법칙 1·2).
//!
//! 인스턴스를 해제하면 메모리를 돌려주고, 스트리밍이면 `DiskStreamer::drop`이 디코더 스레드를 join한다.
//! 콜백 안에서 일어나면 그동안 소리가 멈춘다. 예전에는 두 경우에 콜백에서 해제했다.
//! - 끝난 인스턴스를 정리 채널로 보내지 못했을 때(채널이 가득 참) 그 자리에서 버렸다.
//! - 풀(4096칸)이 가득 찼을 때 새 재생 명령의 인스턴스를 그 자리에서 버렸다. 재생 목록에는 그 id가 남아
//!   한 번도 울리지 않은 트랙을 재시작 복원이 다시 틀 수 있었다(정리 스레드가 지워야 한다).
use rust_lib_atmos_mixer_pro::audio::engine::{AudioEngine, ENGINE_GENERATION};
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::SoundInstance;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;

const FS: u32 = 48_000;
const CH: usize = 2;
const BLOCK: usize = 480;

fn instance(instance_id: u64) -> SoundInstance {
    SoundInstance::new(
        instance_id, 1, 1, "t".into(), None, None, FS, 1, false, 1.0, 1, false, None,
        GLOBAL_STATE.enabled_channels.len(),
    )
}

#[test]
fn 정리_채널이_가득_차면_끝난_인스턴스를_버리지_않고_다음_블록에_다시_보낸다() {
    let (gc_tx, gc_rx) = crossbeam_channel::bounded(1);
    gc_tx.send(instance(1)).unwrap(); // 정리 채널이 가득 찼다
    let mut mixer = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    let mut done = instance(2);
    done.is_playing = false; // 페이드 아웃이 끝났다
    mixer.instances[0] = Some(done);

    let mut buf = vec![0.0f32; CH * BLOCK];
    mixer.process(&mut buf, CH);
    assert!(
        mixer.instances[0].as_ref().is_some_and(|i| i.instance_id == 2),
        "정리 채널이 가득 찼을 때 끝난 인스턴스를 콜백에서 버렸다"
    );

    assert_eq!(gc_rx.recv().unwrap().instance_id, 1); // 정리 스레드가 하나를 비웠다
    mixer.process(&mut buf, CH);
    assert_eq!(
        gc_rx.try_recv().map(|i| i.instance_id).ok(),
        Some(2),
        "채널에 자리가 났는데 끝난 인스턴스를 보내지 않았다"
    );
    assert!(mixer.instances[0].is_none());
}

#[test]
fn 풀이_가득_차면_새_재생_명령의_인스턴스를_콜백에서_버리지_않고_정리_스레드로_넘긴다() {
    let (gc_tx, gc_rx) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    let capacity = mixer.instances.len();
    for (i, slot) in mixer.instances.iter_mut().enumerate() {
        *slot = Some(instance(10_000 + i as u64));
    }

    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    cmd_tx
        .send(AudioCommand::PlayTrack { instance: Box::new(instance(99)), room_volume: 1.0 })
        .unwrap();
    let generation = ENGINE_GENERATION.load(Ordering::SeqCst);
    assert!(AudioEngine::begin_callback(&mut mixer, &cmd_rx, generation));

    assert_eq!(
        gc_rx.try_recv().map(|i| i.instance_id).ok(),
        Some(99),
        "풀이 가득 찼을 때 새 인스턴스를 정리 스레드로 넘기지 않았다(콜백에서 버림)"
    );
    assert_eq!(mixer.instances.iter().filter(|s| s.is_some()).count(), capacity);
    assert!(mixer.instances.iter().flatten().all(|i| i.instance_id != 99));
}
