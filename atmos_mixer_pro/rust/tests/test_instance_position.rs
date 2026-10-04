//! 재생 위치(초)가 재시작 복원과 위치 조회가 쓰는 표에 정확히 올라가는지 본다.
//! 전역 표를 쓰므로 이 파일에는 테스트를 하나만 둔다.
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::playback_cursor::CURSOR_TABLE;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const FS: u32 = 48_000;
const CH: usize = 4;
const BLOCK: usize = 480; // 10ms

fn instance(id: u64, seconds_of_data: f32, is_loop: bool) -> SoundInstance {
    let n = (FS as f32 * seconds_of_data) as usize;
    let data = Arc::new(SoundData { samples: vec![0.1; n], channels: 1, sample_rate: FS });
    let mut inst = SoundInstance::new(
        id, 1, 1, "t".into(), Some(data), None, FS, 1, is_loop, 1.0, 1, false, None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.fade_weight = 1.0;
    inst
}

#[test]
fn 재생_위치가_표에_오르고_루프는_한_바퀴_안으로_접히고_끝나면_지워진다() {
    // 계산: 시작 위치 지정과 루프 접기
    let mut a = instance(11, 2.0, false);
    a.set_start_position(1.25);
    assert!((a.position_seconds() - 1.25).abs() < 1e-9);
    let mut l = instance(12, 2.0, true);
    l.cursor = FS as f64 * 2.5; // 한 바퀴(2초)를 넘김
    assert!((l.position_seconds() - 0.5).abs() < 1e-9, "루프 위치가 접히지 않았다: {}", l.position_seconds());

    // 믹서가 블록마다 위치를 표에 적는다
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    mixer.startup_ramp.current_gain = 1.0;
    let mut inst = instance(21, 3.0, false);
    inst.set_start_position(1.0);
    mixer.instances[5] = Some(inst);
    let mut buf = vec![0.0f32; CH * BLOCK];
    for _ in 0..50 {
        buf.fill(0.0);
        mixer.process(&mut buf, CH);
    } // 0.5초
    let pos = CURSOR_TABLE
        .snapshot()
        .into_iter()
        .find(|(id, _)| *id == 21)
        .map(|(_, s)| s)
        .expect("표에 위치가 없다");
    assert!((pos - 1.5).abs() < 0.02, "위치 {pos} (기대 1.5초)");

    // 끝난 인스턴스는 표에서 빠진다(남은 데이터 2초 + 페이드 0.3초)
    for _ in 0..300 {
        buf.fill(0.0);
        mixer.process(&mut buf, CH);
    }
    assert!(CURSOR_TABLE.snapshot().iter().all(|(id, _)| *id != 21), "끝난 인스턴스가 표에 남았다");
}
