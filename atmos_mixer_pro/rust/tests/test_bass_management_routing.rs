//! 베이스 매니지먼트(서브우퍼 크로스오버)가 **믹서 출력에서** 제대로 동작하는지 검증한다.
//!
//! test_crossover_bass_management.rs는 LR24 필터 단독만 본다. 여기서는 실제
//! `AudioMixer::process`에 톤을 흘려서 위성 채널과 LFE 채널에 무엇이 나오는지 잰다.
//! - 크로스오버 아래 저역은 LFE로, 위 대역은 위성에 남는가
//! - 크로스오버 주파수를 바꾸면 분할 지점이 소리에서도 따라 움직이는가
//! - 주파수 변경, 켜기/끄기, LFE 해제 때 딸깍 소리(불연속)가 없는가 (DSP Law 3)
//! - LFE 지정을 해제하면 저역이 위성으로 돌아오는가

use rust_lib_atmos_mixer_pro::audio::bass_route::compute_bass_route;
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const FS: u32 = 48_000;
const CH: usize = 16;
const BLOCK: usize = 512;
/// 톤이 나가는 위성 채널(0/1은 마스터 리버브·바이노럴이 걸리므로 피한다).
const SAT: usize = 2;
/// LFE(서브우퍼)로 지정할 채널.
const SUB: usize = 3;
const AMP: f32 = 0.1;

fn mixer_playing_tone(freq: f32) -> AudioMixer {
    mixer_playing_tone_on(freq, SAT)
}

/// `channel`로 톤을 보낸다. 서브 채널로 보내면 그 채널의 "자기 신호"(.1 LFE 트랙 역할)다.
fn mixer_playing_tone_on(freq: f32, channel: usize) -> AudioMixer {
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
    for ch in 0..CH {
        GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
    }
    mixer.startup_ramp.current_gain = 1.0;

    // 정수 Hz × 6초라 루프 이음매에서도 위상이 이어진다.
    let n = FS as usize * 6;
    let samples: Vec<f32> = (0..n)
        .map(|i| AMP * (std::f32::consts::TAU * freq * i as f32 / FS as f32).sin())
        .collect();
    let data = Arc::new(SoundData { samples, channels: 1, sample_rate: FS });
    let mut inst = SoundInstance::new(
        1,
        1,
        1,
        "tone".to_string(),
        Some(data),
        None,
        FS,
        1,
        true,
        1.0,
        channel,
        false,
        None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.fade_weight = 1.0;
    mixer.instances[0] = Some(inst);
    mixer
}

/// 블록 단위로 렌더해서 (위성, 서브) 채널 샘플을 이어 붙여 돌려준다.
fn render(mixer: &mut AudioMixer, blocks: usize) -> (Vec<f32>, Vec<f32>) {
    let mut buf = vec![0.0f32; CH * BLOCK];
    let (mut sat, mut sub) = (Vec::new(), Vec::new());
    for _ in 0..blocks {
        buf.fill(0.0);
        mixer.process(&mut buf, CH);
        for f in 0..BLOCK {
            sat.push(buf[f * CH + SAT]);
            sub.push(buf[f * CH + SUB]);
        }
    }
    (sat, sub)
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
}

/// 인접 샘플 간 최대 변화량. 딸깍 소리(불연속)는 여기서 튄다.
fn max_step(x: &[f32]) -> f32 {
    x.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f32::max)
}

fn set_crossover(mixer: &mut AudioMixer, freq: f32) {
    // engine.rs의 SetCrossoverFrequency 핸들러와 같은 동작.
    for c in mixer.crossovers.iter_mut() {
        c.set_target_freq(freq, FS as f32);
    }
}

/// 모든 채널이 한 방에 있고 `lfe`가 그 방의 서브우퍼다(None이면 서브 없음).
fn enable_lfe(mixer: &mut AudioMixer, lfe: Option<usize>) {
    let rooms = vec![Some(1u32); CH];
    let subs: Vec<usize> = lfe.into_iter().collect();
    set_rooms(mixer, &rooms, &subs);
}

