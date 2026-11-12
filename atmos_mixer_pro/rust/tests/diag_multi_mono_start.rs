//! 수동 진단: 여러 모노 파일을 함께 틀 때 채널마다 첫 소리가 몇 프레임 어긋나는가(시작 오차)와, 재생 중 그 어긋남이
//! 변하는가(밀림).
//!
//! 앱의 실제 시작 경로를 그대로 쓴다. 대시보드 Start·OSC 테마 시작·로그인 자동 실행이 모두 부르는
//! `api_theme_start`(전체 정지 → 루프 트랙마다 DiskStreamer를 만들고 재생 명령을 보냄)를 부르고, 장치 대신
//! 시험 스레드가 오디오 콜백과 같은 함수(`AudioEngine::begin_callback` + `render_block`)를 실제 시간 간격
//! (1024프레임 = 21.3ms)으로 부른다. 장치를 열지 않으므로 소리가 나지 않는다. 앱에서는 테마 시작이 콜백 주기의
//! 아무 때나 불리므로, 시도마다 부르는 때를 주기 안에서 고르게 옮긴다.
//!
//! 시험 파일은 48kHz 32bit float 모노 잡음이고 모든 트랙이 같은 내용이다(파일은 트랙마다 따로). 샘플 크기는
//! 0.05~0.2라 0인 샘플이 없다.
//! - 첫 비영 샘플: 채널마다 처음 소리가 난 프레임. 출력 리미터의 5ms 미리보기 지연은 모든 채널에 같아서 차이에는
//!   영향이 없다.
//! - 상관: 페이드 인이 끝난 뒤의 구간에서 기준 채널(가장 먼저 난 채널)과 각 채널의 상호상관이 가장 큰 지연을 찾는다.
//!   내용의 실제 어긋남을 샘플 단위로 다시 확인하고, 구간마다 다시 재서 재생 중 한 트랙만 밀리는지 본다.
//!
//! 자동 테스트가 아니다(#[ignore]). 실행(rust 폴더):
//!   DIAG_DIR=<시험 파일 폴더> DIAG_COUNTS=12,16,40,64 DIAG_TRIALS=10 \
//!   cargo test --release --test diag_multi_mono_start -- --ignored --nocapture --test-threads=1
//! 그 밖의 변수: DIAG_HOLD_SECS(시작 뒤 지켜보는 시간, 기본 4), DIAG_WINDOW_EVERY_SECS(상관 구간 간격, 기본 1),
//! DIAG_FILE_SECS(파일 길이, 기본 20).

use rust_lib_atmos_mixer_pro::api::show::{api_resume_show, api_start_show_state, api_theme_start};
use rust_lib_atmos_mixer_pro::audio::engine::{AudioEngine, ENGINE_GENERATION};
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::common::config::{AppConfig, RoomConfig, TrackConfig};
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const FS: u32 = 48_000;
const BLOCK: usize = 1024;
/// 이보다 크면 소리가 났다고 본다. 첫 샘플(0.05 × 페이드 1/14400 ≈ 3.5e-6)보다 작다.
const THRESHOLD: f32 = 1e-6;
/// 상관 구간 길이(프레임).
const WINDOW: usize = 8 * BLOCK;
/// 테마 시작 뒤 이만큼 지나야 상관 구간을 뜬다(첫 소리 수십 ms + 페이드 300ms + 여유).
const WINDOW_AFTER: u64 = FS as u64 * 6 / 10;
/// 첫 비영 샘플로 짐작한 지연 주변에서 찾는 폭, 그래도 상관이 낮으면 넓게 찾는 폭.
const NEAR: i64 = 16;
const WIDE: i64 = 3 * BLOCK as i64;
/// 같은 내용이 맞다고 볼 정규화 상관의 하한.
const MATCH: f64 = 0.9;
/// 테마 시작 뒤 아직 부르지 않음.
const NOT_CALLED: u64 = u64::MAX;

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

fn env_list(name: &str, default: &[usize]) -> Vec<usize> {
    let parsed: Vec<usize> = std::env::var(name)
        .map(|v| v.split(',').filter_map(|s| s.trim().parse().ok()).collect())
        .unwrap_or_default();
    if parsed.is_empty() {
        default.to_vec()
    } else {
        parsed
    }
}

