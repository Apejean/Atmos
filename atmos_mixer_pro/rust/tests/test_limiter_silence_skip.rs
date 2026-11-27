//! 무음·꺼진 채널의 리미터 건너뛰기(설계 `docs/superpowers/specs/2026-11-27-limiter-silence-skip-design.md`).
//! 건너뛰어도 소리는 한 샘플도 바뀌지 않아야 한다: 늘 `process()`를 부르는 기준과 출력이 같고, 채널을 끄고 켤 때
//! 딸깍이 없다(1.3.16 `test_channel_gate_fade.rs`와 같은 판정). 리미터가 게인을 줄이는 큰 소리에서도 본다.
//! 구현 전에는 `PeakLimiter::is_idle`·`skip_silent`과 `AudioMixer::limiter_skip`이 없어 컴파일되지 않는다(RED).
use rust_lib_atmos_mixer_pro::audio::limiter::PeakLimiter;
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const FS: u32 = 48_000;
const BLOCK: usize = 480;

/// 블록 단위 입력. 0이면 그 블록은 모두 0이다.
fn blocks(pattern: &[(usize, f32)]) -> Vec<Vec<f32>> {
    let mut t = 0usize;
    let mut out = Vec::new();
    for &(count, amp) in pattern {
        for _ in 0..count {
            let block: Vec<f32> = (0..BLOCK)
                .map(|i| amp * (2.0 * std::f32::consts::PI * 997.0 * (t + i) as f32 / FS as f32).sin())
                .collect();
            t += BLOCK;
            out.push(block);
        }
    }
    out
}

/// 기준(늘 process)과 건너뛰는 리미터의 출력을 블록마다 비교한다. 반환: 건너뛴 블록 수.
fn compare(pattern: &[(usize, f32)], what: &str) -> usize {
    let mut reference = PeakLimiter::new(FS as f32, 1.0, 500.0, 0.99);
    let mut skipping = PeakLimiter::new(FS as f32, 1.0, 500.0, 0.99);
    let mut skipped = 0;
    for (b, block) in blocks(pattern).iter().enumerate() {
        let want: Vec<f32> = block.iter().map(|&x| reference.process(x)).collect();
        let silent = block.iter().all(|&x| x == 0.0);
        let got: Vec<f32> = if silent && skipping.is_idle() {
            skipped += 1;
            skipping.skip_silent(BLOCK);
            vec![0.0; BLOCK]
        } else {
            block.iter().map(|&x| skipping.process(x)).collect()
        };
        // 건너뛰기는 상태를 비트까지 맞추므로 출력·텔레메트리가 정확히 같아야 한다(0과 −0은 같은 값으로 본다).
        for (i, (w, g)) in want.iter().zip(&got).enumerate() {
            assert!(w == g, "{what}: 블록 {b} 표본 {i} — 기준 {w}, 건너뛴 쪽 {g}");
        }
        assert_eq!(
            reference.current_gain_reduction_db(),
            skipping.current_gain_reduction_db(),
            "{what}: 블록 {b} 게인 리덕션 텔레메트리가 다르다"
        );
        assert_eq!(reference.short_term_lufs, skipping.short_term_lufs, "{what}: 블록 {b} LUFS가 다르다");
    }
    skipped
}

#[test]
fn 건너뛴_리미터는_늘_처리하는_리미터와_출력이_같다() {
    // 무음 사이의 짧은 소리
    assert!(compare(&[(5, 0.0), (3, 0.3), (40, 0.0), (3, 0.3), (40, 0.0)], "짧은 소리") > 0);
    // 게인 리덕션 중(문턱 위 큰 소리) 바로 무음, 그 뒤 큰 소리
    assert!(compare(&[(20, 1.5), (60, 0.0), (20, 1.5), (5, 0.0), (20, 1.2)], "큰 소리 뒤 무음") > 0);
    // 지연 줄(5ms = 240표본)보다 짧은 무음: 한 블록(480)이라 비우는 데 걸려 건너뛰지 않는다
    compare(&[(10, 0.8), (1, 0.0), (10, 0.8)], "짧은 무음");
    // 긴 무음(3초 LUFS 창이 다 0이 될 만큼) 뒤 큰 소리
    assert!(compare(&[(10, 0.9), (400, 0.0), (10, 1.5)], "긴 무음") > 0);
}

struct Rig {
    mixer: AudioMixer,
    buf: Vec<f32>,
}

const OUT: usize = 8;
const GATED: usize = 2;

fn rig(amp: f32, skip: bool) -> Rig {
    for slot in GLOBAL_STATE.enabled_channels.iter() {
        slot.store(true, Ordering::SeqCst);
    }
    let (gc_tx, _gc_rx) = crossbeam_channel::bounded(16);
    let mut mixer = AudioMixer::new(FS, OUT, BLOCK, gc_tx, None);
    mixer.startup_ramp.current_gain = 1.0;
    mixer.limiter_skip = skip;
    let frames = 4800;
    let mut samples = Vec::with_capacity(frames * 4);
    for i in 0..frames {
        let v = amp * (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / FS as f32).cos();
        for _ in 0..4 {
            samples.push(v);
        }
    }
    let data = Arc::new(SoundData { samples, channels: 4, sample_rate: FS });
    mixer.instances[0] = Some(SoundInstance::new(
        1, 7, 1, "t".into(), Some(data), None, FS, 4, true, 1.0, 0, true, None,
        GLOBAL_STATE.enabled_channels.len(),
    ));
    Rig { mixer, buf: vec![0.0; BLOCK * OUT] }
}