/// 채널별 방과 서브우퍼를 지정한다(엔진은 공간 설정 payload로 받는다 — simple.rs).
fn set_rooms(mixer: &mut AudioMixer, rooms: &[Option<u32>], subs: &[usize]) {
    let is_sub: Vec<bool> = (0..CH).map(|c| subs.contains(&c)).collect();
    let has_speaker = vec![true; CH];
    let route = compute_bass_route(&is_sub, rooms, &has_speaker);
    let _old = mixer.set_bass_routing(route, is_sub);
}

/// 지정한 채널들을 블록 단위로 렌더해서 채널별로 이어 붙여 돌려준다.
fn render_channels(mixer: &mut AudioMixer, blocks: usize, channels: &[usize]) -> Vec<Vec<f32>> {
    let mut buf = vec![0.0f32; CH * BLOCK];
    let mut out = vec![Vec::new(); channels.len()];
    for _ in 0..blocks {
        buf.fill(0.0);
        mixer.process(&mut buf, CH);
        for f in 0..BLOCK {
            for (i, &c) in channels.iter().enumerate() {
                out[i].push(buf[f * CH + c]);
            }
        }
    }
    out
}

/// 1초(약 94블록) 동안 안정화한 뒤 0.5초 구간을 측정한다.
fn settled_rms(mixer: &mut AudioMixer) -> (f32, f32) {
    render(mixer, 94);
    let (sat, sub) = render(mixer, 47);
    (rms(&sat), rms(&sub))
}

#[test]
fn 크로스오버_아래_저역은_서브로_가고_위_대역은_위성에_남는다() {
    // 40Hz(크로스오버 80Hz의 한 옥타브 아래)
    let mut m = mixer_playing_tone(40.0);
    enable_lfe(&mut m, Some(SUB));
    set_crossover(&mut m, 80.0);
    let (sat, sub) = settled_rms(&mut m);
    assert!(sub > sat * 10.0, "40Hz: 서브 {sub:.5} / 위성 {sat:.5} — 저역이 서브로 가지 않았다");

    // 1kHz
    let mut m = mixer_playing_tone(1000.0);
    let (dry_sat, _) = settled_rms(&mut m);
    enable_lfe(&mut m, Some(SUB));
    set_crossover(&mut m, 80.0);
    let (sat, sub) = settled_rms(&mut m);
    assert!((sat / dry_sat - 1.0).abs() < 0.02, "1kHz 위성 레벨이 변했다: {dry_sat:.5} -> {sat:.5}");
    assert!(sub < sat * 0.01, "1kHz가 서브로 샜다: 서브 {sub:.5}");
}

#[test]
fn 크로스오버_주파수를_바꾸면_소리에서도_분할_지점이_따라_움직인다() {
    // 같은 100Hz 톤에 크로스오버만 바꾼다.
    let mut m = mixer_playing_tone(100.0);
    enable_lfe(&mut m, Some(SUB));

    set_crossover(&mut m, 60.0);
    let (sat, sub) = settled_rms(&mut m);
    assert!(sat > sub * 3.0, "60Hz 크로스오버: 100Hz는 위성 쪽이어야 한다 (위성 {sat:.5}, 서브 {sub:.5})");

    set_crossover(&mut m, 100.0);
    let (sat, sub) = settled_rms(&mut m);
    // LR4는 크로스오버 지점에서 양쪽이 각각 -6dB로 같다.
    assert!((sat / sub - 1.0).abs() < 0.15, "100Hz 크로스오버: 양쪽이 같아야 한다 (위성 {sat:.5}, 서브 {sub:.5})");
    assert!((sat / AMP * 2.0f32.sqrt() - 0.5).abs() < 0.05, "크로스오버 지점 위성 레벨이 -6dB가 아니다: {sat:.5}");

    set_crossover(&mut m, 200.0);
    let (sat, sub) = settled_rms(&mut m);
    assert!(sub > sat * 3.0, "200Hz 크로스오버: 100Hz는 서브 쪽이어야 한다 (위성 {sat:.5}, 서브 {sub:.5})");
}