/// 모든 트랙이 같은 잡음(xorshift32 고정 씨앗). 크기 0.05~0.2, 부호는 무작위.
fn noise_wav(path: &std::path::Path, seconds: usize) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: FS,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(path, spec).expect("시험 파일 만들기");
    let mut x: u32 = 0x2545_F491;
    for _ in 0..seconds * FS as usize {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        let u = (x >> 8) as f32 / (1u32 << 24) as f32;
        let v = 0.05 + 0.15 * u;
        w.write_sample(if x & 1 == 0 { v } else { -v }).expect("시험 파일 쓰기");
    }
    w.finalize().expect("시험 파일 닫기");
}

/// 방 하나에 모노 루프 트랙 n개. k번째 트랙은 k번째 파일을 출력 k(CH k+1)로 낸다.
fn config(files: &[String], n: usize) -> AppConfig {
    let tracks = (0..n)
        .map(|k| TrackConfig {
            id: format!("t{k:02}"),
            name: format!("t{k:02}"),
            file_path: files[k].clone(),
            volume: 1.0,
            is_loop: true,
            is_streaming: true,
            output_channel: k as u32,
            output_stereo: false,
            play_osc_address: String::new(),
            stop_osc_address: String::new(),
        })
        .collect();
    AppConfig {
        rooms: vec![RoomConfig {
            id: "r".into(),
            name: "r".into(),
            color_hex: "#ffffff".into(),
            volume: 1.0,
            clear_osc_address: String::new(),
            tracks,
        }],
        ..AppConfig::default()
    }
}

struct Capture {
    /// 출력 채널별 첫 비영 샘플의 전역 프레임(시험 스레드가 시작한 때가 0).
    first: Vec<Option<u64>>,
    /// 상관 구간들: (첫 프레임, 채널별 WINDOW개 샘플).
    windows: Vec<(u64, Vec<Vec<f32>>)>,
    /// 마감보다 2ms 넘게 늦게 돈 블록 수(이 시험 스레드 자체의 지연).
    late_blocks: u32,
}

/// 장치 콜백 대신 1024프레임마다 콜백과 같은 두 함수를 부른다.
fn run_device(
    mut mixer: AudioMixer,
    width: usize,
    stop: Arc<AtomicBool>,
    call_frame: Arc<AtomicU64>,
    window_every: u64,
    t0: Instant,
) -> (AudioMixer, Capture) {
    let generation = ENGINE_GENERATION.load(Ordering::SeqCst);
    let rx = GLOBAL_STATE.command_receiver.clone();
    let mut temp = vec![0.0f32; BLOCK * width];
    let mut data = vec![0.0f32; BLOCK * width];
    let mut cap = Capture { first: vec![None; width], windows: Vec::new(), late_blocks: 0 };
    let mut next_window = u64::MAX;
    let mut k: u64 = 0;
    while !stop.load(Ordering::SeqCst) {
        let deadline = t0 + Duration::from_secs_f64((k * BLOCK as u64) as f64 / FS as f64);
        let now = Instant::now();
        if deadline > now {
            std::thread::sleep(deadline - now);
        } else if now - deadline > Duration::from_millis(2) {
            cap.late_blocks += 1;
        }
        if AudioEngine::begin_callback(&mut mixer, &rx, generation) {
            AudioEngine::render_block(&mut mixer, &mut temp, &mut data, width, width);
        } else {
            data.fill(0.0);
        }
        let base = k * BLOCK as u64;
        for c in 0..width {
            if cap.first[c].is_none() {
                if let Some(f) = (0..BLOCK).find(|&f| data[f * width + c].abs() > THRESHOLD) {
                    cap.first[c] = Some(base + f as u64);
                }
            }
        }
        let called = call_frame.load(Ordering::SeqCst);
        if called != NOT_CALLED && next_window == u64::MAX {
            next_window = called + WINDOW_AFTER;
        }
        // 구간 하나를 블록 단위로 채운다. 마지막 구간이 다 차기 전에 멈추면 그 구간은 버린다.
        let filling =
            cap.windows.last().is_some_and(|(start, chans)| base < start + WINDOW as u64 && chans[0].len() < WINDOW);
        if !filling && base >= next_window {
            cap.windows.push((base, vec![Vec::with_capacity(WINDOW); width]));
            next_window = base + window_every;
        }
        if let Some((start, chans)) = cap.windows.last_mut() {
            if base < *start + WINDOW as u64 {
                for (c, chan) in chans.iter_mut().enumerate() {
                    chan.extend((0..BLOCK).map(|f| data[f * width + c]));
                }
            }
        }
        k += 1;
    }
    if cap.windows.last().is_some_and(|(_, chans)| chans[0].len() < WINDOW) {
        cap.windows.pop();
    }
    (mixer, cap)
}

