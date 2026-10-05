//! 무인 운영 중 시스템 절전 방지. macOS에서는 이 프로세스가 사는 동안 `caffeinate -i -w <pid>`가 떠 있어야 한다.
//! Windows 경로(SetThreadExecutionState)는 여기서 돌릴 수 없어 CI 빌드와 Windows 실기로 확인한다.
#[cfg(target_os = "macos")]
#[test]
fn 절전_방지를_켜면_이_프로세스가_사는_동안_caffeinate가_붙는다() {
    use rust_lib_atmos_mixer_pro::core::keep_awake::prevent_system_sleep;

    let pattern = format!("caffeinate -i -w {}", std::process::id());
    let found = || {
        std::process::Command::new("pgrep")
            .args(["-f", &pattern])
            .output()
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false)
    };
    prevent_system_sleep();
    prevent_system_sleep(); // 두 번 불러도 하나만 뜬다
    let end = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !found() {
        assert!(std::time::Instant::now() < end, "절전 방지를 켰는데 `{pattern}`가 뜨지 않았다");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let count = std::process::Command::new("pgrep").args(["-f", &pattern]).output().unwrap();
    let n = String::from_utf8_lossy(&count.stdout).lines().count();
    assert_eq!(n, 1, "caffeinate가 {n}개 떴다(한 번만 걸어야 한다)");
}