/// `event`를 적용하기 전/후의 정상 상태 기울기와 전환 구간 기울기를 비교한다.
/// 전환 구간이 양쪽 정상 상태 중 큰 쪽의 2배를 넘으면 딸깍 소리로 본다.
fn assert_no_click(label: &str, m: &mut AudioMixer, event: impl FnOnce(&mut AudioMixer)) {
    render(m, 94);
    let (before_sat, before_sub) = render(m, 20);
    event(m);
    let (tr_sat, tr_sub) = render(m, 10); // 전환 직후 약 100ms
    render(m, 94);
    let (after_sat, after_sub) = render(m, 20);

    for (name, before, tr, after) in [
        ("위성", &before_sat, &tr_sat, &after_sat),
        ("서브", &before_sub, &tr_sub, &after_sub),
    ] {
        let reference = max_step(before).max(max_step(after));
        let transition = max_step(tr);
        assert!(
            transition <= reference * 2.0,
            "{label} — {name} 채널 불연속: 전환 구간 최대 변화 {transition:.6}, 정상 상태 {reference:.6} ({:.1}배)",
            transition / reference.max(1e-12)
        );
    }
}

#[test]
fn 크로스오버_주파수를_바꿔도_딸깍_소리가_나지_않는다() {
    let mut m = mixer_playing_tone(60.0);
    enable_lfe(&mut m, Some(SUB));
    set_crossover(&mut m, 80.0);
    assert_no_click("80Hz→120Hz", &mut m, |m| set_crossover(m, 120.0));
    assert_no_click("120Hz→40Hz", &mut m, |m| set_crossover(m, 40.0));
}

#[test]
fn 베이스_매니지먼트를_켜고_꺼도_딸깍_소리가_나지_않는다() {
    let mut m = mixer_playing_tone(40.0);
    set_crossover(&mut m, 80.0);
    assert_no_click("켜기", &mut m, |m| enable_lfe(m, Some(SUB)));
    assert_no_click("끄기", &mut m, |m| enable_lfe(m, None));
    assert_no_click("다시 켜기", &mut m, |m| enable_lfe(m, Some(SUB)));
}

#[test]
fn lfe_지정을_해제하면_저역이_위성으로_돌아온다() {
    let mut m = mixer_playing_tone(40.0);
    let (dry_sat, _) = settled_rms(&mut m);

    enable_lfe(&mut m, Some(SUB));
    set_crossover(&mut m, 80.0);
    settled_rms(&mut m);

    // 화면에서 Set as LFE Subwoofer를 끄면 그 방에 서브가 없어진다.
    assert_no_click("LFE 해제", &mut m, |m| enable_lfe(m, None));
    let (sat, sub) = settled_rms(&mut m);
    assert!(
        (sat / dry_sat - 1.0).abs() < 0.02,
        "LFE 해제 후 위성 40Hz가 원래대로 돌아오지 않았다: {dry_sat:.5} -> {sat:.5}"
    );
    assert!(sub < 1e-6, "LFE 해제 후에도 서브 채널에 저역이 남았다: {sub:.6}");
}

#[test]
fn 헤드폰_방_필터는_실제_서브_출력에_영향을_주지_않는다() {
    // 헤드폰 미리듣기는 "보고 있는 방"의 스피커만 들려준다. 그건 바이노럴 경로
    // 전용이다 — 실제 스피커로 나가는 저역 라우팅은 방 필터와 무관하게 그대로여야 한다.
    let mut with_filter = mixer_playing_tone(40.0);
    enable_lfe(&mut with_filter, Some(SUB));
    set_crossover(&mut with_filter, 80.0);
    with_filter.binaural.enabled = false; // 현장(실제 스피커)
    // 위성과 서브는 방 1에 있는데, 헤드폰으로 보고 있는 방은 다른 방(2)이다.
    with_filter.channel_room_ids = vec![Some(1); CH];
    with_filter.set_binaural_room(Some(2));
    let (_, sub_filtered) = {
        render(&mut with_filter, 94);
        render(&mut with_filter, 47)
    };

    let mut no_filter = mixer_playing_tone(40.0);
    enable_lfe(&mut no_filter, Some(SUB));
    set_crossover(&mut no_filter, 80.0);
    no_filter.binaural.enabled = false;
    let (_, sub_plain) = {
        render(&mut no_filter, 94);
        render(&mut no_filter, 47)
    };

    assert!(rms(&sub_plain) > AMP * 0.5, "기준 렌더에 서브 저역이 없다");
    assert!(
        (rms(&sub_filtered) / rms(&sub_plain) - 1.0).abs() < 0.02,
        "방 필터가 실제 서브 출력을 바꿨다: {:.5} vs {:.5}",
        rms(&sub_filtered),
        rms(&sub_plain)
    );
}

