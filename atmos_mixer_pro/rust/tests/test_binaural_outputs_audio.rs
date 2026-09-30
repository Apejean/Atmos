//! 바이노럴을 켜면 소리가 아예 안 난다는 보고가 반복됐다. 오디오 경로를
//! 앱 없이 그대로 돌려서 어디서 신호가 죽는지 고정한다.

use rust_lib_atmos_mixer_pro::audio::binaural::VirtualMixRoomBinaural;

const BLOCK: usize = 1024;
const NUM_CH: usize = 16;

/// ch0에만 1kHz 사인을 실은 인터리브 버퍼를 만든다.
fn make_buffer(phase: &mut f32) -> Vec<f32> {
    let mut buf = vec![0.0f32; BLOCK * NUM_CH];
    for frame in 0..BLOCK {
        buf[frame * NUM_CH] = (*phase).sin() * 0.5;
        *phase += 2.0 * std::f32::consts::PI * 1000.0 / 48_000.0;
    }
    buf
}

fn peak_of_first_two(buf: &[f32]) -> f32 {
    let mut p = 0.0f32;
    for (i, v) in buf.iter().enumerate() {
        if i % NUM_CH < 2 {
            let a = v.abs();
            if a > p {
                p = a;
            }
        }
    }
    p
}

#[test]
fn binaural_produces_audio_on_the_stereo_bus() {
    let mut b = VirtualMixRoomBinaural::new(NUM_CH, BLOCK);
    b.enabled = true;
    b.mark_base_azimuth_dirty();

    let mut phase = 0.0f32;
    let mut best = 0.0f32;
    // 오버랩-애드가 채워지도록 여러 블록 돌린다.
    for _ in 0..8 {
        let mut buf = make_buffer(&mut phase);
        b.process_interleaved(&mut buf, NUM_CH);
        let p = peak_of_first_two(&buf);
        if p > best {
            best = p;
        }
    }

    assert!(
        best > 0.01,
        "바이노럴을 켰는데 스테레오 버스 출력이 사실상 무음이다(peak={best:.6})"
    );
}

#[test]
fn binaural_survives_a_moving_speaker() {
    // 스피커를 드래그하는 상황: 방위각이 계속 바뀌어도 소리가 끊기면 안 된다.
    let mut b = VirtualMixRoomBinaural::new(NUM_CH, BLOCK);
    b.enabled = true;

    let mut phase = 0.0f32;
    let mut worst = f32::MAX;
    for step in 0..40 {
        {
            let az = b.channel_base_azimuth_mut();
            az[0] = step as f32 * 4.0; // 블록마다 4도씩 회전
        }
        b.mark_base_azimuth_dirty();

        let mut buf = make_buffer(&mut phase);
        b.process_interleaved(&mut buf, NUM_CH);
        if step >= 4 {
            let p = peak_of_first_two(&buf);
            if p < worst {
                worst = p;
            }
        }
    }

    assert!(
        worst > 0.01,
        "스피커를 움직이는 동안 바이노럴 출력이 끊긴다(최소 peak={worst:.6})"
    );
}

/// 드래그 중 출력이 매끄럽게 이어지는지(파형이 튀지 않는지) 본다.
///
/// HRIR을 바꿀 때 진행 중이던 크로스페이드를 접어 넣지 않으면, 출력이
/// 방금까지 가던 방향에서 뒤로 튕겼다가 다시 목표로 향한다. HRIR에는 ITD가
/// 들어 있어서 그 왕복이 앞뒤로 흔들리는 딜레이가 되고, 피치가 휜다.
#[test]
fn moving_speaker_does_not_jerk_the_waveform() {
    fn max_jump(b: &mut VirtualMixRoomBinaural, rotate: bool) -> f32 {
        let mut phase = 0.0f32;
        let mut prev = 0.0f32;
        let mut worst = 0.0f32;
        for step in 0..60 {
            if rotate {
                {
                    let az = b.channel_base_azimuth_mut();
                    az[0] = step as f32 * 4.0;
                }
                b.mark_base_azimuth_dirty();
            }
            let mut buf = make_buffer(&mut phase);
            b.process_interleaved(&mut buf, NUM_CH);
            for frame in 0..BLOCK {
                let y = buf[frame * NUM_CH];
                if step >= 8 {
                    let j = (y - prev).abs();
                    if j > worst {
                        worst = j;
                    }
                }
                prev = y;
            }
        }
        worst
    }

    // 정지 상태에서의 자연스러운 샘플 간 변화량.
    let mut still = VirtualMixRoomBinaural::new(NUM_CH, BLOCK);
    still.enabled = true;
    still.mark_base_azimuth_dirty();
    let baseline = max_jump(&mut still, false);
    assert!(baseline > 0.0, "기준 신호가 흐르지 않는다");

    // 드래그처럼 계속 회전시키는 상태.
    let mut moving = VirtualMixRoomBinaural::new(NUM_CH, BLOCK);
    moving.enabled = true;
    let worst = max_jump(&mut moving, true);

    assert!(
        worst < baseline * 2.5,
        "스피커를 움직이는 동안 파형이 튄다(빨리감기 소리): \
         정지={baseline:.6}, 이동={worst:.6}"
    );
}
