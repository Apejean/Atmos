//! 검토 A6: 디스크 스트리밍이 파일을 다시 열지 못해도 말없이 멈추지 않는다.
//! - 루프가 한 바퀴 끝나 다시 열 때 실패하면 1·2·5초 뒤, 그다음 10초마다 계속 다시 연다. 그동안 그 트랙만 무음이고
//!   디코더 스레드는 살아 있어 믹서가 트랙을 끝내지 않는다. 실패 중인 트랙은 상태 스트림 `ReadFailing:`로 알린다.
//! - 다시 열리면 끊긴 자리(마지막으로 보낸 다음 프레임)부터 이어 보낸다. 그룹 루프는 믹서가 밀린 만큼 건너뛰어
//!   끊기지 않은 루프와 위상이 같아진다.
//! - 기다리는 중에도 정지(스트리머 버림)에 곧바로 반응한다.
//! 전역 상태(실패 목록·믹서 채널 켜기)를 쓰므로 이 파일의 시험은 하나씩 돈다.
use rust_lib_atmos_mixer_pro::audio::engine::{AudioEngine, ENGINE_GENERATION};
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::SoundInstance;
use rust_lib_atmos_mixer_pro::audio::streaming::{reopen_delay, DiskStreamer};
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::core::read_failures;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const FS: u32 = 48_000;
static SERIAL: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("atmos_reopen_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// i번째 프레임 값 = (i+1) × 1e-6 인 32비트 float 모노 WAV(값으로 위치를 알 수 있다, 0은 무음과 구분하려고 피함).
fn ramp_wav(path: &Path, frames: usize) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: FS,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..frames {
        w.write_sample((i + 1) as f32 * 1e-6).unwrap();
    }
    w.finalize().unwrap();
}

fn frame_of(v: f32) -> i64 {
    (v as f64 / 1e-6).round() as i64 - 1
}