#[test]
fn lfe를_다른_채널로_옮겨도_딸깍_소리가_나지_않는다() {
    let mut m = mixer_playing_tone(40.0);
    enable_lfe(&mut m, Some(SUB));
    set_crossover(&mut m, 80.0);
    const NEW_SUB: usize = 5;
    // 블록 단위로 렌더해서 [위성, 옛 서브, 새 서브] 채널을 이어 붙인다.
    let run = |m: &mut AudioMixer, blocks: usize| {
        let mut buf = vec![0.0f32; CH * BLOCK];
        let mut out = [Vec::new(), Vec::new(), Vec::new()];
        for _ in 0..blocks {
            buf.fill(0.0);
            m.process(&mut buf, CH);
            for f in 0..BLOCK {
                for (i, ch) in [SAT, SUB, NEW_SUB].into_iter().enumerate() {
                    out[i].push(buf[f * CH + ch]);
                }
            }
        }
        out
    };
    run(&mut m, 94);
    let before = run(&mut m, 20);
    // SUB(3) → 5번으로 옮긴다.
    enable_lfe(&mut m, Some(NEW_SUB));
    let tr = run(&mut m, 10);
    run(&mut m, 94);
    let after = run(&mut m, 47);

    // 서브 채널들: 옛 채널 페이드아웃, 새 채널 페이드인이 정상 상태 기울기의 2배를 넘지 않는다.
    for (i, name) in [(1, "옛 서브"), (2, "새 서브")] {
        let reference = max_step(&before[i]).max(max_step(&after[i]));
        let transition = max_step(&tr[i]);
        assert!(
            transition <= reference * 2.0,
            "{name} 불연속: 전환 {transition:.6}, 정상 {reference:.6} ({:.1}배)",
            transition / reference.max(1e-12)
        );
    }
    // 위성: 교체하는 동안 저역이 잠깐 위성으로 돌아온다(서브 → 위성 → 새 서브로 에너지가
    // 이어진다). 이때도 원래 톤 자체의 기울기를 넘으면 불연속이다.
    let tone_slope = AMP * std::f32::consts::TAU * 40.0 / FS as f32;
    let sat_tr = max_step(&tr[0]);
    assert!(
        sat_tr <= tone_slope * 1.1,
        "위성 불연속: 전환 {sat_tr:.6}, 원래 톤 기울기 {tone_slope:.6}"
    );

    // 옛 서브 채널은 이제 일반 채널이라 채널 DSP의 DC 블로커(0.5Hz) 꼬리가 1e-6 수준으로
    // 천천히 사라진다(들리지 않는 직류 성분). 저역이 남았는지는 톤 성분으로 본다.
    let old_40 = tone_amplitude(&after[1], 40.0);
    assert!(old_40 < 1e-6, "옛 LFE 채널에 40Hz 저역이 남았다: {old_40:.2e}");
    assert!(rms(&after[1]) < 1e-5, "옛 LFE 채널에 신호가 남았다: {:.2e}", rms(&after[1]));
    assert!(rms(&after[2]) > AMP * 0.5, "새 LFE 채널에 저역이 없다: {:.6}", rms(&after[2]));
}

/// `x` 안에서 `freq` 성분의 진폭(단일 주파수 DFT).
fn tone_amplitude(x: &[f32], freq: f32) -> f32 {
    let (mut re, mut im) = (0.0f64, 0.0f64);
    for (i, v) in x.iter().enumerate() {
        let ph = std::f64::consts::TAU * freq as f64 * i as f64 / FS as f64;
        re += *v as f64 * ph.cos();
        im += *v as f64 * ph.sin();
    }
    (2.0 * (re * re + im * im).sqrt() / x.len().max(1) as f64) as f32
}

