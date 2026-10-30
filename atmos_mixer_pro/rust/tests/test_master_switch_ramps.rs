//! 음소거·페일오버 감쇠·헤드룸·트랙 출력 채널을 바꿀 때 파형이 계단처럼 튀지 않는다(3법칙 3, 2026-10-30).
//!
//! 예전에는 모두 한 샘플 만에 바뀌었다: 음소거는 값을 0으로, 페일오버는 0.01배로, 헤드룸은 블록마다 새 배율로
//! 곱했고, 출력 채널을 바꾸면 소리가 옛 채널에서 끊기고 새 채널에서 바로 시작했다. 모두 "딱" 소리가 난다.
//!
//! 재는 법은 `test_phase_invert_no_click.rs`와 같다. 1kHz 사인을 흘리며 인접 샘플 간 최대 변화량을 재고, 전환 뒤의
//! 최댓값이 정상 상태의 2배를 넘으면 튄 것이다. 신호는 코사인(마루에서 시작)이고 블록(480프레임)이 정확히 10주기라
//! 명령이 걸리는 블록 첫 샘플이 늘 마루다. 영교차점에서 바뀌면 즉시 전환해도 튀지 않아 시험이 헛통과한다.
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

/// 페일오버 시험이 전역 상태를 바꾸므로 이 파일의 시험은 한 번에 하나만 돈다.
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
        mixer.startup_ramp.current_gain = 1.0; // 앱 시작 3초 뮤트 램프는 이 시험의 대상이 아니다
        // 100주기 코사인(루프 경계도 이어진다).
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
            rig.block(); // 재생 시작 페이드·볼륨 스무딩이 끝나 크기가 멈출 때까지(약 2초, 0.2초에는 아직 오르는 중이었다)
        }
        rig
    }

    /// 한 블록을 돌리고 채널마다 인접 샘플 간 최대 변화량(앞 블록 마지막 샘플과의 경계 포함)을 돌려준다.
    fn block(&mut self) -> Vec<f32> {
        self.buf.fill(0.0);
        self.mixer.process(&mut self.buf, self.channels);
        let mut jumps = vec![0.0f32; self.channels];
        for frame in 0..BLOCK {
            for ch in 0..self.channels {
                let y = self.buf[frame * self.channels + ch];
                jumps[ch] = jumps[ch].max((y - self.prev[ch]).abs());
                self.prev[ch] = y;
            }
        }
        jumps
    }

    /// 블록 안 채널의 최대 진폭.
    fn peak(&self, ch: usize) -> f32 {
        (0..BLOCK).map(|f| self.buf[f * self.channels + ch].abs()).fold(0.0, f32::max)
    }

    fn command(&mut self, cmd: AudioCommand) {
        let (tx, rx) = crossbeam_channel::bounded(4);
        tx.send(cmd).unwrap();
        assert!(AudioEngine::begin_callback(&mut self.mixer, &rx, ENGINE_GENERATION.load(Ordering::SeqCst)));
    }

    /// 전환 직후 몇 블록 동안 채널마다의 최대 변화량.
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
    assert!(
        worst < baseline * 2.0,
        "{what}: 파형이 계단처럼 튄다(딸깍 소리). baseline={baseline:.6}, worst={worst:.6}"
    );
}

#[test]
fn 마스터_음소거는_딸깍_없이_줄었다가_딸깍_없이_돌아온다() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut rig = Rig::new(2);
    let baseline = rig.block()[0];
    let level = rig.peak(0); // 믹서 기본 게인이 걸린 정상 상태 크기

    rig.command(AudioCommand::SetMasterMute { muted: true });
    assert_no_step("음소거", baseline, rig.worst_after(8)[0]);
    assert!(rig.peak(0) < 1e-4, "음소거가 끝까지 가지 않았다: {}", rig.peak(0));

    rig.command(AudioCommand::SetMasterMute { muted: false });
    assert_no_step("음소거 해제", baseline, rig.worst_after(8)[0]);
    assert!((rig.peak(0) - level).abs() < level * 0.02, "음소거 해제 뒤 원래 크기로 돌아오지 않았다: {}", rig.peak(0));
}

#[test]
fn 페일오버_감쇠는_딸깍_없이_걸리고_풀린다() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            GLOBAL_STATE.is_failover_mode.store(false, Ordering::SeqCst);
        }
    }
    let _reset = Reset;
    let mut rig = Rig::new(2);
    let baseline = rig.block()[0];
    let level = rig.peak(0); // 믹서 기본 게인이 걸린 정상 상태 크기

    GLOBAL_STATE.is_failover_mode.store(true, Ordering::SeqCst);
    assert_no_step("페일오버 감쇠", baseline, rig.worst_after(8)[0]);
    assert!((rig.peak(0) - level * 0.01).abs() < level * 0.002, "-40dB까지 내려가지 않았다: {}", rig.peak(0));

    GLOBAL_STATE.is_failover_mode.store(false, Ordering::SeqCst);
    assert_no_step("페일오버 해제", baseline, rig.worst_after(8)[0]);
    assert!((rig.peak(0) - level).abs() < level * 0.02, "페일오버 해제 뒤 원래 크기로 돌아오지 않았다: {}", rig.peak(0));
}

#[test]
fn 헤드룸을_바꿔도_딸깍_없이_새_크기로_간다() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut rig = Rig::new(2);
    let baseline = rig.block()[0];
    let level = rig.peak(0); // 믹서 기본 게인이 걸린 정상 상태 크기
    let limiter = rig.mixer.peak_limiter_enabled; // 리미터 켜기/끄기는 이 시험의 대상이 아니다

    rig.command(AudioCommand::ApplyGlobalTuning { master_headroom_db: -12.0, peak_limiter_enabled: limiter });
    assert_no_step("헤드룸 -12dB", baseline, rig.worst_after(8)[0]);
    let expected = level * 10f32.powf(-12.0 / 20.0);
    assert!((rig.peak(0) - expected).abs() < level * 0.01, "헤드룸 -12dB 크기가 아니다: {} (기대 {expected})", rig.peak(0));
}

#[test]
fn 출력_채널을_바꾸면_옛_채널은_서서히_줄고_새_채널은_서서히_커진다() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut rig = Rig::new(2);
    let baseline = rig.block()[0];
    let level = rig.peak(0); // 믹서 기본 게인이 걸린 정상 상태 크기
    assert!(rig.peak(1) < 1e-4, "바꾸기 전에 새 채널에서 소리가 났다: {}", rig.peak(1));

    rig.command(AudioCommand::SetTrackOutput {
        room_id: ROOM,
        track_id: TRACK,
        output_channel: 1,
        output_stereo: false,
    });
    let worst = rig.worst_after(8);
    assert_no_step("옛 채널(CH1)", baseline, worst[0]);
    assert_no_step("새 채널(CH2)", baseline, worst[1]);
    assert!(rig.peak(0) < 1e-4, "옛 채널(CH1)에 소리가 남았다: {}", rig.peak(0));
    assert!((rig.peak(1) - level).abs() < level * 0.02, "새 채널(CH2)이 원래 크기가 아니다: {}", rig.peak(1));
}
