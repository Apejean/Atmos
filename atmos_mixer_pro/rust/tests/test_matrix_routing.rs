//! 매트릭스 라우팅(설계 `docs/superpowers/specs/2026-11-14-matrix-routing-design.md` 개정 3) — 믹서의 표 경로.
//!
//! 판정은 "같은 소리를 옛 경로(트랙별 출력 매핑)로 낸 믹서"와 견준다. 표 경로가 파일 채널 s를 출력 o로 보내면, 그
//! 채널만 담은 모노 파일을 출력 o로 보낸 옛 경로 인스턴스와 같은 소리여야 한다(채널 DSP·리미터는 출력 채널 단위라
//! 같다). 계산식을 시험에 다시 쓰지 않는다.

use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::{SoundData, SoundInstance};
use rust_lib_atmos_mixer_pro::audio::route::RouteEntry;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

const FS: u32 = 48_000;
const BLOCK: usize = 480;
const ROOM: u32 = 1;
const FRAMES: usize = 4_800; // 0.1초 루프 — 10Hz의 배수 주파수는 끊김 없이 돈다

/// 전역 게이트·줄 표를 바꾸는 시험이 있어 이 파일의 시험은 하나씩 돈다.
static SERIAL: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

/// 채널 c는 (300 + 100c)Hz 사인, 크기 0.2.
fn tones(channels: usize) -> Vec<Vec<f32>> {
    (0..channels)
        .map(|c| {
            (0..FRAMES)
                .map(|i| 0.2 * (2.0 * std::f32::consts::PI * (300.0 + 100.0 * c as f32) * i as f32 / FS as f32).sin())
                .collect()
        })
        .collect()
}

fn interleave(chs: &[Vec<f32>]) -> Arc<SoundData> {
    let n = chs.len();
    let mut samples = Vec::with_capacity(FRAMES * n);
    for i in 0..FRAMES {
        for c in chs {
            samples.push(c[i]);
        }
    }
    Arc::new(SoundData { samples, channels: n as u16, sample_rate: FS })
}

fn mixer(width: usize) -> AudioMixer {
    let (gc_tx, _gc_rx) = crossbeam_channel::bounded(16);
    let mut m = AudioMixer::new(FS, width, BLOCK, gc_tx, None);
    m.startup_ramp.current_gain = 1.0;
    m
}

fn inst(id: u32, data: Arc<SoundData>, out: usize, stereo: bool) -> SoundInstance {
    let ch = data.channels;
    SoundInstance::new(
        id as u64, id, ROOM, format!("t{id}"), Some(data), None, FS, ch, true, 1.0, out, stereo, None,
        GLOBAL_STATE.enabled_channels.len(),
    )
}

fn table(entries: &[(u16, u16, f32)]) -> Vec<RouteEntry> {
    entries.iter().map(|&(src, out, gain)| RouteEntry { src, out, gain }).collect()
}

/// 페이드·볼륨 스무딩이 끝날 때까지 돌린 뒤 마지막 블록.
fn settle(m: &mut AudioMixer, width: usize) -> Vec<f32> {
    let mut buf = vec![0.0f32; BLOCK * width];
    for _ in 0..200 {
        buf.fill(0.0);
        m.process(&mut buf, width);
    }
    buf
}

fn channel(buf: &[f32], width: usize, ch: usize) -> Vec<f32> {
    buf.iter().skip(ch).step_by(width).copied().collect()
}

fn peak(v: &[f32]) -> f32 {
    v.iter().fold(0.0f32, |a, &x| a.max(x.abs()))
}

fn assert_close(a: &[f32], b: &[f32], what: &str) {
    let worst = a.iter().zip(b).fold(0.0f32, |w, (x, y)| w.max((x - y).abs()));
    assert!(worst <= 1e-6, "{what}: 차이 {worst}");
}

/// 옛 경로 기준: 파일 채널 하나씩을 모노 인스턴스로 출력에 보낸 믹서.
fn legacy_reference(width: usize, sends: &[(usize, usize)], chs: &[Vec<f32>]) -> Vec<f32> {
    let mut m = mixer(width);
    for (k, &(src, out)) in sends.iter().enumerate() {
        m.instances[k] = Some(inst(10 + k as u32, interleave(&chs[src..src + 1]), out, false));
    }
    settle(&mut m, width)
}

