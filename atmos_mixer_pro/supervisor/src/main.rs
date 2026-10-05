// 감시 프로그램은 창 없이 돈다(Windows에서 콘솔 창이 뜨지 않게).
#![windows_subsystem = "windows"]
//! 충돌·멈춤 뒤 Atmos Mixer Pro를 다시 띄우는 감시 프로그램. 인자는 `run::Config::parse` 참고.
use atmos_supervisor::logfile::log;
use atmos_supervisor::run::{default_dir, run, Config};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    match Config::parse(&args, &exe_dir) {
        Ok(cfg) => run(&cfg),
        Err(msg) => {
            log(&default_dir(), &format!("감시를 시작하지 못했다: {msg}"));
            std::process::exit(2);
        }
    }
}