impl Rig {
    fn block(&mut self) -> Vec<f32> {
        self.buf.fill(0.0);
        self.mixer.process(&mut self.buf, OUT);
        self.buf.clone()
    }
}

/// 처음부터 소리가 없는 채널(CH5~8) 수. 블록마다 이 채널들의 리미터는 늘 건너뛴다.
const ALWAYS_SILENT: u64 = (OUT - 4) as u64;
/// CH3를 닫아 두는 블록 수(30초). 닫힌 채널도 채널 DSP는 돈다(#137). DC 블로커(r = 0.999934, 시정수 약 0.3초)의
/// 꼬리가 비정규수 처리(FTZ) 아래에서 정확히 0이 되기까지 약 24초가 걸리고, 그 뒤부터 CH3 리미터를 건너뛴다.
const CLOSED: usize = 3_000;

/// 같은 흐름(재생 → CH3 끔 → 오래 무음 → 다시 켬)을 건너뛰기 켜고 끈 두 믹서로 돌려 출력이 정확히 같은지 보고,
/// 다시 켤 때 샘플 간 최대 변화가 정상의 2배 안인지 본다. 건너뛰기 켠 믹서가 실제로 CH5~8을 늘, CH3를 닫힌 동안
/// 건너뛰었는지도 센다. 전역 게이트를 쓰므로 두 믹서를 같은 블록에서 번갈아 돈다.
fn reopen_case(amp: f32) {
    // 실제 콜백(engine.rs)처럼 비정규수를 0으로 다룬다.
    let _denormals = rust_lib_atmos_mixer_pro::audio::denormal::DenormalGuard::new();
    let mut on = rig(amp, true);
    let mut off = rig(amp, false);
    let mut prev = 0.0f32;
    let mut steady_jump = 0.0f32;
    let mut reopen_jump = 0.0f32;
    let mut gated_skip_blocks = 0usize;
    let open_at = 200 + CLOSED;
    let total = open_at + 40;
    for b in 0..total {
        if b == 200 {
            GLOBAL_STATE.enabled_channels[GATED].store(false, Ordering::SeqCst);
        }
        if b == open_at {
            GLOBAL_STATE.enabled_channels[GATED].store(true, Ordering::SeqCst);
        }
        let skipped_before = on.mixer.limiter_skipped;
        let a = on.block();
        let skipped = on.mixer.limiter_skipped - skipped_before;
        let c = off.block();
        for (i, (x, y)) in a.iter().zip(&c).enumerate() {
            assert!(x == y, "진폭 {amp}: 블록 {b} 표본 {i} — 건너뛰기 켬 {x}, 끔 {y}");
        }
        // 건너뛸 때도 리미터 상태를 넘겨야 한다: 출력은 0이라 같아도, LUFS·게인 리덕션이 멈춰 있으면 여기서 드러난다.
        for (ch, (l_on, l_off)) in on.mixer.limiters.iter().zip(&off.mixer.limiters).take(OUT).enumerate() {
            assert_eq!(l_on.short_term_lufs, l_off.short_term_lufs, "진폭 {amp}: 블록 {b} CH{} LUFS", ch + 1);
            assert_eq!(
                l_on.current_gain_reduction_db(),
                l_off.current_gain_reduction_db(),
                "진폭 {amp}: 블록 {b} CH{} 게인 리덕션",
                ch + 1
            );
        }
        if b < 200 || b >= open_at + 1 {
            assert_eq!(skipped, ALWAYS_SILENT, "진폭 {amp}: 블록 {b} — 소리 없는 CH5~8만 건너뛰어야 한다");
        } else {
            assert!(skipped == ALWAYS_SILENT || skipped == ALWAYS_SILENT + 1, "진폭 {amp}: 블록 {b} 건너뛴 수 {skipped}");
            if skipped == ALWAYS_SILENT + 1 {
                gated_skip_blocks += 1;
            }
        }
        for frame in 0..BLOCK {
            let y = a[frame * OUT + GATED];
            let jump = (y - prev).abs();
            prev = y;
            if (150..200).contains(&b) {
                steady_jump = steady_jump.max(jump);
            }
            if (open_at..open_at + 5).contains(&b) {
                reopen_jump = reopen_jump.max(jump);
            }
        }
    }
    assert_eq!(off.mixer.limiter_skipped, 0, "건너뛰기를 끈 믹서가 건너뛰었다");
    assert!(gated_skip_blocks > 0, "진폭 {amp}: CH3를 닫아 둔 30초 동안 CH3 리미터를 한 번도 건너뛰지 않았다");
    assert!(steady_jump > 0.0, "진폭 {amp}: CH3에 소리가 없다");
    assert!(reopen_jump <= steady_jump * 2.0, "진폭 {amp}: 다시 켤 때 튀었다: {reopen_jump}, 정상 {steady_jump}");
    println!("진폭 {amp}: CH3를 닫은 {CLOSED}블록 중 {gated_skip_blocks}블록에서 CH3 리미터를 건너뜀");
}

#[test]
fn 채널을_끄고_오래_둔_뒤_켜도_딸깍_없고_건너뛰기와_상관없이_출력이_같다() {
    reopen_case(0.5); // 리미터가 일하지 않는 크기
    reopen_case(1.4); // 리미터가 게인을 줄이는 크기
}