#[test]
fn one_mono_track_splits_to_three_outputs() {
    let _g = lock();
    let chs = tones(1);
    let mut m = mixer(6);
    let mut i = inst(1, interleave(&chs), 0, false);
    i.set_initial_routes(&table(&[(0, 0, 1.0), (0, 2, 1.0), (0, 4, 1.0)]));
    m.instances[0] = Some(i);
    let buf = settle(&mut m, 6);
    let (c1, c3, c5) = (channel(&buf, 6, 0), channel(&buf, 6, 2), channel(&buf, 6, 4));
    assert!(peak(&c1) > 0.1, "CH1에 소리가 없다");
    assert_eq!(c1, c3, "CH1과 CH3이 샘플 단위로 같아야 한다");
    assert_eq!(c1, c5, "CH1과 CH5가 샘플 단위로 같아야 한다");
    assert_eq!(peak(&channel(&buf, 6, 1)), 0.0, "CH2는 무음이어야 한다");
    assert_eq!(peak(&channel(&buf, 6, 3)), 0.0, "CH4는 무음이어야 한다");
    // 옛 경로로 CH1에 보낸 같은 트랙과 같은 소리다.
    assert_close(&c1, &channel(&legacy_reference(6, &[(0, 0)], &chs), 6, 0), "CH1 = 옛 경로 CH1");
}

#[test]
fn each_cell_has_its_own_volume() {
    let _g = lock();
    let chs = tones(1);
    let mut m = mixer(4);
    let mut i = inst(1, interleave(&chs), 0, false);
    i.set_initial_routes(&table(&[(0, 0, 1.0), (0, 1, 0.5)]));
    m.instances[0] = Some(i);
    let buf = settle(&mut m, 4);
    let (a, b) = (channel(&buf, 4, 0), channel(&buf, 4, 1));
    for (x, y) in a.iter().zip(&b) {
        assert!((y - x * 0.5).abs() <= 1e-6, "CH2가 CH1의 절반이 아니다: {x} {y}");
    }
}

#[test]
fn file_channels_can_be_reordered_and_skipped() {
    let _g = lock();
    let chs = tones(4);
    let sends = [(0usize, 2usize), (1, 0), (2, 5), (3, 1)];
    let mut m = mixer(8);
    let mut i = inst(1, interleave(&chs), 0, true);
    i.set_initial_routes(&table(&[(0, 2, 1.0), (1, 0, 1.0), (2, 5, 1.0), (3, 1, 1.0)]));
    m.instances[0] = Some(i);
    let got = settle(&mut m, 8);
    let want = legacy_reference(8, &sends, &chs);
    for ch in 0..8 {
        assert_close(&channel(&got, 8, ch), &channel(&want, 8, ch), &format!("CH{}", ch + 1));
    }
    assert_eq!(peak(&channel(&got, 8, 3)), 0.0, "건너뛴 CH4는 무음");
}

#[test]
fn two_file_channels_mix_into_one_output_as_a_sum() {
    let _g = lock();
    let chs = tones(2);
    let mut m = mixer(4);
    let mut i = inst(1, interleave(&chs), 0, true);
    i.set_initial_routes(&table(&[(0, 1, 1.0), (1, 1, 1.0)]));
    m.instances[0] = Some(i);
    let got = settle(&mut m, 4);
    // 두 채널을 같은 출력으로 보낸 옛 경로 인스턴스 둘의 합과 같다(평균이 아니다).
    let want = legacy_reference(4, &[(0, 1), (1, 1)], &chs);
    assert_close(&channel(&got, 4, 1), &channel(&want, 4, 1), "CH2 = 두 채널의 합");
    assert_eq!(peak(&channel(&got, 4, 0)), 0.0, "CH1은 무음");
}

#[test]
fn a_seeded_table_sounds_like_the_mapping_it_came_from() {
    let _g = lock();
    // 매핑과 같은 소리가 나는 표(설계 7.3, 개정 3): 모노 → out, N채널 스테레오/멀티 → N:N, N채널 모노 출력 → 1/N씩.
    type Case = (usize, usize, bool, Vec<(u16, u16, f32)>);
    let cases: Vec<Case> = vec![
        (1, 2, false, vec![(0, 2, 1.0)]),
        (1, 2, true, vec![(0, 2, 1.0)]), // 모노 파일은 스테레오 출력이어도 자기 채널 하나(1.3.18)
        (2, 2, true, vec![(0, 2, 1.0), (1, 3, 1.0)]), // 스테레오 자리는 짝(CH3/4)으로만 열린다(1.3.17)
        (4, 0, true, vec![(0, 0, 1.0), (1, 1, 1.0), (2, 2, 1.0), (3, 3, 1.0)]),
        (2, 3, false, vec![(0, 3, 0.5), (1, 3, 0.5)]),
    ];
    for (n, out, stereo, cells) in cases {
        let chs = tones(n);
        let mut legacy = mixer(6);
        legacy.instances[0] = Some(inst(1, interleave(&chs), out, stereo));
        let want = settle(&mut legacy, 6);
        let mut matrix = mixer(6);
        let mut i = inst(1, interleave(&chs), out, stereo);
        i.set_initial_routes(&table(&cells));
        matrix.instances[0] = Some(i);
        let got = settle(&mut matrix, 6);
        for ch in 0..6 {
            assert_close(
                &channel(&got, 6, ch),
                &channel(&want, 6, ch),
                &format!("{n}채널 out={out} stereo={stereo} CH{}", ch + 1),
            );
        }
    }
}

