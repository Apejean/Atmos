//! 채널 위치 -> 바이노럴 방위각 변환의 물리적 방향 회귀 테스트.
//!
//! 이 부호는 두 단계를 거쳐 확정됐다.
//!
//! 1) SOFA 조회에 넣는 azimuth_deg는 양수=왼쪽 우세, 음수=오른쪽 우세다
//!    (수학 교과서 관례와 반대. `binaural_numeric_check` 실측으로 확인).
//! 2) 실제 3D 룸(assets/3d_simulator/studio_engine.html)에서 Dart 채널
//!    x좌표는 Three.js world X에 반전 없이 그대로 들어간다
//!    (`posX = sp.x - room.width/2`). 사용자가 "정면"이라 부르는 카메라
//!    ("Back View" 프리셋 — 마네킹 눈이 보는 방향과 같은 방향을 보는,
//!    마네킹 뒤통수 너머의 시점)에서는 forward×up 벡터 계산상 화면
//!    오른쪽이 world -X다.
//!
//! 두 사실이 서로를 상쇄해서, 최종 공식(`mixer.rs`의
//! `dx.atan2(dy)`, 부호 반전 없음)이 정답이 된다: dx<0(화면 오른쪽, 정면
//! 카메라 기준) -> 음수 azimuth -> 오른쪽 귀 우세.
//!
//! 처음에는 1번만 확인하고 2번(실제 룸 좌표 매핑)을 검증 없이 "월드
//! +X=화면 오른쪽"이라 가정해 `-dx`로 뒤집는 회귀를 만들었다. 실기(스피커
//! 레이아웃 + 헤드폰)에서 "Ch1이 오른쪽에 있는데 왼쪽에서 들린다"는 사용자
//! 보고로 발견해 되돌렸다. 이 테스트는 그 방향을 고정해 나중에 좌표계나
//! SOFA 조회부를 건드릴 때 조용히 다시 뒤집히는 것을 막는다.

use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::common::config::{Point3D, RoomZone};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

fn point(x: f32, y: f32, z: f32) -> Point3D {
    Point3D { x, y, z, ..Default::default() }
}

/// 10m x 8m 방. 채널 0을 리스너 기준 dx_from_center만큼 X축으로 옮겨 둔다.
///
/// 부호 규약(중요, 직관과 반대): 이 프로젝트의 "정면" 카메라(사용자가
/// 마네킹 눈이 보는 방향을 따라 마네킹 뒤통수 너머로 보는 시점, 3D 룸의
/// "Back View" 프리셋)에서는 world +X가 화면 왼쪽, world -X가 화면
/// 오른쪽이다(studio_engine.html의 카메라 벡터 계산, 파일 상단 doc 참고).
/// 즉 **dx_from_center가 음수일 때 "리스너 기준 오른쪽"**이다.
fn build_mixer_with_channel_at(dx_from_center: f32) -> AudioMixer {
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(48000, 2, 512, gc_tx, None);
    mixer.binaural.enabled = true;
    GLOBAL_STATE.hrtf_yaw.store(0.0f32.to_bits(), Ordering::Relaxed);

    let zone = RoomZone {
        room_id: 1,
        boundary_min: point(0.0, 0.0, 0.0),
        boundary_max: point(10.0, 8.0, 3.0),
        boundary_delay_ms: 0.0,
        boundary_eq_bands: vec![],
        transmission_loss_db: 0.0,
        absorption_coeff: 0.0,
        ear_level: 1.2,
    };
    let center_x = 5.0;
    let center_y = 4.0;

    mixer.channel_positions = vec![Some(point(center_x + dx_from_center, center_y, 1.5))];
    mixer.room_zones = vec![zone];
    mixer.recalculate_binaural_channel_azimuths();

    mixer
}

