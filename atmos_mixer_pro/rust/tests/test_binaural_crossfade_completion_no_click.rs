//! 바이노럴 HRIR 전환이 끝나는 블록에서 파형이 깨지면 안 된다.
//!
//! 오버랩-애드 테일은 "이번 블록 끝 시점의 IR 섞임 비율"로 만들어야 한다.
//! 예전에는 크로스페이드가 블록 도중에 끝나면 `is_switching`이 먼저 false가
//! 되어 테일을 **옛 IR** 결과로 채웠다. 다음 블록은 새 IR로 시작하므로 그 블록
//! 전체가 옛 잔향과 새 소리가 어긋난 채 더해졌다. 실측: 방위각 한 번 변경에
//! 전환 후 4번째 블록에서 2차 차분이 기준의 최대 923배(실기 보고: "스피커를
//! 드래그하면 틱틱 끊기는 소리").

use rust_lib_atmos_mixer_pro::audio::binaural::VirtualMixRoomBinaural;

const FS: f32 = 48_000.0;
const BLOCK: usize = 1024;
const NCH: usize = 16;

fn worst_ratio_after_step(step_deg: f32) -> f32 {
    let mut bin = VirtualMixRoomBinaural::new(NCH, BLOCK);
    bin.enabled = true;
    bin.mark_base_azimuth_dirty();
    let mut phase = 0.0f32;
    let (mut p1, mut p2) = ([0.0f32; 2], [0.0f32; 2]);
    let (mut baseline, mut worst) = (0.0f32, 0.0f32);
    const STEP_BLK: usize = 30;
    for blk in 0..50 {
        if blk == STEP_BLK {
            bin.channel_base_azimuth_mut()[0] = step_deg;
            bin.mark_base_azimuth_dirty();
        }
        let mut buf = vec![0.0f32; BLOCK * NCH];
        for f in 0..BLOCK {
            buf[f * NCH] = phase.sin() * 0.3;
            phase += 2.0 * std::f32::consts::PI * 440.0 / FS;
        }
        bin.process_interleaved(&mut buf, NCH);
        for f in 0..BLOCK {
            for c in 0..2 {
                let y = buf[f * NCH + c];
                let d2 = (y - 2.0 * p1[c] + p2[c]).abs();
                if (20..STEP_BLK).contains(&blk) { baseline = baseline.max(d2); }
                if blk >= STEP_BLK { worst = worst.max(d2); }
                p2[c] = p1[c];
                p1[c] = y;
            }
        }
    }
    worst / baseline
}

#[test]
fn azimuth_change_does_not_corrupt_the_completion_block() {
    for deg in [10.0f32, 40.0, 90.0] {
        let r = worst_ratio_after_step(deg);
        println!("0 -> {deg}도 전환: 최대 튐 {r:.2}배");
        assert!(r < 20.0, "0 -> {deg}도 전환 후 파형이 기준의 {r:.1}배로 튄다");
    }
}
