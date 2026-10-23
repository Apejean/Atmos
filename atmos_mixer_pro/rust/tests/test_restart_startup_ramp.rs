//! 엔진이 스스로 다시 시작할 때만 부팅 뮤트 램프가 짧다(HANDOFF 남은 일 8, 2026-10-21 사용자 결정).
//!
//! 예전에는 자기 재시작(워치독·장치 오류·장치 복귀)의 새 믹서도 3초 램프를 타서, 멈춘 자리부터 이어 튼 소리가
//! 재개 순간 약 −15dB에서 3초에 걸쳐 커졌다. 이어 트는 트랙에는 따로 300ms 페이드 인이 있다. 앱을 켤 때와
//! 사람이 장치를 바꿀 때는 3초 그대로다.

use rust_lib_atmos_mixer_pro::audio::mixer::{AudioMixer, StartupMuteRamp, RESTART_RAMP_S, STARTUP_RAMP_S};
use rust_lib_atmos_mixer_pro::core::restart_resume::set_short_startup_ramp;

const SR: f32 = 48_000.0;
const BLOCK: usize = 512;

#[test]
fn 자기_재시작이_여는_믹서만_램프가_짧다() {
    // 전역 표시를 바꾸므로 이 파일에서 믹서를 만드는 시험은 이것 하나다.
    let mixer = |restarting: bool| {
        set_short_startup_ramp(restarting);
        let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
        let mixer = AudioMixer::new(SR as u32, 2, BLOCK, gc_tx, None);
        set_short_startup_ramp(false);
        mixer
    };
    let seconds = |m: &AudioMixer| 1.0 / (m.startup_ramp.ramp_step * SR);

    let restarted = mixer(true);
    assert!(
        (seconds(&restarted) - RESTART_RAMP_S).abs() < 1e-3,
        "자기 재시작 믹서의 램프가 {}초다",
        seconds(&restarted)
    );
    let started = mixer(false);
    assert!(
        (seconds(&started) - STARTUP_RAMP_S).abs() < 1e-3,
        "앱 시작·장치 변경 믹서의 램프가 {}초다",
        seconds(&started)
    );
}

#[test]
fn 재시작_램프는_0_3초에_끝난다() {
    let mut ramp = StartupMuteRamp::with_seconds(SR, RESTART_RAMP_S);
    let mut frames = 0usize;
    while frames < (SR * 0.29) as usize {
        ramp.advance_block(BLOCK);
        frames += BLOCK;
    }
    assert!(ramp.block_gain(0) < 1.0, "0.29초에 램프가 이미 끝났다");
    while frames < (SR * 0.31) as usize {
        ramp.advance_block(BLOCK);
        frames += BLOCK;
    }
    assert!((ramp.block_gain(0) - 1.0).abs() < 1e-6, "0.31초인데 램프가 아직 {}", ramp.block_gain(0));
}
