//! 바이노럴 방위각 부호 검증용 수동 청음 프로브.
//!
//! BINAURAL_SPATIAL_RENDERING_SPEC.md §5 항목 5: SOFA 데이터셋의 방위각 부호
//! 관례(양수가 왼쪽인지 오른쪽인지)는 코드로 확인할 수 없고 헤드폰 청음이
//! 필요하다. 이 바이너리는 채널 기준 방위각(channel_base_azimuth)을 직접
//! 설정해 1kHz 톤을 바이노럴 렌더링하고, 연결된 오디오 인터페이스로 그대로
//! 재생한다(앱 전체를 띄우지 않고 이 계산 하나만 격리해서 듣기 위함).
//!
//! 사용법: `cargo run --release --bin binaural_azimuth_probe`
//! 헤드폰을 오디오 인터페이스에 꽂고, 콘솔에 출력되는 각도별로 어느 쪽
//! 귀에서 크게 들리는지 확인한다.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rust_lib_atmos_mixer_pro::audio::binaural::VirtualMixRoomBinaural;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

fn main() {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("기본 출력 장치를 찾을 수 없습니다");
    println!(
        "출력 장치: {}",
        device.name().unwrap_or_else(|_| "알 수 없음".into())
    );

    let config = device
        .default_output_config()
        .expect("기본 출력 설정을 가져올 수 없습니다");
    let sample_rate = config.sample_rate().0;
    let hw_channels = config.channels() as usize;
    println!("샘플레이트: {} Hz, 하드웨어 채널: {}", sample_rate, hw_channels);

    // 헤드 요각은 0으로 고정한다(헤드 트래커 없음) — 순수하게
    // channel_base_azimuth 하나만의 효과를 듣기 위함.
    GLOBAL_STATE.hrtf_yaw.store(0.0f32.to_bits(), Ordering::Relaxed);

    let block_size = 1024usize;
    let binaural = Arc::new(Mutex::new(VirtualMixRoomBinaural::new(1, block_size)));
    {
        let mut b = binaural.lock().unwrap();
        b.enabled = true;
        b.channel_base_azimuth_mut()[0] = 0.0;
        b.mark_base_azimuth_dirty();
    }

    // 테스트 순서: 정면 -> 오른쪽(+90, 우리 공식대로면 오른쪽이어야 함) -> 왼쪽(-90) -> 정면.
    // 각 단계 사이 침묵을 넣어 구간을 귀로 구분하기 쉽게 한다.
    let schedule: Vec<(f32, &str)> = vec![
        (0.0, "정면(0°)"),
        (90.0, "+90° — 우리 공식(dx.atan2(dy), +X=+90°)대로면 오른쪽이어야 함"),
        (-90.0, "-90° — 우리 공식대로면 왼쪽이어야 함"),
        (0.0, "정면(0°)로 복귀"),
    ];

    let noise_state = Arc::new(Mutex::new(0x2545F4914F6CDD1Du64));
    let step_idx = Arc::new(Mutex::new(0usize));
    let samples_in_step = Arc::new(Mutex::new(0u64));
    let seconds_per_step = 4u64;
    let samples_per_step = sample_rate as u64 * seconds_per_step;

    // 각 스텝 시작 시 콘솔에 안내를 찍기 위해 마지막으로 안내한 스텝 인덱스를 기록.
    let announced = Arc::new(Mutex::new(usize::MAX));

    println!(
        "\n{}초씩 {}단계 재생합니다. 각 단계에서 어느 쪽 귀가 더 큰지 들어보세요.\n",
        seconds_per_step,
        schedule.len()
    );

    let binaural_cb = binaural.clone();
    let noise_state_cb = noise_state.clone();
    let step_idx_cb = step_idx.clone();
    let samples_in_step_cb = samples_in_step.clone();
    let announced_cb = announced.clone();
    let schedule_cb = schedule.clone();

    let err_fn = |err| eprintln!("스트림 오류: {}", err);

    let stream = device
        .build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut idx = step_idx_cb.lock().unwrap();
                let mut count = samples_in_step_cb.lock().unwrap();
                let mut rng_state = noise_state_cb.lock().unwrap();
                let mut ann = announced_cb.lock().unwrap();

                if *idx >= schedule_cb.len() {
                    data.fill(0.0);
                    return;
                }

                if *ann != *idx {
                    *ann = *idx;
                    let (az, label) = schedule_cb[*idx];
                    println!(">>> 지금부터: {} (azimuth={:.0}°)", label, az);
                    let mut b = binaural_cb.lock().unwrap();
                    b.channel_base_azimuth_mut()[0] = az;
                    b.mark_base_azimuth_dirty();
                }

                let frames = data.len() / hw_channels;
                let mut mono_block = vec![0.0f32; frames * 2]; // 모노 소스를 채널0에만 채운 2채널(binaural 입력 규약)
                for f in 0..frames {
                    // 1kHz 순음은 좌우 구분이 가장 어려운 주파수다(저주파
                    // 시간차 단서와 고주파 레벨차 단서 둘 다 약함). 화이트
                    // 노이즈는 전 대역을 자극해 ITD/ILD 둘 다 강하게 걸리므로
                    // 좌우 위치 구분이 훨씬 쉽다. xorshift64로 대역 제한 없는
                    // 노이즈를 생성한다(암호학적 품질 불필요, 청음 테스트용).
                    let mut x = *rng_state;
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    *rng_state = x;
                    let u = (x >> 40) as f32 / (1u64 << 24) as f32; // [0, 1)
                    let s = (u * 2.0 - 1.0) * 0.25;
                    mono_block[f * 2] = s;
                }

                {
                    let mut b = binaural_cb.lock().unwrap();
                    b.process_interleaved(&mut mono_block, 2);
                }

                for f in 0..frames {
                    let l = mono_block[f * 2];
                    let r = mono_block[f * 2 + 1];
                    for ch in 0..hw_channels {
                        data[f * hw_channels + ch] = if ch == 0 {
                            l
                        } else if ch == 1 {
                            r
                        } else {
                            0.0
                        };
                    }
                }

                *count += frames as u64;
                if *count >= samples_per_step {
                    *count = 0;
                    *idx += 1;
                }
            },
            err_fn,
            None,
        )
        .expect("스트림 생성 실패");

    stream.play().expect("스트림 재생 실패");

    let total_secs = seconds_per_step * schedule.len() as u64 + 1;
    std::thread::sleep(std::time::Duration::from_secs(total_secs));
    println!("\n완료.");
}
