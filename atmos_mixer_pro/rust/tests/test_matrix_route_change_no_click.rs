//! 매트릭스 라우팅(설계 개정 3) — 재생 중 경로를 바꿀 때 딸깍이 없다(3법칙 3, 설계 6.3·8.2).
//! 재는 법은 `test_master_switch_ramps.rs`와 같다: 1kHz 코사인(블록 480프레임 = 정확히 10주기라 명령이 걸리는 블록 첫
//! 샘플이 마루)을 흘리며 인접 샘플 간 최대 변화량을 재고, 정상 상태의 2배를 넘으면 튄 것이다.

use rust_lib_atmos_mixer_pro::audio::engine::{AudioEngine, ENGINE_GENERATION};
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

const FS: u32 = 48_000;
const BLOCK: usize = 480;
const AMP: f32 = 0.5;
const ROOM: u32 = 1;
const TRACK: u32 = 7;

static SERIAL: Mutex<()> = Mutex::new(());

struct Rig {
    mixer: AudioMixer,
    channels: usize,
    buf: Vec<f32>,
    prev: Vec<f32>,
}

impl Rig {
    fn new(channels: usize) -> Self {
        let (gc_tx, _gc_rx) = crossbeam_channel::bounded(16);
        let mut mixer = AudioMixer::new(FS, channels, BLOCK, gc_tx, None);
        mixer.startup_ramp.current_gain = 1.0;
        let samples: Vec<f32> = (0..4800)
            .map(|i| AMP * (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / FS as f32).cos())
            .collect();
        let data = Arc::new(SoundData { samples, channels: 1, sample_rate: FS });
        mixer.instances[0] = Some(SoundInstance::new(
            1, TRACK, ROOM, "t".into(), Some(data), None, FS, 1, true, 1.0, 0, false, None,
            GLOBAL_STATE.enabled_channels.len(),
        ));
        let mut rig = Rig { mixer, channels, buf: vec![0.0; BLOCK * channels], prev: vec![0.0; channels] };
        for _ in 0..200 {
            rig.block();
        }
        rig
    }

    fn block(&mut self) -> Vec<f32> {
        self.buf.fill(0.0);
        self.mixer.process(&mut self.buf, self.channels);
        let mut jumps = vec![0.0f32; self.channels];
        for frame in 0..BLOCK {
            for (ch, jump) in jumps.iter_mut().enumerate() {
                let y = self.buf[frame * self.channels + ch];
                *jump = jump.max((y - self.prev[ch]).abs());
                self.prev[ch] = y;
            }
        }
        jumps
    }

    fn peak(&self, ch: usize) -> f32 {
        (0..BLOCK).map(|f| self.buf[f * self.channels + ch].abs()).fold(0.0, f32::max)
    }

    /// 여러 명령을 한 콜백에 넣는다(같은 블록 앞에서 모두 처리된다).
    fn commands(&mut self, cmds: Vec<AudioCommand>) {
        let (tx, rx) = crossbeam_channel::bounded(16);
        for c in cmds {
            tx.send(c).unwrap();
        }
        assert!(AudioEngine::begin_callback(&mut self.mixer, &rx, ENGINE_GENERATION.load(Ordering::SeqCst)));
    }

    fn worst_after(&mut self, blocks: usize) -> Vec<f32> {
        let mut worst = vec![0.0f32; self.channels];
        for _ in 0..blocks {
            for (w, j) in worst.iter_mut().zip(self.block()) {
                *w = w.max(j);
            }
        }
        worst
    }
}

fn assert_no_step(what: &str, baseline: f32, worst: f32) {
    assert!(baseline > 0.01, "{what}: 기준 신호가 흐르지 않는다(baseline={baseline:.6})");
    assert!(worst < baseline * 2.0, "{what}: 파형이 계단처럼 튄다(딸깍). baseline={baseline:.6}, worst={worst:.6}");
}

fn set_output(ch: usize) -> AudioCommand {
    AudioCommand::SetTrackOutput { room_id: ROOM, track_id: TRACK, output_channel: ch, output_stereo: false }
}

#[test]
fn two_output_changes_in_one_callback_do_not_cut_the_sounding_channel() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut rig = Rig::new(3);
    let baseline = rig.block()[0];
    let level = rig.peak(0);
    // CH1 → CH2 → CH3을 한 콜백에. 예전에는 두 번째가 페이드 중에 와서 들리던 CH1을 한 샘플 만에 끊었다.
    rig.commands(vec![set_output(1), set_output(2)]);
    let worst = rig.worst_after(10);
    for (ch, w) in worst.iter().enumerate() {
        assert_no_step(&format!("CH{}", ch + 1), baseline, *w);
    }
    assert!(rig.peak(0) < 1e-4 && rig.peak(1) < 1e-4, "CH1·CH2에 소리가 남았다");
    assert!((rig.peak(2) - level).abs() < level * 0.02, "마지막 요청(CH3)으로 끝나지 않았다: {}", rig.peak(2));
}

#[test]
fn a_change_back_during_the_fade_ends_on_the_last_request() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut rig = Rig::new(3);
    let baseline = rig.block()[0];
    let level = rig.peak(0);
    rig.commands(vec![set_output(1)]);
    // 페이드 도중(같은 블록이 다 돌기 전) 다시 CH1로.
    rig.commands(vec![set_output(0)]);
    let worst = rig.worst_after(10);
    for (ch, w) in worst.iter().enumerate() {
        assert_no_step(&format!("CH{}", ch + 1), baseline, *w);
    }
    assert!((rig.peak(0) - level).abs() < level * 0.02, "CH1로 돌아오지 않았다: {}", rig.peak(0));
    assert!(rig.peak(1) < 1e-4, "CH2에 소리가 남았다");
}