fn wait_ready(s: &DiskStreamer) {
    let t0 = Instant::now();
    while !s.first_chunk_ready.load(Ordering::Acquire) {
        assert!(t0.elapsed() < Duration::from_secs(3), "첫 묶음이 3초 안에 오지 않았다");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn 다시_열기_간격은_1_2_5초_뒤_그다음_10초마다다() {
    assert_eq!(reopen_delay(1), Duration::from_secs(1));
    assert_eq!(reopen_delay(2), Duration::from_secs(2));
    assert_eq!(reopen_delay(3), Duration::from_secs(5));
    assert_eq!(reopen_delay(4), Duration::from_secs(10));
    assert_eq!(reopen_delay(50), Duration::from_secs(10));
}

#[test]
fn 상태_문자열은_실패_수와_첫_파일_이름이다() {
    let _g = lock();
    assert_eq!(read_failures::status(), "ReadFailing:0");
    assert_eq!(read_failures::current_status(), None, "실패가 없으면 처음 값을 보내지 않는다");
    let a = read_failures::begin(Path::new("D:/audio/zone 1.wav"));
    assert_eq!(read_failures::status(), "ReadFailing:1:zone 1.wav", "경로가 아니라 마지막 이름만");
    let b = read_failures::begin(Path::new("D:/audio/b:c.wav"));
    assert_eq!(read_failures::status(), "ReadFailing:2:zone 1.wav", "첫 파일은 먼저 실패한 것");
    assert_eq!(read_failures::current_status().as_deref(), Some("ReadFailing:2:zone 1.wav"));
    drop(a);
    assert_eq!(read_failures::status(), "ReadFailing:1:b:c.wav", "이름에 ':'가 있어도 맨 뒤에 둔다");
    drop(b);
    assert_eq!(read_failures::status(), "ReadFailing:0");
    assert_eq!(read_failures::failing_count(), 0);
}

#[test]
fn 루프를_다시_열지_못하면_멈추지_않고_기다렸다가_돌아오면_끊긴_자리부터_잇는다() {
    let _g = lock();
    let dir = temp_dir("loop");
    let path = dir.join("loop_a.wav");
    let moved = dir.join("loop_a_moved.wav");
    let frames = FS as usize / 4; // 0.25초 루프
    ramp_wav(&path, frames);

    let mut s = DiskStreamer::new(path.to_string_lossy().to_string(), true, FS).unwrap();
    wait_ready(&s);
    // 디코더는 이미 파일을 열었다. 다음 바퀴에서 다시 열 때 실패하게 치운다(열린 핸들로는 계속 읽힌다).
    std::fs::rename(&path, &moved).unwrap();

    let mut rx = s.chunk_receiver.take().unwrap();
    let mut values: Vec<f32> = Vec::new();
    // 링이 마를 때까지 꺼낸다. 마른 뒤에도 스레드는 살아 있어야 한다(믹서가 끝내지 않게).
    let mut idle_since = Instant::now();
    let t0 = Instant::now();
    loop {
        match rx.pop() {
            Ok(c) => {
                values.extend_from_slice(&c);
                idle_since = Instant::now();
            }
            Err(_) => {
                if idle_since.elapsed() > Duration::from_millis(400) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        assert!(t0.elapsed() < Duration::from_secs(5), "링이 마르지 않는다(다시 열기가 실패하지 않았다)");
    }
    assert!(!rx.is_abandoned(), "다시 열기에 실패하자 디코더 스레드가 끝났다(믹서가 루프를 멈춘다)");
    assert_eq!(read_failures::failing_count(), 1, "실패 중인 트랙이 상태에 없다");
    assert_eq!(read_failures::status(), "ReadFailing:1:loop_a.wav");
    let last_before = *values.last().expect("다시 열기 전 소리가 없다");
    assert_eq!(frame_of(last_before), frames as i64 - 1, "바퀴 끝까지 보내고 멈춰야 한다");

    // 파일이 돌아오면 다음 시도(1초 뒤)에서 이어 간다.
    std::fs::rename(&moved, &path).unwrap();
    let back = Instant::now();
    let first_after = loop {
        if let Ok(c) = rx.pop() {
            break c[0];
        }
        assert!(back.elapsed() < Duration::from_secs(4), "파일이 돌아왔는데 다시 열지 않는다");
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(frame_of(first_after), 0, "루프는 다음 바퀴 처음부터 이어야 한다");
    assert_eq!(read_failures::failing_count(), 0, "회복했는데 실패 상태가 남았다");
    assert_eq!(read_failures::status(), "ReadFailing:0");

    s.chunk_receiver = Some(rx);
    drop(s);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn 다시_열기를_기다리는_중에도_정지하면_곧바로_끝나고_실패_상태를_지운다() {
    let _g = lock();
    let dir = temp_dir("stop");
    let path = dir.join("loop_b.wav");
    let moved = dir.join("loop_b_moved.wav");
    ramp_wav(&path, FS as usize / 8);

    let mut s = DiskStreamer::new(path.to_string_lossy().to_string(), true, FS).unwrap();
    wait_ready(&s);
    std::fs::rename(&path, &moved).unwrap();
    let mut rx = s.chunk_receiver.take().unwrap();
    let t0 = Instant::now();
    while read_failures::failing_count() == 0 {
        while rx.pop().is_ok() {}
        assert!(t0.elapsed() < Duration::from_secs(5), "다시 열기 실패 상태가 되지 않는다");
        std::thread::sleep(Duration::from_millis(5));
    }
    std::thread::sleep(Duration::from_millis(300)); // 기다림 한가운데
    let stop = Instant::now();
    drop(s); // 정지: 실행 깃발을 내리고 스레드를 기다린다
    let took = stop.elapsed();
    assert!(took < Duration::from_millis(200), "정지에 {took:?} 걸렸다(100ms 단위로 반응해야 한다)");
    assert_eq!(read_failures::failing_count(), 0, "정지한 트랙이 실패 상태에 남았다");
    drop(rx);
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- 믹서까지: 끊겼다 돌아온 그룹 루프와 끊기지 않은 그룹 루프의 위상 ----

const CH: usize = 2;
const BLOCK: usize = 480;
const GROUP: u64 = 91;

struct Rig {
    mixer: AudioMixer,
    tx: crossbeam_channel::Sender<AudioCommand>,
    rx: crossbeam_channel::Receiver<AudioCommand>,
    temp: Vec<f32>,
    out: Vec<f32>,
    _gc_rx: crossbeam_channel::Receiver<SoundInstance>,
}

impl Rig {
    fn new() -> Self {
        for ch in 0..CH {
            GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
        }
        let (gc_tx, gc_rx) = crossbeam_channel::unbounded();
        let mut mixer = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
        mixer.startup_ramp.current_gain = 1.0;
        let (tx, rx) = crossbeam_channel::unbounded();
        Rig { mixer, tx, rx, temp: vec![0.0; BLOCK * CH], out: vec![0.0; BLOCK * CH], _gc_rx: gc_rx }
    }

    fn block(&mut self) {
        let generation = ENGINE_GENERATION.load(Ordering::SeqCst);
        assert!(AudioEngine::begin_callback(&mut self.mixer, &self.rx, generation));
        AudioEngine::render_block(&mut self.mixer, &mut self.temp, &mut self.out, CH, CH);
    }

    fn instance(&self, id: u64) -> &SoundInstance {
        self.mixer.instances.iter().flatten().find(|i| i.instance_id == id).expect("인스턴스가 풀에 없다")
    }
}

fn armed_loop(id: u64, out: usize, path: &Path) -> SoundInstance {
    let s = DiskStreamer::new(path.to_string_lossy().to_string(), true, FS).unwrap();
    wait_ready(&s);
    let mut inst = SoundInstance::new(
        id, id as u32, 1, format!("t{id}"), None, Some(s), FS, 1, true, 1.0, out, false, None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.arm_for_group(GROUP);
    inst
}

#[test]
fn 끊겼다_돌아온_그룹_루프는_끊기지_않은_루프와_프레임_차이가_없다() {
    let _g = lock();
    let dir = temp_dir("phase");
    let a_path = dir.join("phase_a.wav");
    let b_path = dir.join("phase_b.wav");
    let b_moved = dir.join("phase_b_moved.wav");
    let frames = FS as usize / 2; // 0.5초 루프
    ramp_wav(&a_path, frames);
    ramp_wav(&b_path, frames);

    let mut rig = Rig::new();
    rig.tx.send(AudioCommand::PlayTrack { instance: armed_loop(1, 0, &a_path), room_volume: 1.0 }).unwrap();
    rig.tx.send(AudioCommand::PlayTrack { instance: armed_loop(2, 1, &b_path), room_volume: 1.0 }).unwrap();
    rig.tx.send(AudioCommand::StartGroup { group_id: GROUP }).unwrap();
    rig.block();
    std::fs::rename(&b_path, &b_moved).unwrap();

    // B가 링을 다 쓰고 밀리기 시작할 때까지 돌린다(디코더는 다음 바퀴를 열지 못한다).
    let t0 = Instant::now();
    while rig.instance(2).lag_frames <= 0.0 {
        rig.block();
        std::thread::sleep(Duration::from_micros(200));
        assert!(t0.elapsed() < Duration::from_secs(10), "B가 밀리지 않는다(다시 열기가 실패하지 않았다)");
    }
    assert_eq!(read_failures::failing_count(), 1);
    // 1.3초 동안 끊긴 채 돈다(실시간 속도로, 디코더의 다시 열기는 벽시계를 따른다).
    let outage = Instant::now();
    while outage.elapsed() < Duration::from_millis(1300) {
        rig.block();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(rig.instance(2).lag_frames > 0.5 * FS as f64, "끊긴 동안 밀린 프레임을 세지 않았다");
    std::fs::rename(&b_moved, &b_path).unwrap();

    // 다시 열려 따라잡을 때까지 돌린다.
    let back = Instant::now();
    while read_failures::failing_count() > 0 || rig.instance(2).lag_frames > 0.0 || rig.instance(2).rejoin_gain < 1.0 {
        rig.block();
        std::thread::sleep(Duration::from_millis(10));
        assert!(back.elapsed() < Duration::from_secs(15), "B가 돌아와 따라잡지 못했다");
    }
    for _ in 0..20 {
        rig.block();
    }
    let (pa, pb) = (rig.instance(1).position_seconds(), rig.instance(2).position_seconds());
    let diff_frames = ((pa - pb) * FS as f64).abs();
    // 위치 숫자만이 아니라 지금 내는 소리(커서가 가리키는 파일 프레임)도 같아야 한다. 출력 버스는 채널 DSP를
    // 지나므로 값으로 위치를 읽을 수 없어, 인스턴스가 들고 있는 묶음에서 읽는다.
    let now_frame = |id: u64| {
        let inst = rig.instance(id);
        frame_of(inst.stream_buffer[inst.cursor as usize])
    };
    let (a_now, b_now) = (now_frame(1), now_frame(2));
    eprintln!("위상: A {pa:.6}s B {pb:.6}s 차이 {diff_frames:.3}프레임, 지금 내는 파일 프레임 A {a_now} B {b_now}");
    assert_eq!(a_now as f64, (pa * FS as f64).round(), "A의 위치와 내는 소리가 다르다(시험 가정 확인)");
    assert!(diff_frames < 0.5, "끊겼다 돌아온 B의 위치가 A와 {diff_frames}프레임 다르다");
    assert_eq!(a_now, b_now, "끊겼다 돌아온 B가 내는 소리가 A와 다른 프레임이다");
    let _ = std::fs::remove_dir_all(&dir);
}
