//! 헤드폰 미리듣기(바이노럴)의 음색이 평탄해야 한다.
//!
//! 앱이 쓰는 HRTF(MIT KEMAR, mit_kemar_normal_pinna.sofa)는 보정되지 않은 원본 측정이다.
//! 측정용 스피커의 저역 부족과 귓구멍 공진이 그대로 들어 있어서, 정면 음원이 중역 대비
//! 2~4kHz는 +16dB, 100Hz 아래는 -5~-22dB였다(실기 보고: "바이노럴 켜면 소리가 엄청
//! 날카롭다"). 게다가 44.1kHz 데이터를 48kHz 엔진에서 그대로 써서 주파수·도달 시간차가
//! 8.8% 어긋나 있었다.
//!
//! 보정 후에도 **방향감은 그대로**여야 한다 — 좌우 레벨 차(ILD)와 도달 시간차(ITD).

use rust_lib_atmos_mixer_pro::audio::binaural::VirtualMixRoomBinaural;

const FS: f32 = 48_000.0;
const BLOCK: usize = 512;
const NUM_CH: usize = 2;

/// `azimuth_deg` 방향 한 채널에 사인을 넣고 (왼쪽, 오른쪽) 귀의 정상 상태 RMS를 잰다.
fn ear_levels(azimuth_deg: f32, freq: f32) -> (f32, f32) {
    let mut b = VirtualMixRoomBinaural::new_with_rate(NUM_CH, BLOCK, FS);
    b.enabled = true;
    b.set_channel_base_azimuths(&[azimuth_deg, 0.0]);

    let mut n = 0usize;
    let (mut sl, mut sr, mut cnt) = (0.0f32, 0.0f32, 0.0f32);
    for block in 0..90 {
        let mut buf = vec![0.0f32; BLOCK * NUM_CH];
        for f in 0..BLOCK {
            buf[f * NUM_CH] = 0.1 * (std::f32::consts::TAU * freq * n as f32 / FS).sin();
            n += 1;
        }
        b.process_interleaved(&mut buf, NUM_CH);
        if block >= 45 {
            for f in 0..BLOCK {
                sl += buf[f * NUM_CH] * buf[f * NUM_CH];
                sr += buf[f * NUM_CH + 1] * buf[f * NUM_CH + 1];
                cnt += 1.0;
            }
        }
    }
    ((sl / cnt).sqrt(), (sr / cnt).sqrt())
}

/// 정면 음원의 두 귀 평균 레벨(dB, 입력 대비).
fn front_db(freq: f32) -> f32 {
    let (l, r) = ear_levels(0.0, freq);
    let input_rms = 0.1 / 2.0f32.sqrt();
    20.0 * (((l * l + r * r) * 0.5).sqrt() / input_rms).log10()
}

#[test]
fn hrtf는_엔진_샘플레이트로_맞춰진다() {
    let b = VirtualMixRoomBinaural::new_with_rate(NUM_CH, BLOCK, FS);
    assert_eq!(
        b.hrtf_sample_rate(),
        Some(FS),
        "44.1kHz HRTF를 48kHz 엔진에서 그대로 쓰면 주파수·도달 시간차가 8.8% 어긋난다"
    );
}

#[test]
fn 정면_음원의_음색이_평탄하다() {
    // 40Hz~6kHz: 확산음장 등화가 색채를 걷어내야 하는 대역 — ±5dB 안.
    let freqs = [40.0, 60.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0];
    let levels: Vec<f32> = freqs.iter().map(|&f| front_db(f)).collect();
    for (f, db) in freqs.iter().zip(&levels) {
        println!("{f:>6.0}Hz: {db:+.1}dB");
    }
    let max = levels.iter().cloned().fold(f32::MIN, f32::max);
    let min = levels.iter().cloned().fold(f32::MAX, f32::min);
    assert!(
        max - min <= 10.0,
        "정면 음원의 40Hz~6kHz 편차가 {:.1}dB다(목표 10dB 이내 = ±5dB)",
        max - min
    );
}

