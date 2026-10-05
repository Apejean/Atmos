//! 감시 판단 규칙: 하트비트 형식, 기록 회전, 재실행 대기, 멈춤 판단. 프로세스를 띄우지 않는다.
use atmos_supervisor::backoff::Backoff;
use atmos_supervisor::heartbeat::{atomic_write, read_heartbeat, read_pid, Heartbeat, HEARTBEAT};
use atmos_supervisor::logfile::{append_line, SUPERVISOR_LOG};
use atmos_supervisor::run::Config;
use atmos_supervisor::watch::{Verdict, Watch};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("atmos_sup_rules_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> =
        std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

#[test]
fn 하트비트는_다섯_키가_모두_있어야_읽는다() {
    let hb = Heartbeat {
        pid: 42,
        ui_ms: 1_700_000_000_123,
        audio_cb_ms: 1_700_000_000_100,
        engine_active: true,
        silent_warnings: 2,
    };
    assert_eq!(Heartbeat::parse(&hb.render()), Some(hb.clone()));
    assert_eq!(
        Heartbeat::parse("silent_warnings=0\nextra=x\nengine_active=0\naudio_cb_ms=0\nui_ms=5\npid=7\n"),
        Some(Heartbeat { pid: 7, ui_ms: 5, audio_cb_ms: 0, engine_active: false, silent_warnings: 0 }),
        "순서가 달라도, 모르는 키가 섞여도 읽는다"
    );
    assert_eq!(Heartbeat::parse("pid=7\nui_ms=5\naudio_cb_ms=0\nengine_active=1\n"), None, "키가 빠짐");
    assert_eq!(
        Heartbeat::parse("pid=7\nui_ms=5\naudio_cb_ms=0\nengine_active=1\nsilent_warnings="),
        None,
        "값이 잘림"
    );
    assert_eq!(
        Heartbeat::parse("pid=7\nui_ms=5\naudio_cb_ms=0\nengine_active=yes\nsilent_warnings=0\n"),
        None,
        "engine_active는 0/1"
    );
    assert_eq!(Heartbeat::parse(""), None);

    let dir = fresh_dir("hb");
    assert_eq!(read_heartbeat(&dir), None, "파일이 없으면 None");
    atomic_write(&dir.join(HEARTBEAT), &hb.render()).unwrap();
    assert_eq!(read_heartbeat(&dir), Some(hb));
    assert!(!dir.join(format!("{HEARTBEAT}.tmp")).exists(), "임시 파일이 남았다");

    std::fs::write(dir.join("a"), "123\n").unwrap();
    std::fs::write(dir.join("b"), "pid=456\n").unwrap();
    std::fs::write(dir.join("c"), "pid=\n").unwrap();
    assert_eq!(read_pid(&dir.join("a")), Some(123), "app.pid 형식");
    assert_eq!(read_pid(&dir.join("b")), Some(456), "clean_exit 형식");
    assert_eq!(read_pid(&dir.join("c")), None);
    assert_eq!(read_pid(&dir.join("none")), None);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 감시_기록은_크기를_넘으면_밀고_정한_개수만_남긴다() {
    let dir = fresh_dir("log");
    // 한 줄 10바이트("line-0000" + 줄바꿈), 파일당 30바이트를 넘으면 민다, 밀린 파일은 2개까지
    for i in 0..20 {
        append_line(&dir, SUPERVISOR_LOG, &format!("line-{i:04}"), 30, 2).unwrap();
    }
    assert_eq!(
        names(&dir),
        vec![SUPERVISOR_LOG.to_string(), format!("{SUPERVISOR_LOG}.1"), format!("{SUPERVISOR_LOG}.2")]
    );
    let current = std::fs::read_to_string(dir.join(SUPERVISOR_LOG)).unwrap();
    assert!(current.ends_with("line-0019\n"), "지금 파일 끝: {current:?}");
    let older = std::fs::read_to_string(dir.join(format!("{SUPERVISOR_LOG}.1"))).unwrap();
    assert!(older.contains("line-0015"), ".1 내용: {older:?}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 지금_파일을_옮기지_못하면_밀지_않고_그_줄은_지금_파일에_남긴다() {
    let dir = fresh_dir("log_stuck");
    std::fs::write(dir.join(format!("{SUPERVISOR_LOG}.1")), "old\n").unwrap();
    // 옮길 자리에 비어 있지 않은 폴더가 있어 지금 파일의 이름 바꾸기가 실패한다.
    // Windows에서 다른 프로그램이 기록 파일을 쥐고 있을 때와 같다.
    std::fs::create_dir_all(dir.join(format!("{SUPERVISOR_LOG}.rotating")).join("x")).unwrap();
    for i in 0..10 {
        append_line(&dir, SUPERVISOR_LOG, &format!("line-{i:04}"), 30, 2).unwrap();
    }
    let current = std::fs::read_to_string(dir.join(SUPERVISOR_LOG)).unwrap();
    assert_eq!(current.lines().count(), 10, "밀지 못해도 줄을 버리면 안 된다: {current:?}");
    assert_eq!(
        std::fs::read_to_string(dir.join(format!("{SUPERVISOR_LOG}.1"))).unwrap(),
        "old\n",
        "밀지 못했는데 오래된 기록이 바뀌었다"
    );
    assert!(!dir.join(format!("{SUPERVISOR_LOG}.2")).exists(), "밀지 못했는데 오래된 기록을 옮겼다");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn 대기_시간은_두_배씩_늘고_최대에서_멈추며_오래_잘_돌면_처음으로_돌아간다() {
    let s = Duration::from_secs;
    let mut b = Backoff::new(s(2), s(60), s(300));
    let delays: Vec<u64> = (0..7).map(|_| b.next_delay(s(10)).as_secs()).collect();
    assert_eq!(delays, vec![2, 4, 8, 16, 32, 60, 60]);
    assert_eq!(b.next_delay(s(300)).as_secs(), 2, "5분 이상 잘 돌았으면 처음 값");
    assert_eq!(b.next_delay(s(299)).as_secs(), 4, "5분이 안 되면 이어서 늘어난다");
}

fn hb(ui: u64, audio: u64, engine: bool) -> Heartbeat {
    Heartbeat { pid: 1, ui_ms: ui, audio_cb_ms: audio, engine_active: engine, silent_warnings: 0 }
}

const S: fn(u64) -> Duration = Duration::from_secs;

#[test]
fn ui_하트비트가_30초_넘게_안_바뀌면_멈춤이고_바뀌면_괜찮다() {
    let t0 = Instant::now();
    let mut w = Watch::new(t0, S(90), S(30), S(120));
    let beat = hb(100, 0, false);
    // 1초마다 본다(간격이 5초를 넘지 않게). ui 100을 처음 본 것은 1초.
    for sec in 1..=31 {
        assert_eq!(w.observe(t0 + S(sec), Some(&beat)).verdict, Verdict::Ok, "{sec}초");
    }
    assert_eq!(w.observe(t0 + S(32), Some(&beat)).verdict, Verdict::UiHang);
    assert_eq!(w.observe(t0 + S(33), Some(&hb(101, 0, false))).verdict, Verdict::Ok, "값이 바뀌면 괜찮다");
}

#[test]
fn 하트비트가_한_번도_안_오면_기한_뒤_시작_중_멈춤이다() {
    let t0 = Instant::now();
    let mut w = Watch::new(t0, S(90), S(30), S(120));
    for sec in 1..=90 {
        assert_eq!(w.observe(t0 + S(sec), None).verdict, Verdict::Ok, "{sec}초");
    }
    assert_eq!(w.observe(t0 + S(91), None).verdict, Verdict::StartupHang);
}

#[test]
fn 엔진이_켜져_있는데_콜백_시각이_120초_넘게_멈추면_오디오_멈춤이다() {
    let t0 = Instant::now();
    let mut w = Watch::new(t0, S(90), S(30), S(120));
    // UI는 매초 바뀌고 콜백 시각(500)은 그대로. 500을 처음 본 것은 1초.
    for sec in 1..=121 {
        let seen = w.observe(t0 + S(sec), Some(&hb(sec, 500, true)));
        assert_eq!(seen.verdict, Verdict::Ok, "{sec}초");
        assert!(!seen.audio_progressed, "{sec}초: 콜백 시각이 안 바뀌었는데 돈다고 했다");
    }
    assert_eq!(w.observe(t0 + S(122), Some(&hb(122, 500, true))).verdict, Verdict::AudioStall);
    let seen = w.observe(t0 + S(123), Some(&hb(123, 501, true)));
    assert_eq!(seen.verdict, Verdict::Ok);
    assert!(seen.audio_progressed, "콜백 시각이 바뀌면 콜백이 돈다고 알려야 한다");

    // 엔진이 꺼져 있으면 오디오는 판단하지 않는다(장치 없음, 엔진 재기동 중)
    let mut w = Watch::new(t0, S(90), S(30), S(120));
    for sec in 1..=200 {
        assert_eq!(w.observe(t0 + S(sec), Some(&hb(sec, 500, false))).verdict, Verdict::Ok, "{sec}초");
    }
    // 엔진이 다시 켜지면 그때부터 잰다
    for sec in 201..=321 {
        assert_eq!(w.observe(t0 + S(sec), Some(&hb(sec, 500, true))).verdict, Verdict::Ok, "{sec}초");
    }
    assert_eq!(w.observe(t0 + S(322), Some(&hb(322, 500, true))).verdict, Verdict::AudioStall);
}

#[test]
fn 확인_간격이_벌어지면_절전에서_깨어난_것으로_보고_멈춤_시간을_다시_잰다() {
    let t0 = Instant::now();
    let mut w = Watch::new(t0, S(90), S(30), S(120));
    let beat = hb(100, 7, true);
    assert_eq!(w.observe(t0 + S(1), Some(&beat)).verdict, Verdict::Ok);
    // 10분 동안 확인이 없었다(사람이 절전시킴). 깨어난 직후 멀쩡한 앱을 죽이면 안 된다.
    let seen = w.observe(t0 + S(601), Some(&beat));
    assert!(seen.gap);
    assert_eq!(seen.verdict, Verdict::Ok);
    // 그 뒤로도 값이 안 바뀌면 다시 잰 시점부터 30초 뒤 멈춤
    for sec in 602..=631 {
        assert_eq!(w.observe(t0 + S(sec), Some(&beat)).verdict, Verdict::Ok, "{sec}초");
    }
    assert_eq!(w.observe(t0 + S(632), Some(&beat)).verdict, Verdict::UiHang);

    // 첫 하트비트를 기다리는 중에 벌어져도 기한을 다시 잰다
    let mut w = Watch::new(t0, S(90), S(30), S(120));
    assert_eq!(w.observe(t0 + S(1), None).verdict, Verdict::Ok);
    let seen = w.observe(t0 + S(500), None);
    assert!(seen.gap);
    assert_eq!(seen.verdict, Verdict::Ok);
}

#[test]
fn 인자가_없으면_스펙_기본값이고_판단_시간과_폴더를_바꿀_수_있다() {
    let exe_dir = Path::new("/opt/atmos");
    let cfg = Config::parse(&[], exe_dir).unwrap();
    assert_eq!((cfg.hang, cfg.first_heartbeat, cfg.audio_stall), (S(30), S(90), S(120)));
    assert_eq!(cfg.tick, S(1));
    assert_eq!((cfg.backoff_min, cfg.backoff_max, cfg.stable), (S(2), S(60), S(300)));
    assert!(!cfg.logon);
    assert_eq!(cfg.app.parent(), Some(exe_dir), "기본 앱은 감시 프로그램 옆에 있다");
    assert_eq!(cfg.dir, std::env::temp_dir().join("atmos_mixer_pro_logs"), "앱 로그 폴더와 같아야 한다");

    let args: Vec<String> = [
        "--logon", "--app", "/x/app", "--dir", "/tmp/d", "--hang-secs", "2.5", "--first-heartbeat-secs", "3",
        "--audio-stall-secs", "4", "--tick-ms", "100", "--backoff-min-ms", "200",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let cfg = Config::parse(&args, exe_dir).unwrap();
    assert!(cfg.logon);
    assert_eq!(cfg.app, PathBuf::from("/x/app"));
    assert_eq!(cfg.dir, PathBuf::from("/tmp/d"));
    assert_eq!(cfg.hang, Duration::from_millis(2500));
    assert_eq!((cfg.first_heartbeat, cfg.audio_stall), (S(3), S(4)));
    assert_eq!((cfg.tick, cfg.backoff_min), (Duration::from_millis(100), Duration::from_millis(200)));

    let bad = |list: &[&str]| {
        Config::parse(&list.iter().map(|s| s.to_string()).collect::<Vec<_>>(), exe_dir).is_err()
    };
    assert!(bad(&["--hang-secs"]), "값이 빠짐");
    assert!(bad(&["--hang-secs", "0"]), "0초");
    assert!(bad(&["--hang-secs", "abc"]));
    assert!(bad(&["--tick-ms", "-5"]));
    assert!(bad(&["--what"]), "모르는 인자");
}