// ─────────────────────────────────────────────────────────────────────────────
// 서브 출력 표준 동작
//   서브 출력 = LPF120(LFE 트랙) x (LFE +10dB 토글) + 메인에서 넘어온 저역
//   그리고 그 합 전체가 서브 채널의 딜레이·EQ·게인을 지난다.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn 서브_채널의_자기_신호는_120hz_로우패스를_지난다() {
    // 서브 단자에 중·고역이 나가면 안 된다. 예전에는 서브로 지정한 채널의 자기 신호가
    // 풀레인지로 그대로 나갔다.
    let mut m = mixer_playing_tone_on(1000.0, SUB);
    enable_lfe(&mut m, Some(SUB));
    set_crossover(&mut m, 80.0);
    let (_, sub_1k) = settled_rms(&mut m);
    assert!(sub_1k < AMP * 0.01, "서브로 1kHz가 나갔다: {sub_1k:.5}");

    let mut m = mixer_playing_tone_on(40.0, SUB);
    enable_lfe(&mut m, Some(SUB));
    set_crossover(&mut m, 80.0);
    let (_, sub_40) = settled_rms(&mut m);
    let expected = AMP / 2.0f32.sqrt();
    assert!(
        (sub_40 / expected - 1.0).abs() < 0.05,
        "40Hz가 서브로 온전히 나가지 않았다: {sub_40:.5} (기대 {expected:.5})"
    );
}

#[test]
fn 서브로_지정하지_않은_채널은_일반_채널처럼_풀레인지다() {
    // 서브를 해제하면(또는 한 번도 지정하지 않으면) 그 채널은 보통 스피커다.
    let mut m = mixer_playing_tone_on(1000.0, SUB);
    enable_lfe(&mut m, None);
    let (_, sub) = settled_rms(&mut m);
    let expected = AMP / 2.0f32.sqrt();
    assert!(
        (sub / expected - 1.0).abs() < 0.05,
        "베이스 매니지먼트가 꺼졌는데 1kHz가 줄었다: {sub:.5}"
    );
}

#[test]
fn lfe_부스트는_로우패스_이후에_10db를_올린다() {
    let level = |boost: bool, freq: f32| {
        let mut m = mixer_playing_tone_on(freq, SUB);
        enable_lfe(&mut m, Some(SUB));
        set_crossover(&mut m, 80.0);
        m.lfe_boost_enabled = boost;
        settled_rms(&mut m).1
    };
    let off = level(false, 40.0);
    let on = level(true, 40.0);
    let ratio_db = 20.0 * (on / off).log10();
    assert!(
        (ratio_db - 10.0).abs() < 0.3,
        "LFE 부스트가 +10dB가 아니다: {ratio_db:+.2}dB"
    );
    // 부스트는 로우패스 **이후**라 고역을 되살리지 않는다.
    let high = level(true, 1000.0);
    assert!(high < AMP * 0.03, "부스트가 서브 고역을 키웠다: {high:.5}");
}

#[test]
fn lfe_부스트는_메인에서_넘어온_저역에는_걸리지_않는다() {
    // 표준에서 +10dB는 .1(LFE) 트랙에만 적용된다. 베이스 매니지먼트로 넘어온 메인의
    // 저역까지 키우면 저역 밸런스가 무너진다.
    let level = |boost: bool| {
        let mut m = mixer_playing_tone(40.0); // 톤은 위성(SAT)에
        enable_lfe(&mut m, Some(SUB));
        set_crossover(&mut m, 80.0);
        m.lfe_boost_enabled = boost;
        settled_rms(&mut m).1
    };
    let off = level(false);
    let on = level(true);
    assert!(off > AMP * 0.5, "메인 저역이 서브로 넘어오지 않았다: {off:.5}");
    assert!(
        (on / off - 1.0).abs() < 0.02,
        "LFE 부스트가 메인에서 넘어온 저역까지 키웠다: {off:.5} -> {on:.5}"
    );
}