#[test]
fn 고역_대역이_먹먹하거나_날카롭지_않다() {
    // 정면 음원은 8kHz 부근에 KEMAR 귓바퀴 노치(약 -15dB)가 원래 있다 — 방향 정보라 남긴다.
    // 그래서 한 주파수가 아니라 5~12kHz 대역 에너지로 음색을 본다.
    let power = |fs: &[f32]| {
        let p: f32 = fs.iter().map(|&f| 10.0f32.powf(front_db(f) / 10.0)).sum::<f32>();
        10.0 * (p / fs.len() as f32).log10()
    };
    let mid = power(&[500.0, 1000.0, 2000.0]);
    let high = power(&[5000.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0, 12000.0]);
    let diff = high - mid;
    assert!(
        (-8.0..=4.0).contains(&diff),
        "5~12kHz 대역이 중역 대비 {diff:+.1}dB다(-8dB보다 작으면 먹먹, +4dB보다 크면 날카로움)"
    );
}

#[test]
fn 저역이_돌아온다() {
    let low = front_db(40.0);
    let mid = front_db(500.0);
    assert!(
        (low - mid).abs() <= 3.0,
        "40Hz가 500Hz 대비 {:+.1}dB다(보정 전 -21dB). 측정 스피커의 저역 부족이 남아 있다",
        low - mid
    );
}

#[test]
fn 귓구멍_공진이_빠진다() {
    let presence = front_db(3000.0);
    let mid = front_db(500.0);
    assert!(
        presence - mid <= 6.0,
        "3kHz가 500Hz 대비 {:+.1}dB다(보정 전 +16dB). 날카로운 공진이 남아 있다",
        presence - mid
    );
}

#[test]
fn 좌우_레벨차가_유지된다() {
    // 왼쪽 90도 음원: 고역에서 왼쪽 귀가 확실히 커야 한다(머리 그림자).
    let (l, r) = ear_levels(90.0, 3000.0);
    let ild = 20.0 * (l / r).log10();
    assert!(ild >= 6.0, "왼쪽 음원의 3kHz 좌우 레벨차가 {ild:.1}dB로 너무 작다");
    // 오른쪽 음원은 반대.
    let (l, r) = ear_levels(-90.0, 3000.0);
    let ild = 20.0 * (r / l).log10();
    assert!(ild >= 6.0, "오른쪽 음원의 3kHz 좌우 레벨차가 {ild:.1}dB로 너무 작다");
}

#[test]
fn 도달_시간차가_유지된다() {
    // 왼쪽 90도 음원에 클릭을 넣어 두 귀의 도달 시각 차이를 잰다.
    // 사람 머리 기준 약 0.6~0.8ms. 샘플레이트를 잘못 쓰면 8.8% 짧아진다.
    let mut b = VirtualMixRoomBinaural::new_with_rate(NUM_CH, BLOCK, FS);
    b.enabled = true;
    b.set_channel_base_azimuths(&[90.0, 0.0]);
    // 방위각 전환(크로스페이드)이 끝나도록 무음을 흘린다.
    for _ in 0..40 {
        let mut buf = vec![0.0f32; BLOCK * NUM_CH];
        b.process_interleaved(&mut buf, NUM_CH);
    }
    let mut left = Vec::new();
    let mut right = Vec::new();
    for block in 0..4 {
        let mut buf = vec![0.0f32; BLOCK * NUM_CH];
        if block == 0 {
            buf[0] = 1.0;
        }
        b.process_interleaved(&mut buf, NUM_CH);
        for f in 0..BLOCK {
            left.push(buf[f * NUM_CH]);
            right.push(buf[f * NUM_CH + 1]);
        }
    }
    let onset = |x: &[f32]| {
        let max = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        let i = x.iter().position(|v| v.abs() >= 0.1 * max).unwrap_or(0);
        i as f32
    };
    let itd_ms = (onset(&right) - onset(&left)) / FS * 1000.0;
    assert!(
        (0.6..=0.8).contains(&itd_ms),
        "왼쪽 90도 음원의 도달 시간차가 {itd_ms:.3}ms다(기대 0.6~0.8ms, 왼쪽 귀가 먼저)"
    );
}