/// 채널에 화이트 노이즈를 흘려 바이노럴 출력의 L/R RMS를 잰다.
fn measure_lr_rms(mixer: &mut AudioMixer) -> (f32, f32) {
    for ch in 0..2 {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    mixer.startup_ramp.current_gain = 1.0;

    // 채널 0 하나에만 소리가 나가도록 mono 소스를 채널 0으로 라우팅.
    let mut samples = vec![0.0f32; 512 * 400];
    let mut rng: u64 = 0x2545F4914F6CDD1D;
    for s in samples.iter_mut() {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let u = (rng >> 40) as f32 / (1u64 << 24) as f32;
        *s = (u * 2.0 - 1.0) * 0.5;
    }
    let sound_data = Arc::new(SoundData { samples, channels: 1, sample_rate: 48000 });

    let mut instance = SoundInstance::new(
        1, 1, 1, "noise".to_string(), Some(sound_data), None, 48000, 1,
        true, 1.0, 0, false, None, GLOBAL_STATE.enabled_channels.len(),
    );
    instance.fade_weight = 1.0;
    mixer.instances[0] = Some(instance);

    let out_channels = 2usize;
    let mut out = vec![0.0f32; out_channels * 512];

    // 크로스페이드가 정착하도록 여러 콜백을 먼저 흘린다.
    for _ in 0..8 {
        mixer.process(&mut out, out_channels);
    }

    let mut rms_l_sq = 0.0f32;
    let mut rms_r_sq = 0.0f32;
    let mut count = 0usize;
    for _ in 0..20 {
        mixer.process(&mut out, out_channels);
        for frame in 0..512 {
            let l = out[frame * out_channels];
            let r = out[frame * out_channels + 1];
            rms_l_sq += l * l;
            rms_r_sq += r * r;
            count += 1;
        }
    }

    ((rms_l_sq / count as f32).sqrt(), (rms_r_sq / count as f32).sqrt())
}

#[test]
fn channel_physically_right_of_listener_is_louder_in_right_ear() {
    // dx=-4.0이 "정면" 카메라 기준 리스너의 오른쪽이다(위 build_mixer_with_channel_at
    // 문서 참고). 실기(스피커 레이아웃 + 헤드폰)에서 사용자가 "Ch1이 오른쪽에
    // 있는데 왼쪽에서 들린다"고 보고해 발견한 방향이 이 테스트다.
    let mut mixer = build_mixer_with_channel_at(-4.0);
    let (l, r) = measure_lr_rms(&mut mixer);
    assert!(
        r > l,
        "리스너 기준 오른쪽(정면 카메라에서 dx<0)에 있는 채널이 오른쪽 귀보다 \
         왼쪽 귀에서 크게 들린다(좌우 반전 회귀). L={:.5} R={:.5}",
        l, r
    );
}

#[test]
fn channel_physically_left_of_listener_is_louder_in_left_ear() {
    // dx=+4.0이 "정면" 카메라 기준 리스너의 왼쪽이다.
    let mut mixer = build_mixer_with_channel_at(4.0);
    let (l, r) = measure_lr_rms(&mut mixer);
    assert!(
        l > r,
        "리스너 기준 왼쪽(정면 카메라에서 dx>0)에 있는 채널이 왼쪽 귀보다 \
         오른쪽 귀에서 크게 들린다(좌우 반전 회귀). L={:.5} R={:.5}",
        l, r
    );
}

#[test]
fn channel_directly_in_front_is_roughly_symmetric() {
    let mut mixer = build_mixer_with_channel_at(0.0); // 리스너 정면(+Y 방향)
    let (l, r) = measure_lr_rms(&mut mixer);
    let ratio = l / r.max(1e-6);
    assert!(
        ratio > 0.5 && ratio < 2.0,
        "정면 채널의 좌우 에너지가 과도하게 비대칭이다: L={:.5} R={:.5} ratio={:.2}",
        l, r, ratio
    );
}

#[test]
fn unbound_channel_falls_back_to_front_without_crashing() {
    // 어떤 RoomZone에도 속하지 않는 위치.
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(48000, 2, 512, gc_tx, None);
    mixer.channel_positions = vec![Some(point(9999.0, 9999.0, 1.5))];
    mixer.room_zones = vec![RoomZone {
        room_id: 1,
        boundary_min: point(0.0, 0.0, 0.0),
        boundary_max: point(10.0, 8.0, 3.0),
        boundary_delay_ms: 0.0,
        boundary_eq_bands: vec![],
        transmission_loss_db: 0.0,
        absorption_coeff: 0.0,
        ear_level: 1.2,
    }];
    mixer.recalculate_binaural_channel_azimuths(); // 크래시하지 않아야 한다
}

#[test]
fn missing_position_falls_back_to_front_without_crashing() {
    let (gc_tx, _) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(48000, 2, 512, gc_tx, None);
    mixer.channel_positions = vec![None];
    mixer.room_zones = vec![];
    mixer.recalculate_binaural_channel_azimuths(); // 크래시하지 않아야 한다
}
