//! 오디오 콜백 안의 명령 처리와 블록 처리는 힙을 잡거나 풀지 않는다(3법칙 1·2, 2026-10-30).
//!
//! 이 시험 파일에만 쓰는 전역 할당자가, 시험 스레드에서 무장한 동안의 할당·재할당·해제 횟수를 센다(정리
//! 스레드처럼 다른 스레드가 푸는 것은 세지 않는다). 예전에는 콜백 안에서 이런 일이 있었다(HANDOFF "기존 3법칙
//! 위반"): 재생 명령의 Box 해제, 한 밴드 EQ 변경의 `target_bands` 복제, 채널 튜닝 일괄 적용의 Vec 해제, 장치가
//! 임시 버퍼보다 큰 블록을 줄 때의 `temp_buf.resize`.
use rust_lib_atmos_mixer_pro::audio::engine::{AudioEngine, ENGINE_GENERATION};
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::SoundInstance;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::common::config::EqBand;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::Ordering;

const FS: u32 = 48_000;
const CH: usize = 2;
const BLOCK: usize = 480;

struct CountingAlloc;

thread_local! {
    static ARMED: Cell<bool> = const { Cell::new(false) };
    static HEAP_OPS: Cell<usize> = const { Cell::new(0) };
}

fn count() {
    // const 초기화 TLS만 읽는다(지연 초기화가 할당자 안에서 다시 할당하지 않게).
    let _ = ARMED.try_with(|armed| {
        if armed.get() {
            HEAP_OPS.with(|n| n.set(n.get() + 1));
        }
    });
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        System.alloc(layout)
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        System.alloc_zeroed(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        count();
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

/// `f`를 도는 동안 이 스레드의 힙 할당·재할당·해제 횟수.
fn heap_ops(f: impl FnOnce()) -> usize {
    // 전역 상태는 처음 읽을 때 만들어진다. 그 할당을 세지 않게 무장 전에 한 번 읽는다.
    let _ = GLOBAL_STATE.enabled_channels.len();
    HEAP_OPS.with(|n| n.set(0));
    ARMED.with(|a| a.set(true));
    f();
    ARMED.with(|a| a.set(false));
    HEAP_OPS.with(|n| n.get())
}

fn instance(instance_id: u64) -> SoundInstance {
    SoundInstance::new(
        instance_id, 1, 1, "t".into(), None, None, FS, 1, false, 1.0, 1, false, None,
        GLOBAL_STATE.enabled_channels.len(),
    )
}

/// 명령 하나를 콜백 첫머리(begin_callback)로 처리하고, 그동안의 힙 연산 횟수를 돌려준다.
fn ops_for_command(mixer: &mut AudioMixer, cmd: AudioCommand) -> usize {
    let (cmd_tx, cmd_rx) = crossbeam_channel::bounded(4);
    cmd_tx.send(cmd).unwrap();
    let generation = ENGINE_GENERATION.load(Ordering::SeqCst);
    let mut handled = false;
    let ops = heap_ops(|| handled = AudioEngine::begin_callback(mixer, &cmd_rx, generation));
    assert!(handled);
    ops
}

fn mixer() -> AudioMixer {
    let (gc_tx, _gc_rx) = crossbeam_channel::bounded(16);
    AudioMixer::new(FS, CH, BLOCK, gc_tx, None)
}

#[test]
fn 재생_명령을_풀에_담을_때_힙을_건드리지_않는다() {
    let mut mixer = mixer();
    let cmd = AudioCommand::PlayTrack { instance: instance(7), room_volume: 1.0 };
    assert_eq!(ops_for_command(&mut mixer, cmd), 0, "재생 명령 처리 중 콜백에서 힙을 잡거나 풀었다");
    assert!(mixer.instances.iter().flatten().any(|i| i.instance_id == 7));
}

#[test]
fn 한_밴드_eq_변경은_힙을_건드리지_않는다() {
    let mut mixer = mixer();
    let cmd = AudioCommand::UpdateSingleBandEq {
        channel: 1,
        band: 2,
        freq: 1000.0,
        gain_db: -3.0,
        q_factor: 1.4,
        filter_type_idx: 2,
    };
    assert_eq!(ops_for_command(&mut mixer, cmd), 0, "한 밴드 EQ 변경 중 콜백에서 힙을 잡거나 풀었다");
    let band = &mixer.channel_dsp[1].target_bands[2];
    assert_eq!((band.freq, band.gain, band.q_factor), (1000.0, -3.0, 1.4));
}

#[test]
fn 채널_튜닝_일괄_적용은_힙을_건드리지_않는다() {
    let mut mixer = mixer();
    let bands = vec![EqBand { gain: 2.0, ..EqBand::default() }; 3];
    let tunings = vec![(0, 1.5, bands.clone(), true, -2.0), (1, 0.0, bands, false, 0.0)];
    let cmd = AudioCommand::ApplyAllChannelTunings { tunings };
    assert_eq!(ops_for_command(&mut mixer, cmd), 0, "채널 튜닝 적용 중 콜백에서 Vec을 풀었다");
    assert!(mixer.channel_dsp[0].phase_invert);
    assert_eq!(mixer.channel_dsp[0].target_bands[0].gain, 2.0);
}

#[test]
fn 장치가_임시_버퍼보다_큰_블록을_줘도_늘리지_않고_나눠서_모두_채운다() {
    let mut mixer = mixer();
    // 임시 버퍼는 100프레임분, 장치는 333프레임을 달라고 한다(4조각).
    let mut temp_buf = vec![0.0f32; 100 * CH];
    let mut data = vec![0.5f32; 333 * CH];
    let mut frames = 0;
    let ops = heap_ops(|| frames = AudioEngine::render_block(&mut mixer, &mut temp_buf, &mut data, CH, CH));
    assert_eq!(ops, 0, "큰 블록에서 콜백 안 임시 버퍼를 늘렸다");
    assert_eq!(frames, 333);
    assert_eq!(temp_buf.len(), 100 * CH);
    // 재생 중인 소리가 없으니 믹서 출력은 무음이다. 마지막 프레임까지 모두 덮어써야 한다.
    assert!(data.iter().all(|&s| s == 0.0), "임시 버퍼를 넘는 뒤쪽 프레임을 채우지 않았다");
}