#[test]
fn 합산_저역도_서브_채널의_게인을_지난다() {
    // 서브의 레벨 트림·딜레이·EQ는 서브 출력 전체에 걸려야 한다. 예전에는 메인에서
    // 넘어온 저역이 서브 채널 DSP를 건너뛰어, 서브 게인을 내려도 그 저역은 그대로였다.
    let level = |sub_gain_db: f32| {
        let mut m = mixer_playing_tone(40.0);
        enable_lfe(&mut m, Some(SUB));
        set_crossover(&mut m, 80.0);
        m.channel_dsp[SUB].set_gain_db(sub_gain_db);
        settled_rms(&mut m).1
    };
    let unity = level(0.0);
    let trimmed = level(-6.0);
    let ratio_db = 20.0 * (trimmed / unity).log10();
    assert!(
        (ratio_db + 6.0).abs() < 0.3,
        "서브 게인 -6dB가 합산 저역에 적용되지 않았다: {ratio_db:+.2}dB"
    );
}

#[test]
fn 메인_채널의_튜닝은_서브로_넘어간_저역에_걸리지_않는다() {
    // 표준 베이스 매니지먼트: 저역을 먼저 가르고, 스피커별 보정(게인·EQ·극성·딜레이)은 그
    // 스피커가 실제로 내는 소리에만 건다. 예전에는 메인 채널 DSP를 다 거친 뒤에 저역을 갈라서
    // 메인의 로우컷·쉘프·게인·극성이 서브로 넘어간 저역에 한 번 더 걸렸다(실기: 서브에서
    // 메인 사운드의 저음이 9~13dB 작게 나옴).
    use rust_lib_atmos_mixer_pro::common::config::{EqBand, EqType};
    let sub_level = |tuned: bool| {
        let mut m = mixer_playing_tone(40.0);
        enable_lfe(&mut m, Some(SUB));
        set_crossover(&mut m, 80.0);
        if tuned {
            let d = &mut m.channel_dsp[SAT];
            d.set_gain_db(-12.0);
            d.update_eq_targets(
                &[
                    EqBand { enabled: true, freq: 60.0, gain: 0.0, q_factor: 0.707, filter_type: EqType::LowCut, slope_db_per_oct: 24 },
                    EqBand { enabled: true, freq: 300.0, gain: -6.0, q_factor: 0.707, filter_type: EqType::LowShelf, slope_db_per_oct: 12 },
                ],
                FS as f32,
            );
            d.phase_invert = true;
            d.update_delay_target(10.0);
        }
        settled_rms(&mut m).1
    };
    let plain = sub_level(false);
    let tuned = sub_level(true);
    assert!(plain > AMP * 0.5, "기준 서브 저역이 없다: {plain:.5}");
    let diff_db = 20.0 * (tuned / plain).log10();
    assert!(diff_db.abs() < 0.3, "메인 튜닝이 서브로 넘어간 저역을 {diff_db:+.1}dB 바꿨다");
}

#[test]
fn 극성이_반대인_메인_둘의_같은_저역이_서브에서_상쇄되지_않는다() {
    // 뒤쪽 스피커는 극성이 뒤집혀 있을 수 있다. 예전에는 그 극성이 서브로 넘어간 저역에도
    // 걸려서, 앞·뒤 스피커가 같은 저음(모노 베이스)을 내면 서브에서 서로 지워졌다.
    const SAT2: usize = 6;
    let sub_level = |invert_second: bool| {
        let mut m = mixer_playing_tone(40.0);
        // 같은 톤(같은 위상)을 SAT2에도 튼다.
        let n = FS as usize * 6;
        let samples: Vec<f32> = (0..n)
            .map(|i| AMP * (std::f32::consts::TAU * 40.0 * i as f32 / FS as f32).sin())
            .collect();
        let data = Arc::new(SoundData { samples, channels: 1, sample_rate: FS });
        let mut inst = SoundInstance::new(
            2, 1, 2, "tone2".to_string(), Some(data), None, FS, 1, true, 1.0, SAT2, false, None,
            GLOBAL_STATE.enabled_channels.len(),
        );
        inst.fade_weight = 1.0;
        m.instances[1] = Some(inst);
        enable_lfe(&mut m, Some(SUB));
        set_crossover(&mut m, 80.0);
        m.channel_dsp[SAT2].phase_invert = invert_second;
        settled_rms(&mut m).1
    };
    let same = sub_level(false);
    let inverted = sub_level(true);
    assert!(same > AMP, "두 메인의 저역이 서브에서 합쳐지지 않았다: {same:.5}");
    let diff_db = 20.0 * (inverted / same).log10();
    assert!(
        diff_db.abs() < 0.3,
        "한 메인의 극성을 뒤집자 서브 저역이 {diff_db:+.1}dB 바뀌었다(상쇄)"
    );
}