/// b가 a보다 lag 프레임 늦을 때의 정규화 상관.
fn correlation(a: &[f32], b: &[f32], lag: i64) -> f64 {
    let n = a.len().min(b.len()) as i64;
    let (from, to) = (0i64.max(-lag), n.min(n - lag));
    if to - from < (n / 2) {
        return 0.0;
    }
    let (mut ab, mut aa, mut bb) = (0.0f64, 0.0f64, 0.0f64);
    for t in from..to {
        let x = a[t as usize] as f64;
        let y = b[(t + lag) as usize] as f64;
        ab += x * y;
        aa += x * x;
        bb += y * y;
    }
    if aa <= 0.0 || bb <= 0.0 {
        0.0
    } else {
        ab / (aa * bb).sqrt()
    }
}

/// guess 주변에서 상관이 가장 큰 지연. 낮으면 넓게 다시 찾는다. (지연, 상관).
fn best_lag(a: &[f32], b: &[f32], guess: i64) -> (i64, f64) {
    let search = |lo: i64, hi: i64| {
        (lo..=hi).map(|l| (l, correlation(a, b, l))).fold((0i64, f64::MIN), |m, x| if x.1 > m.1 { x } else { m })
    };
    let near = search(guess - NEAR, guess + NEAR);
    if near.1 >= MATCH {
        near
    } else {
        search(-WIDE, WIDE)
    }
}

struct Trial {
    /// 테마 시작을 부른 프레임과 그때부터 다음 콜백 경계까지(프레임).
    call_frame: u64,
    to_boundary: u64,
    api_ms: f64,
    /// (출력 채널, 첫 비영 샘플 프레임).
    starts: Vec<(usize, u64)>,
    missing: usize,
    /// 첫 상관 구간의 채널별 지연(기준 채널보다 늦은 프레임)과 그 상관.
    lags: Vec<(usize, i64, f64)>,
    /// 첫 비영 샘플 차이와 상관 지연이 다른 채널 수.
    disagree: usize,
    /// 구간마다 다시 잰 지연이 첫 구간과 달라진 최대 폭(프레임)과 상관이 낮았던 경우 수.
    drift: i64,
    weak: usize,
    windows: usize,
    late_blocks: u32,
}

/// 테마 시작(대시보드 Start·OSC·로그인 자동 실행이 쓰는 함수).
fn theme_start() -> Result<(), String> {
    api_theme_start().map_err(|e| e.message)
}

fn trial(files: &[String], n: usize, hold: Duration, window_every: u64, phase: f64) -> Trial {
    trial_with(files, n, hold, window_every, phase, &theme_start)
}

