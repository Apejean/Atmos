//! 부팅 뮤트 램프가 채널 수와 무관하게 설계값(3초)을 지키는지 고정한다.
//!
//! 예전에는 `apply()` 하나가 "게인 적용 + 전진"을 함께 했는데, 그 호출이
//! `for ch { for frame { ... } }` 안에 있어서 프레임당 한 번이 아니라
//! **프레임 x 채널**만큼 전진했다. 12채널 장치에서는 3초로 설계한 램프가
//! 0.25초 만에 끝나 부팅 초기 스피커 충격음 차단이 거의 무력해졌고,
//! 같은 프레임인데도 채널마다 다른 게인이 걸렸다.

use rust_lib_atmos_mixer_pro::audio::mixer::StartupMuteRamp;

const SR: f32 = 48_000.0;
const BLOCK: usize = 1024;

#[test]
fn 램프는_채널수와_무관하게_3초에_완료된다() {
    for channels in [2usize, 12, 16] {
        let mut ramp = StartupMuteRamp::new(SR);
        let mut frames_done = 0usize;

        // 2.9초까지: 실제 믹서와 동일하게 채널 루프 안에서 block_gain을 읽고,
        // 전진은 블록당 한 번만 한다.
        while frames_done < (SR * 2.9) as usize {
            for _ch in 0..channels {
                for f in 0..BLOCK {
                    let _ = ramp.block_gain(f);
                }
            }
            ramp.advance_block(BLOCK);
            frames_done += BLOCK;
        }
        assert!(
            ramp.block_gain(0) < 1.0,
            "{channels}채널: 2.9초 시점에 램프가 이미 끝났다(게인 {})",
            ramp.block_gain(0)
        );

        // 3.1초까지 돌리면 완료되어야 한다.
        while frames_done < (SR * 3.1) as usize {
            ramp.advance_block(BLOCK);
            frames_done += BLOCK;
        }
        assert!(
            (ramp.block_gain(0) - 1.0).abs() < 1e-6,
            "{channels}채널: 3.1초인데 램프가 아직 {}",
            ramp.block_gain(0)
        );
    }
}

#[test]
fn block_gain은_상태를_바꾸지_않는다() {
    let mut ramp = StartupMuteRamp::new(SR);

    // 채널 루프를 흉내 내 같은 프레임을 여러 번 읽어도 값이 같아야 한다.
    let g = ramp.block_gain(100);
    for _ in 0..16 {
        assert_eq!(ramp.block_gain(100), g, "block_gain 호출이 상태를 전진시켰다");
    }

    // 전진은 정확히 프레임 수만큼.
    let before = ramp.block_gain(0);
    ramp.advance_block(BLOCK);
    let after = ramp.block_gain(0);
    let expected = before + BLOCK as f32 / (SR * 3.0);
    assert!(
        (after - expected).abs() < 1e-6,
        "블록당 전진량이 프레임 수와 맞지 않는다: {after} vs {expected}"
    );
}
