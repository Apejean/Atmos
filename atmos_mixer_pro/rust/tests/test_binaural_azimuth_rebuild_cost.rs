//! 스피커를 드래그하면 위치 갱신이 초당 30번씩 온다. 그때마다 **전 채널**의
//! HRIR을 다시 만들면(최근접 3점 탐색 + IR 보간 + FFT) 오디오 스레드가
//! 그 작업에 잠겨 소리가 끊긴다.
//!
//! 실기 보고: "ch1 스피커를 움직이는 순간 끊기면서 이상하게 들린다.
//! 배치하고 나면 정상."
//!
//! 실제로 각도가 바뀌는 건 드래그 중인 채널 하나뿐이므로, 나머지는 건너뛰어야
//! 한다.

use rust_lib_atmos_mixer_pro::audio::binaural::VirtualMixRoomBinaural;

const BLOCK: usize = 1024;
const NUM_CH: usize = 16;

fn run_one_callback(b: &mut VirtualMixRoomBinaural) {
    let mut buf = vec![0.0f32; BLOCK * NUM_CH];
    for (i, v) in buf.iter_mut().enumerate() {
        *v = ((i % 97) as f32 / 97.0) - 0.5;
    }
    b.process_interleaved(&mut buf, NUM_CH);
}

#[test]
fn only_channels_whose_angle_changed_are_rebuilt() {
    let mut b = VirtualMixRoomBinaural::new(NUM_CH, BLOCK);
    b.enabled = true;

    // 초기 배치 적용.
    {
        let az = b.channel_base_azimuth_mut();
        for (i, a) in az.iter_mut().enumerate() {
            *a = i as f32 * 20.0;
        }
    }
    b.mark_base_azimuth_dirty();
    run_one_callback(&mut b);

    // 드래그 한 프레임: ch3만 각도가 바뀐다. 나머지는 그대로.
    {
        let az = b.channel_base_azimuth_mut();
        az[3] += 4.0;
    }
    b.mark_base_azimuth_dirty();

    let before = b.debug_hrir_rebuild_count();
    run_one_callback(&mut b);
    let rebuilt = b.debug_hrir_rebuild_count() - before;

    assert_eq!(
        rebuilt, 1,
        "각도가 바뀐 채널은 1개인데 {rebuilt}개를 다시 만들었다 — \
         드래그 중 오디오 스레드가 과부하된다"
    );

    // 위치가 전혀 안 바뀐 갱신은 아무것도 다시 만들면 안 된다.
    b.mark_base_azimuth_dirty();
    let before = b.debug_hrir_rebuild_count();
    run_one_callback(&mut b);
    assert_eq!(
        b.debug_hrir_rebuild_count() - before,
        0,
        "각도가 그대로인데 HRIR을 다시 만들었다"
    );
}