/// `start`(테마 시작이나 이어 가기)를 콜백 주기의 phase(0~1) 지점에서 부르고 채널별 시작을 잰다.
fn trial_with(
    files: &[String],
    n: usize,
    hold: Duration,
    window_every: u64,
    phase: f64,
    start: &dyn Fn() -> Result<(), String>,
) -> Trial {
    let width = n.max(16);
    // 이전 시도의 명령·재생 목록이 남지 않게 비운다.
    while GLOBAL_STATE.command_receiver.try_recv().is_ok() {}
    GLOBAL_STATE.clear_playing_tracks();
    *GLOBAL_STATE.config.write().expect("설정 잠금") = Some(config(files, n));
    for c in 0..width {
        GLOBAL_STATE.enabled_channels[c].store(true, Ordering::SeqCst);
    }

    let (gc_tx, gc_rx) = crossbeam_channel::unbounded();
    let mut mixer = AudioMixer::new(FS, width, BLOCK, gc_tx, None);
    // 앱에서는 엔진이 켜진 지 한참 뒤에 테마가 시작되므로 시작 램프는 끝나 있다.
    mixer.startup_ramp.current_gain = 1.0;
    let stop = Arc::new(AtomicBool::new(false));
    let call_frame = Arc::new(AtomicU64::new(NOT_CALLED));
    let t0 = Instant::now();
    let device = {
        let (stop, call_frame) = (stop.clone(), call_frame.clone());
        std::thread::spawn(move || run_device(mixer, width, stop, call_frame, window_every, t0))
    };

    // 1초 뒤 + 콜백 주기의 phase(0~1)만큼 더 기다렸다가 부른다.
    let at = t0 + Duration::from_secs_f64(1.0 + phase * BLOCK as f64 / FS as f64);
    std::thread::sleep(at.saturating_duration_since(Instant::now()));
    let call = t0.elapsed();
    let called = (call.as_secs_f64() * FS as f64) as u64;
    call_frame.store(called, Ordering::SeqCst);
    let started = Instant::now();
    if let Err(e) = start() {
        println!("  시작 오류: {e}");
    }
    let api_ms = started.elapsed().as_secs_f64() * 1000.0;
    std::thread::sleep(hold);
    stop.store(true, Ordering::SeqCst);
    let (mixer, cap) = device.join().expect("시험 스레드");
    drop(mixer); // 인스턴스와 디코더 스레드를 여기서 정리한다
    drop(gc_rx);

    let starts: Vec<(usize, u64)> = (0..width).filter_map(|c| cap.first[c].map(|f| (c, f))).collect();
    let missing = n.saturating_sub(starts.len());
    let (mut lags, mut disagree, mut drift, mut weak) = (Vec::new(), 0usize, 0i64, 0usize);
    if let Some(&(reference, ref_first)) = starts.iter().min_by_key(|s| s.1) {
        for &(c, f) in &starts {
            let guess = f as i64 - ref_first as i64;
            let mut first_lag: Option<i64> = None;
            for (w, (_, chans)) in cap.windows.iter().enumerate() {
                let (lag, r) = best_lag(&chans[reference], &chans[c], first_lag.unwrap_or(guess));
                if r < MATCH {
                    weak += 1;
                    continue;
                }
                match first_lag {
                    None => {
                        first_lag = Some(lag);
                        if w == 0 {
                            lags.push((c, lag, r));
                            if lag != guess {
                                disagree += 1;
                            }
                        }
                    }
                    Some(l0) => drift = drift.max((lag - l0).abs()),
                }
            }
        }
    }
    Trial {
        call_frame: called,
        to_boundary: (BLOCK as u64 - called % BLOCK as u64) % BLOCK as u64,
        api_ms,
        starts,
        missing,
        lags,
        disagree,
        drift,
        weak,
        windows: cap.windows.len(),
        late_blocks: cap.late_blocks,
    }
}

fn ms(frames: f64) -> f64 {
    frames * 1000.0 / FS as f64
}

fn env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name).ok().and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

