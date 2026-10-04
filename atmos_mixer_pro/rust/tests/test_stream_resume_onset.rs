//! 스트리밍 재생은 첫 묶음이 늦게 와도 0에서 페이드 인한다(딸깍 없음, DSP 3법칙).
//! 재시작 복원은 DiskStreamer가 시작 위치까지 디코딩해 버린 뒤에야 첫 묶음을 보내므로 첫 소리가 늦다.
//! 그동안 페이드가 올라가 버리면 파형 중간에서 큰 크기로 시작한다.
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::SoundInstance;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;

const FS: u32 = 48_000;
const CH: usize = 2;
const BLOCK: usize = 480; // 10ms

#[test]
fn 첫_묶음이_늦게_온_스트리밍_재생은_0에서_페이드_인한다() {
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    mixer.startup_ramp.current_gain = 1.0;
    let (mut tx, rx) = rtrb::RingBuffer::<Vec<f32>>::new(8);
    let mut inst = SoundInstance::new(
        31, 1, 1, "s".into(), None, None, FS, 1, true, 1.0, 1, false, None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.stream_receiver = Some(rx);
    mixer.instances[0] = Some(inst);

    // 시작 위치까지 건너뛰는 0.5초(페이드 0.3초보다 길다) 동안 첫 묶음이 없다
    let mut buf = vec![0.0f32; CH * BLOCK];
    for _ in 0..50 {
        buf.fill(0.0);
        mixer.process(&mut buf, CH);
        assert!(buf.iter().all(|v| v.abs() < 1e-6), "첫 묶음 전에 소리가 났다");
    }

    // 첫 묶음(직류 0.5)이 도착한 뒤 50ms를 모은다. 출력단(리미터 등)에 지연이 있어 첫 소리는 블록 중간에 나온다.
    tx.push(vec![0.5; FS as usize]).unwrap();
    let mut out = Vec::new();
    for _ in 0..5 {
        buf.fill(0.0);
        mixer.process(&mut buf, CH);
        out.extend((0..BLOCK).map(|f| (0..CH).fold(0.0f32, |m, c| m.max(buf[f * CH + c].abs()))));
    }
    let start = out.iter().position(|v| *v > 1e-6).expect("첫 묶음이 왔는데 소리가 나지 않는다");
    let onset = out[start..start + 48].iter().fold(0.0f32, |m, v| m.max(*v));
    // 페이드 인(0.3초)이면 첫 1ms는 0.5 × 48/14400 ≈ 0.002 안팎이다
    assert!(onset < 0.02, "첫 소리 1ms 최대 {onset} — 페이드 인 없이 큰 크기로 시작했다(딸깍)");
}