#[test]
fn a_closed_output_is_silent_and_the_others_still_sound() {
    let _g = lock();
    let chs = tones(1);
    GLOBAL_STATE.enabled_channels[2].store(false, Ordering::Relaxed);
    let mut m = mixer(4);
    let mut i = inst(1, interleave(&chs), 0, false);
    // CH3은 닫힘, CH40은 믹서 폭 밖, 파일 채널 5는 없음 — 패닉 없이 나머지만 난다.
    i.set_initial_routes(&table(&[(0, 0, 1.0), (0, 2, 1.0), (0, 39, 1.0), (5, 1, 1.0)]));
    m.instances[0] = Some(i);
    let buf = settle(&mut m, 4);
    GLOBAL_STATE.enabled_channels[2].store(true, Ordering::Relaxed);
    assert!(peak(&channel(&buf, 4, 0)) > 0.1, "열린 CH1은 나야 한다");
    assert_eq!(peak(&channel(&buf, 4, 2)), 0.0, "닫힌 CH3은 무음이어야 한다(출력 설정이 이긴다, U1)");
    assert_eq!(peak(&channel(&buf, 4, 1)), 0.0, "파일에 없는 채널의 칸은 무음");
}

#[test]
fn an_empty_table_is_silent_and_does_not_fall_back_to_the_mapping() {
    let _g = lock();
    let chs = tones(1);
    let mut m = mixer(4);
    let mut i = inst(1, interleave(&chs), 0, false);
    i.set_initial_routes(&[]);
    m.instances[0] = Some(i);
    let buf = settle(&mut m, 4);
    assert_eq!(peak(&buf), 0.0, "유효 칸이 0개면 무음이다(설계 D5)");
}

#[test]
fn a_table_track_sends_nothing_to_its_mapping_output() {
    let _g = lock();
    let chs = tones(1);
    let mut m = mixer(4);
    let mut i = inst(1, interleave(&chs), 0, false); // 매핑 값은 CH1
    i.set_initial_routes(&table(&[(0, 3, 1.0)]));
    m.instances[0] = Some(i);
    let buf = settle(&mut m, 4);
    assert_eq!(peak(&channel(&buf, 4, 0)), 0.0, "표 경로 트랙이 매핑 출력(CH1)으로도 나갔다(겹쳐 나감)");
    assert!(peak(&channel(&buf, 4, 3)) > 0.1, "표의 CH4로 나가지 않았다");
}

#[test]
fn the_output_kind_rule_applies_to_the_mapping_only() {
    let _g = lock();
    let chs = tones(1);
    // 모노 줄을 모두 끈다: 매핑 경로의 모노 트랙은 자리 종류가 맞지 않아 무음, 표 경로 트랙은 칸마다 게이트만 본다.
    for f in &GLOBAL_STATE.mono_rows_open {
        f.store(false, Ordering::Relaxed);
    }
    let mut m = mixer(4);
    m.instances[0] = Some(inst(1, interleave(&chs), 0, false));
    let mut t = inst(2, interleave(&chs), 1, false);
    t.set_initial_routes(&table(&[(0, 1, 1.0)]));
    m.instances[1] = Some(t);
    let buf = settle(&mut m, 4);
    for f in &GLOBAL_STATE.mono_rows_open {
        f.store(true, Ordering::Relaxed);
    }
    assert_eq!(peak(&channel(&buf, 4, 0)), 0.0, "모노 줄이 꺼진 매핑 트랙은 무음이어야 한다");
    assert!(peak(&channel(&buf, 4, 1)) > 0.1, "표 경로 트랙은 자리 종류 검사를 받지 않는다(설계 개정 3 ③)");
}

#[test]
fn a_table_reads_file_channels_beyond_the_mixer_width() {
    let _g = lock();
    let chs = tones(24);
    let mut m = mixer(16);
    let mut i = inst(1, interleave(&chs), 0, true);
    i.set_initial_routes(&table(&[(20, 2, 1.0)]));
    m.instances[0] = Some(i);
    let got = settle(&mut m, 16);
    let want = legacy_reference(16, &[(20, 2)], &chs);
    assert_close(&channel(&got, 16, 2), &channel(&want, 16, 2), "파일 채널 21 → CH3");

    // 같은 파일을 표 없이 틀면 예전처럼 믹서 폭(16채널)까지만 N:N으로 난다.
    let mut legacy = mixer(16);
    legacy.instances[0] = Some(inst(1, interleave(&chs), 0, true));
    let got = settle(&mut legacy, 16);
    let sends: Vec<(usize, usize)> = (0..16).map(|c| (c, c)).collect();
    let want = legacy_reference(16, &sends, &chs);
    for ch in 0..16 {
        assert_close(&channel(&got, 16, ch), &channel(&want, 16, ch), &format!("옛 경로 CH{}", ch + 1));
    }
}