fn test_dir() -> std::path::PathBuf {
    std::env::var("DIAG_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("atmos_multi_mono_start"))
}

/// 트랙 n개의 시험 파일(같은 잡음, 파일은 따로). 이미 있으면 그대로 쓴다.
fn noise_files(dir: &std::path::Path, file_secs: usize, n: usize) -> Vec<String> {
    std::fs::create_dir_all(dir).expect("시험 폴더");
    (0..n)
        .map(|k| {
            let path = dir.join(format!("noise_{file_secs}s_{k:02}.wav"));
            if !path.exists() {
                noise_wav(&path, file_secs);
            }
            path.to_string_lossy().into_owned()
        })
        .collect()
}

/// 시도 하나를 한 줄로 남기고 시작 차이(프레임)를 돌려준다.
fn print_trial(n: usize, label: &str, r: &Trial) -> u64 {
    let first = r.starts.iter().map(|s| s.1).min().unwrap_or(0);
    let last = r.starts.iter().map(|s| s.1).max().unwrap_or(0);
    let spread = last - first;
    let lag_spread = r.lags.iter().map(|l| l.1).max().unwrap_or(0) - r.lags.iter().map(|l| l.1).min().unwrap_or(0);
    let min_r = r.lags.iter().map(|l| l.2).fold(f64::MAX, f64::min);
    let mut groups = std::collections::BTreeMap::new();
    for &(_, f) in &r.starts {
        *groups.entry(((f - first) as f64 / BLOCK as f64).round() as i64).or_insert(0usize) += 1;
    }
    let off_block = r
        .starts
        .iter()
        .filter(|s| (s.1 - first) % BLOCK as u64 != 0)
        .map(|s| format!("CH{}:{}", s.0 + 1, s.1 - first))
        .collect::<Vec<_>>();
    println!(
        "N={n:>2} {label}: 부른 뒤 다음 콜백까지 {:>5.1}ms, 시작 함수 {:>5.1}ms, 첫 소리 {:>5.1}~{:>5.1}ms, \
         시작 차이 {:>5}프레임({:>5.1}ms), 콜백 무리별 트랙 수 {groups:?}, 콜백 경계 밖 {off_block:?}, 못 들음 {}, \
         상관 지연 폭 {lag_spread}프레임(최저 상관 {min_r:.3}, 첫 비영과 다름 {}), 밀림 {}프레임(구간 {}개, 낮은 상관 {}), 늦은 블록 {}",
        ms(r.to_boundary as f64),
        r.api_ms,
        ms(first.saturating_sub(r.call_frame) as f64),
        ms(last.saturating_sub(r.call_frame) as f64),
        spread,
        ms(spread as f64),
        r.missing,
        r.disagree,
        r.drift,
        r.windows,
        r.weak,
        r.late_blocks
    );
    spread
}

#[test]
#[ignore]
fn 여러_모노_루프의_시작_오차() {
    let counts = env_list("DIAG_COUNTS", &[12, 16, 40, 64]);
    let trials = env_usize("DIAG_TRIALS", 10).max(1);
    let hold = Duration::from_secs(env_usize("DIAG_HOLD_SECS", 4) as u64);
    let window_every = env_usize("DIAG_WINDOW_EVERY_SECS", 1).max(1) as u64 * FS as u64;
    let file_secs = env_usize("DIAG_FILE_SECS", 20);
    let dir = test_dir();
    let max_n = counts.iter().copied().max().unwrap_or(0);
    let files = noise_files(&dir, file_secs, max_n);
    GLOBAL_STATE.engine_sample_rate.store(FS, Ordering::SeqCst);
    println!(
        "시험: 48kHz, 블록 {BLOCK}프레임({:.1}ms), 트랙 수 {counts:?}, 시도 {trials}회(부르는 때를 콜백 주기 안에서 고르게), \
         시작 뒤 {}초, 상관 구간 {WINDOW}프레임을 {}초마다, 파일 {}초 × {max_n}개({})",
        ms(BLOCK as f64),
        hold.as_secs(),
        window_every / FS as u64,
        file_secs,
        dir.display()
    );

    for &n in &counts {
        let mut spreads = Vec::new();
        let mut split = 0usize;
        let mut worst: Option<(u64, Trial)> = None;
        for t in 0..trials {
            let r = trial(&files, n, hold, window_every, t as f64 / trials as f64);
            let spread = print_trial(n, &format!("시도 {:>2}", t + 1), &r);
            if spread > 0 {
                split += 1;
            }
            if worst.as_ref().is_none_or(|w| spread > w.0) {
                worst = Some((spread, r));
            }
            spreads.push(spread);
        }
        spreads.sort();
        println!(
            "== N={n}: 시작 차이 최소/중앙/최대 {}/{}/{}프레임 ({:.1}/{:.1}/{:.1}ms), 0이 아닌 시도 {split}/{trials}",
            spreads[0],
            spreads[spreads.len() / 2],
            spreads[spreads.len() - 1],
            ms(spreads[0] as f64),
            ms(spreads[spreads.len() / 2] as f64),
            ms(spreads[spreads.len() - 1] as f64)
        );
        if let Some((_, w)) = worst {
            let per: Vec<String> = w.lags.iter().map(|&(c, l, _)| format!("CH{}:{}", c + 1, l)).collect();
            println!("   가장 어긋난 시도의 채널별 늦음(상관, 프레임): {}", per.join(" "));
        }
    }
}

/// 수동 진단: 앱이 다시 켜져 공연을 이어 갈 때(`api_start_show_state` → `api_resume_show`, core::show_state::resume)
/// 여러 모노 루프의 시작 오차. 모든 트랙이 같은 위치(DIAG_RESUME_SECS)에서 이어 가므로 맞으면 차이가 0이어야 한다.
/// 이어 가기는 DiskStreamer가 시작 위치까지 해독해 버린 뒤 첫 묶음을 보내므로 위치가 뒤일수록 첫 소리가 늦다.
/// 지난 상태 읽기가 프로세스마다 한 번이라(show_state::start의 Once) 한 번 실행에 한 번 잰다. 부르는 때(DIAG_PHASE,
/// 콜백 주기의 0~1)를 바꿔 가며 되풀이하는 것은 스크립트가 한다. 실행(rust 폴더):
///   DIAG_DIR=<시험 파일 폴더> DIAG_COUNT=16 DIAG_FILE_SECS=300 DIAG_RESUME_SECS=290 DIAG_PHASE=0.3 \
///   cargo test --release --test diag_multi_mono_start 여러_모노_이어_가기 -- --ignored --nocapture
#[test]
#[ignore]
fn 여러_모노_이어_가기_시작_오차() {
    let n = env_usize("DIAG_COUNT", 16);
    let file_secs = env_usize("DIAG_FILE_SECS", 300);
    let resume_secs = env_f64("DIAG_RESUME_SECS", 60.0);
    let phase = env_f64("DIAG_PHASE", 0.0).clamp(0.0, 0.999);
    let hold = Duration::from_secs(env_usize("DIAG_HOLD_SECS", 4) as u64);
    let dir = test_dir();
    let files = noise_files(&dir, file_secs, n);
    GLOBAL_STATE.engine_sample_rate.store(FS, Ordering::SeqCst);

    // 방금 저장한 공연 상태: 모든 트랙이 같은 위치.
    let state_dir = dir.join(format!("show_state_{}", std::process::id()));
    std::fs::create_dir_all(&state_dir).expect("상태 폴더");
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let tracks: Vec<serde_json::Value> = (0..n)
        .map(|k| serde_json::json!({ "room_id": "r", "track_id": format!("t{k:02}"), "seconds": resume_secs }))
        .collect();
    let state = serde_json::json!({ "saved_at_ms": now_ms, "active_room_id": "r", "tracks": tracks });
    std::fs::write(state_dir.join("show_state.json"), state.to_string()).expect("상태 파일");
    *GLOBAL_STATE.config.write().expect("설정 잠금") = Some(config(&files, n));
    api_start_show_state(state_dir.to_string_lossy().into_owned());

    println!(
        "이어 가기 시험: 48kHz, 블록 {BLOCK}프레임, 트랙 {n}개가 모두 {resume_secs}초에서 이어 감, 파일 {file_secs}초, \
         부르는 때 콜백 주기의 {phase:.2}"
    );
    let resume = || api_resume_show().map_err(|e| e.message);
    let r = trial_with(&files, n, hold, FS as u64, phase, &resume);
    print_trial(n, &format!("이어 가기 {resume_secs}초"), &r);
    let per: Vec<String> = r.lags.iter().map(|&(c, l, _)| format!("CH{}:{}", c + 1, l)).collect();
    println!("   채널별 늦음(상관, 프레임): {}", per.join(" "));
    let _ = std::fs::remove_dir_all(&state_dir);
}
