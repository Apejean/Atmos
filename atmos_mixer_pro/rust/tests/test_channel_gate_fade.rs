//! 출력 채널을 열고 닫을 때 딸깍 없이 줄고 커진다(3법칙 3, 사용자 지시 2026-11-21 "채널 페이드부터 고쳐").
//! 예전에는 채널 게이트(`GLOBAL_STATE.enabled_channels`)를 한 샘플 만에 적용했다: 닫으면 그 채널 소리가 바로 끊기고,
//! 열면 바로 시작했다. 또 닫힌 채널은 채널 DSP(지연·EQ)를 건너뛰어, 다시 열면 지연선에 남아 있던 옛 소리가 튀어나올
//! 수 있었다. 재는 법은 `test_master_switch_ramps.rs`와 같다: 1kHz 코사인을 흘리며 인접 샘플 간 최대 변화량을 재고,
//! 전환 뒤 최댓값이 정상 상태의 2배를 넘으면 튄 것이다. 블록(480프레임)이 정확히 10주기라 바뀌는 블록 첫 샘플이 늘
//! 마루다(영교차점에서 바뀌면 즉시 전환해도 튀지 않아 헛통과한다). 전역 상태를 쓰므로 시험은 하나다.
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const FS: u32 = 48_000;
const BLOCK: usize = 480;
const OUT: usize = 8;
/// 닫고 여는 채널(CH3). 트랙은 CH1에서 시작하는 4채널이라 CH1이 열려 있는 한 트랙 자리는 열려 있다.
const GATED: usize = 2;

struct Rig {
    mixer: AudioMixer,
    buf: Vec<f32>,
    prev: Vec<f32>,
}

impl Rig {
    /// 블록 하나를 돌리고 채널마다 (최대 진폭, 인접 샘플 간 최대 변화량)을 돌려준다.
    fn block(&mut self) -> Vec<(f32, f32)> {
        self.buf.fill(0.0);
        self.mixer.process(&mut self.buf, OUT);
        let mut out = vec![(0.0f32, 0.0f32); OUT];
        for frame in 0..BLOCK {
            for (ch, slot) in out.iter_mut().enumerate() {
                let y = self.buf[frame * OUT + ch];
                slot.0 = slot.0.max(y.abs());
                slot.1 = slot.1.max((y - self.prev[ch]).abs());
                self.prev[ch] = y;
            }
        }
        out
    }

    /// 블록 n개 동안 채널 `ch`의 (최대 진폭, 최대 변화량).
    fn run(&mut self, blocks: usize, ch: usize) -> (f32, f32) {
        let mut worst = (0.0f32, 0.0f32);
        for _ in 0..blocks {
            let (p, j) = self.block()[ch];
            worst = (worst.0.max(p), worst.1.max(j));
        }
        worst
    }
}

fn rig(delay_ms: f32) -> Rig {
    for slot in GLOBAL_STATE.enabled_channels.iter() {
        slot.store(true, Ordering::SeqCst);
    }
    let (gc_tx, _gc_rx) = crossbeam_channel::bounded(16);
    let mut mixer = AudioMixer::new(FS, OUT, BLOCK, gc_tx, None);
    mixer.startup_ramp.current_gain = 1.0; // 앱 시작 뮤트 램프는 이 시험의 대상이 아니다
    mixer.channel_dsp[GATED].update_delay_target(delay_ms);
    let frames = 4800; // 100주기, 루프 경계도 이어진다
    let mut samples = Vec::with_capacity(frames * 4);
    for i in 0..frames {
        let v = 0.5 * (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / FS as f32).cos();
        for _ in 0..4 {
            samples.push(v);
        }
    }
    let data = Arc::new(SoundData { samples, channels: 4, sample_rate: FS });
    mixer.instances[0] = Some(SoundInstance::new(
        1, 7, 1, "t".into(), Some(data), None, FS, 4, true, 1.0, 0, true, None,
        GLOBAL_STATE.enabled_channels.len(),
    ));
    let mut rig = Rig { mixer, buf: vec![0.0; BLOCK * OUT], prev: vec![0.0; OUT] };
    for _ in 0..200 {
        rig.block(); // 재생 시작 페이드·볼륨·지연 스무딩이 끝날 때까지
    }
    rig
}

#[test]
fn 채널을_닫고_열_때_딸깍_없이_줄고_커지며_지연이_있어도_옛_소리가_튀지_않는다() {
    for delay_ms in [0.0f32, 5.0] {
        let mut rig = rig(delay_ms);
        let (steady_peak, steady_jump) = rig.run(20, GATED);
        assert!(steady_peak > 0.1, "지연 {delay_ms}ms: 열린 CH3에 소리가 없다");

        // 닫는다: 서서히 줄어 무음이 된다. 같은 트랙의 CH1은 그대로 들린다.
        GLOBAL_STATE.enabled_channels[GATED].store(false, Ordering::SeqCst);
        let (_, closing_jump) = rig.run(5, GATED);
        assert!(
            closing_jump <= steady_jump * 2.0,
            "지연 {delay_ms}ms: CH3을 닫을 때 튀었다: 전환 {closing_jump}, 정상 {steady_jump}"
        );
        let (closed_peak, _) = rig.run(20, GATED);
        assert!(closed_peak < 1e-6, "지연 {delay_ms}ms: 닫은 CH3에 소리가 남았다: {closed_peak}");
        let (ch1_peak, _) = rig.run(5, 0);
        assert!(ch1_peak > 0.1, "지연 {delay_ms}ms: CH3을 닫았는데 CH1까지 멈췄다");

        // 다시 연다: 서서히 커진다(지연선에 남은 옛 소리가 튀어나오지 않는다).
        GLOBAL_STATE.enabled_channels[GATED].store(true, Ordering::SeqCst);
        let (_, opening_jump) = rig.run(5, GATED);
        assert!(
            opening_jump <= steady_jump * 2.0,
            "지연 {delay_ms}ms: CH3을 열 때 튀었다: 전환 {opening_jump}, 정상 {steady_jump}"
        );
        let (reopened_peak, _) = rig.run(20, GATED);
        assert!(reopened_peak > 0.1, "지연 {delay_ms}ms: 다시 연 CH3에 소리가 없다");
    }
}