#[test]
fn 바이노럴을_켜도_메인에서_넘어온_저역이_헤드폰에_들어간다() {
    // 예전 순서는 채널 DSP -> 바이노럴 렌더 -> 서브에 저역 더하기였다. 그래서 헤드폰
    // 미리듣기에서는 메인이 잘라낸 저역이 통째로 사라졌다(소리가 날카롭게 들린다).
    use rust_lib_atmos_mixer_pro::common::config::Point3D;
    let headphone_rms = |bm: bool| {
        let mut m = mixer_playing_tone(40.0);
        let p = |x: f32, y: f32| Point3D { x, y, z: 1.5, ..Default::default() };
        m.channel_positions = vec![None; CH];
        m.channel_positions[SAT] = Some(p(3.0, 5.0));
        m.channel_positions[SUB] = Some(p(5.0, 5.0));
        m.listener_position = Some(Point3D { x: 5.0, y: 3.0, z: 1.2, ..Default::default() });
        m.binaural.enabled = true;
        m.recalculate_binaural_channel_azimuths();
        if bm {
            enable_lfe(&mut m, Some(SUB));
            set_crossover(&mut m, 80.0);
        }
        let mut buf = vec![0.0f32; CH * BLOCK];
        let mut lr = Vec::new();
        for block in 0..141 {
            buf.fill(0.0);
            m.process(&mut buf, CH);
            if block >= 94 {
                for f in 0..BLOCK {
                    lr.push(buf[f * CH]);
                    lr.push(buf[f * CH + 1]);
                }
            }
        }
        rms(&lr)
    };
    let without_bm = headphone_rms(false);
    let with_bm = headphone_rms(true);
    // 절대 레벨은 보지 않는다. 지금 HRTF 데이터셋(보정 안 된 KEMAR 원본)은 40Hz를
    // -30dB 이상 깎아서 절대값이 작다 — 그건 별도 문제다. 여기서는 베이스 매니지먼트를
    // 켰을 때 저역이 **사라지지 않는지**(켜기 전과 같은 경로로 헤드폰에 오는지)만 본다.
    assert!(without_bm > 0.0, "기준 헤드폰 출력이 0이다");
    println!("헤드폰 40Hz: BM 끔 {without_bm:.6}, BM 켬 {with_bm:.6}");
    let ratio_db = 20.0 * (with_bm / without_bm).log10();
    assert!(
        ratio_db > -3.0,
        "베이스 매니지먼트를 켜자 헤드폰의 40Hz가 {ratio_db:+.1}dB로 사라졌다"
    );
}

#[test]
fn lfe_부스트_토글은_딸깍_소리를_내지_않는다() {
    let mut m = mixer_playing_tone_on(40.0, SUB);
    enable_lfe(&mut m, Some(SUB));
    set_crossover(&mut m, 80.0);
    assert_no_click("LFE 부스트 켜기", &mut m, |m| m.lfe_boost_enabled = true);
    assert_no_click("LFE 부스트 끄기", &mut m, |m| m.lfe_boost_enabled = false);
}

#[test]
fn lfe_트랙은_크로스오버_설정과_무관하게_120hz까지_나간다() {
    // 크로스오버는 **메인 스피커** 쪽 설정이다(메인의 몇 Hz 아래를 잘라 서브로 보낼지).
    // 서브 자기 신호(.1 LFE 트랙)는 크로스오버와 무관하게 120Hz 대역까지 그대로 나가야 한다.
    let level = |crossover: f32| {
        let mut m = mixer_playing_tone_on(100.0, SUB);
        enable_lfe(&mut m, Some(SUB));
        set_crossover(&mut m, crossover);
        settled_rms(&mut m).1
    };
    let low_xo = level(60.0);
    let high_xo = level(150.0);
    let diff_db = 20.0 * (low_xo / high_xo).log10();
    assert!(
        diff_db.abs() < 0.1,
        "크로스오버를 60Hz/150Hz로 바꾸자 LFE 트랙 100Hz가 {diff_db:+.2}dB 달라졌다"
    );
    // LR4 120Hz 로우패스에서 100Hz는 1/(1+(100/120)^4) = -3.4dB.
    let expected = AMP / 2.0f32.sqrt() / (1.0 + (100.0f32 / 120.0).powi(4));
    let err_db = 20.0 * (high_xo / expected).log10();
    assert!(
        err_db.abs() < 0.5,
        "LFE 트랙 100Hz가 120Hz 로우패스 기대값과 {err_db:+.2}dB 다르다"
    );
}


