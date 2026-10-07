//! 스트리밍 단발(루프 아님)은 파일이 끝나면 재생을 마친다(HANDOFF 8번 B4).
//!
//! 디코더 스레드(DiskStreamer)는 파일 끝에서 링버퍼의 보내는 쪽(Producer)을 버리고 끝난다. 예전 믹서는 링이 비면
//! "디코더가 늦는 중"으로만 보고 기다려서, 끝난 효과음이 재생 목록에 계속 남았고 재시작 복원이 그 트랙을 매번 다시
//! 틀었다. 보내는 쪽이 끝났고 남은 묶음도 없을 때만 끝으로 본다 — 묶음이 늦게 오는 것과는 구분해야 한다.
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::SoundInstance;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;

const FS: u32 = 48_000;
const CH: usize = 2;
const BLOCK: usize = 480; // 10ms

/// 1kHz 사인(진폭 0.5) 모노 묶음. 출력단 저역 차단에 걸리지 않게 직류 대신 쓴다.
fn sine_chunk(frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|n| 0.5 * (2.0 * std::f32::consts::PI * 1000.0 * n as f32 / FS as f32).sin())
        .collect()
}

/// 믹서 하나와 스트리밍 단발 인스턴스 하나(링 받는 쪽을 연결). 링 보내는 쪽과 정리 채널 받는 쪽을 돌려준다.
fn mixer_with_stream_oneshot(
    instance_id: u64,
) -> (AudioMixer, rtrb::Producer<Vec<f32>>, crossbeam_channel::Receiver<SoundInstance>) {
    let (gc_tx, gc_rx) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    mixer.startup_ramp.current_gain = 1.0;
    let (tx, rx) = rtrb::RingBuffer::<Vec<f32>>::new(8);
    let mut inst = SoundInstance::new(
        instance_id, 1, 1, "shot".into(), None, None, FS, 1, false, 1.0, 1, false, None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.stream_receiver = Some(rx);
    mixer.instances[0] = Some(inst);
    (mixer, tx, gc_rx)
}

/// 한 블록을 돌리고 그 블록의 최대 절댓값.
fn run_block(mixer: &mut AudioMixer, buf: &mut [f32]) -> f32 {
    buf.fill(0.0);
    mixer.process(buf, CH);
    buf.iter().fold(0.0f32, |m, v| m.max(v.abs()))
}

#[test]
fn 보내는_쪽이_끝나고_남은_묶음을_다_쓰면_스트리밍_단발은_멈춰_정리된다() {
    let (mut mixer, mut tx, gc_rx) = mixer_with_stream_oneshot(41);
    tx.push(sine_chunk(BLOCK * 3)).unwrap(); // 30ms
    drop(tx); // 디코더 스레드가 파일 끝에서 끝났다
    let mut buf = vec![0.0f32; CH * BLOCK];
    // 페이드 아웃(0.3초)보다 넉넉히 2초
    for _ in 0..200 {
        run_block(&mut mixer, &mut buf);
        if let Ok(done) = gc_rx.try_recv() {
            assert_eq!(done.instance_id, 41);
            assert!(mixer.instances[0].is_none(), "정리로 보낸 인스턴스 자리가 비지 않았다");
            return;
        }
    }
    panic!("파일이 끝난 스트리밍 단발이 2초가 지나도 재생 중으로 남았다(정리 채널에 오지 않음)");
}

#[test]
fn 보내는_쪽이_살아_있으면_묶음이_늦어도_스트리밍은_멈추지_않는다() {
    let (mut mixer, mut tx, gc_rx) = mixer_with_stream_oneshot(42);
    tx.push(sine_chunk(BLOCK * 3)).unwrap();
    let mut buf = vec![0.0f32; CH * BLOCK];
    // 첫 묶음을 다 쓴 뒤 2초 동안 다음 묶음이 없다(디코더가 늦음)
    for _ in 0..200 {
        run_block(&mut mixer, &mut buf);
    }
    assert!(gc_rx.try_recv().is_err(), "묶음이 늦을 뿐인데 스트리밍을 끝냈다");
    assert!(
        mixer.instances[0].as_ref().is_some_and(|i| i.is_playing),
        "묶음이 늦을 뿐인데 재생을 멈췄다"
    );
    // 다음 묶음이 오면 다시 소리가 난다
    tx.push(sine_chunk(FS as usize)).unwrap();
    let peak = (0..5).map(|_| run_block(&mut mixer, &mut buf)).fold(0.0f32, f32::max);
    assert!(peak > 0.01, "늦게 온 묶음을 재생하지 않았다(최대 {peak})");
}

#[test]
fn 남은_묶음이_있으면_보내는_쪽이_끝나도_끝까지_재생한다() {
    let (mut mixer, mut tx, gc_rx) = mixer_with_stream_oneshot(43);
    // 묶음 세 개(각 0.5초)를 넣고 바로 보내는 쪽을 버린다 — 링에 남은 묶음을 다 쓰기 전에는 끝나면 안 된다.
    for _ in 0..3 {
        tx.push(sine_chunk(FS as usize / 2)).unwrap();
    }
    drop(tx);
    let mut buf = vec![0.0f32; CH * BLOCK];
    let mut sounding_blocks = 0;
    let mut finished = false;
    for _ in 0..300 {
        if run_block(&mut mixer, &mut buf) > 1e-4 {
            sounding_blocks += 1;
        }
        if gc_rx.try_recv().is_ok() {
            finished = true;
            break;
        }
    }
    // 1.5초 = 150블록. 첫 블록은 페이드 인·출력단 지연으로 작을 수 있어 조금 여유를 둔다.
    assert!(
        sounding_blocks >= 145,
        "남은 묶음을 다 재생하기 전에 끝냈다(소리 난 블록 {sounding_blocks}/150)"
    );
    assert!(finished, "끝까지 재생한 뒤에도 정리되지 않았다");
}