// ─────────────────────────────────────────────────────────────────────────────
// 방별 베이스 매니지먼트
//   각 메인 스피커는 자기 방의 서브로만 저역을 보낸다. 서브가 없는 방은 풀레인지.
// ─────────────────────────────────────────────────────────────────────────────

/// 방 B에 있는 메인 스피커 채널.
const OTHER: usize = 4;
/// 방 B의 서브우퍼 채널.
const SUB_B: usize = 5;
const ROOM_A: Option<u32> = Some(11);
const ROOM_B: Option<u32> = Some(22);

/// SAT·SUB는 방 A, OTHER·SUB_B는 방 B. 나머지 채널은 방 A.
fn two_rooms() -> Vec<Option<u32>> {
    let mut rooms = vec![ROOM_A; CH];
    rooms[OTHER] = ROOM_B;
    rooms[SUB_B] = ROOM_B;
    rooms
}

#[test]
fn 서브가_없는_방의_스피커는_풀레인지로_남고_다른_방_서브로_가지_않는다() {
    // 예전에는 서브가 전체에 하나라, 다른 방 스피커의 저역까지 그 서브로 모였다.
    let mut m = mixer_playing_tone_on(40.0, OTHER); // 방 B 스피커에 40Hz
    set_rooms(&mut m, &two_rooms(), &[SUB]); // 서브는 방 A에만
    set_crossover(&mut m, 80.0);
    render_channels(&mut m, 94, &[OTHER, SUB]);
    let out = render_channels(&mut m, 47, &[OTHER, SUB]);
    let (other, sub_a) = (rms(&out[0]), rms(&out[1]));

    let full = AMP / 2.0f32.sqrt();
    assert!(
        (other / full - 1.0).abs() < 0.05,
        "서브가 없는 방 B 스피커의 40Hz가 잘렸다: {other:.5} (기대 {full:.5})"
    );
    assert!(sub_a < 1e-6, "방 B의 저역이 방 A 서브로 갔다: {sub_a:.6}");
}

#[test]
fn 방마다_자기_방_서브로만_저역이_간다() {
    let subs = [SUB, SUB_B];

    // 방 A 스피커의 40Hz -> 방 A 서브만
    let mut m = mixer_playing_tone_on(40.0, SAT);
    set_rooms(&mut m, &two_rooms(), &subs);
    set_crossover(&mut m, 80.0);
    render_channels(&mut m, 94, &[SUB, SUB_B]);
    let out = render_channels(&mut m, 47, &[SUB, SUB_B]);
    assert!(rms(&out[0]) > AMP * 0.5, "방 A 저역이 방 A 서브로 가지 않았다: {:.5}", rms(&out[0]));
    assert!(rms(&out[1]) < 1e-6, "방 A 저역이 방 B 서브로 샜다: {:.6}", rms(&out[1]));

    // 방 B 스피커의 40Hz -> 방 B 서브만
    let mut m = mixer_playing_tone_on(40.0, OTHER);
    set_rooms(&mut m, &two_rooms(), &subs);
    set_crossover(&mut m, 80.0);
    render_channels(&mut m, 94, &[SUB, SUB_B]);
    let out = render_channels(&mut m, 47, &[SUB, SUB_B]);
    assert!(rms(&out[1]) > AMP * 0.5, "방 B 저역이 방 B 서브로 가지 않았다: {:.5}", rms(&out[1]));
    assert!(rms(&out[0]) < 1e-6, "방 B 저역이 방 A 서브로 샜다: {:.6}", rms(&out[0]));
}
