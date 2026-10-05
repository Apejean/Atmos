//! 감시 대상 앱 프로세스. 감시가 띄운 앱(자식)과 감시보다 먼저 떠 있던 앱(넘겨받음)을 같은 방법으로 다룬다.
use crate::heartbeat::{read_pid, APP_LOCK, APP_PID};
use std::fs::{OpenOptions, TryLockError};
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// 감시 대상 앱.
pub enum Target {
    /// 감시가 띄운 앱.
    Child(Child),
    /// 감시보다 먼저 떠 있던 앱.
    Adopted(Adopted),
}

impl Target {
    /// 앱을 띄운다. 감시는 창 없이 돌므로 표준 입출력은 쓰지 않는다.
    pub fn spawn(app: &Path, args: &[String]) -> std::io::Result<Target> {
        // 작업 폴더를 앱 폴더로 바꾸므로 상대 경로는 먼저 절대 경로로 만든다(유닉스는 바뀐 작업 폴더에서 찾는다).
        let app = std::path::absolute(app)?;
        let mut command = Command::new(&app);
        command.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        if let Some(dir) = app.parent().filter(|d| !d.as_os_str().is_empty()) {
            command.current_dir(dir);
        }
        Ok(Target::Child(command.spawn()?))
    }

    pub fn pid(&self) -> u32 {
        match self {
            Target::Child(child) => child.id(),
            Target::Adopted(adopted) => adopted.pid,
        }
    }

    /// 끝났으면 Some(종료 코드). 종료 코드를 알 수 없으면(신호로 끝남, macOS에서 넘겨받은 앱) Some(None).
    pub fn try_exit(&mut self) -> Option<Option<i32>> {
        match self {
            Target::Child(child) => match child.try_wait() {
                Ok(Some(status)) => Some(status.code()),
                Ok(None) => None,
                Err(_) => Some(None),
            },
            Target::Adopted(adopted) => adopted.try_exit(),
        }
    }

    /// 강제 종료하고 끝날 때까지 기다린다.
    pub fn kill(&mut self) {
        match self {
            Target::Child(child) => {
                let _ = child.kill();
                let _ = child.wait();
            }
            Target::Adopted(adopted) => adopted.kill(),
        }
    }
}

/// 다른 프로세스가 [path] 잠금을 쥐고 있나. 앱은 떠 있는 동안 app.lock을 쥔다(rust core::app_signals).
/// 확인하려고 잠깐 잡았다 놓는다. 바로 그 순간 시작하는 앱은 중복으로 보고 끝날 수 있지만(종료 코드 3)
/// 감시가 그 종료를 보고 대상을 다시 정한다.
pub fn lock_held(path: &Path) -> bool {
    let Ok(file) = OpenOptions::new().read(true).write(true).open(path) else {
        return false;
    };
    matches!(file.try_lock(), Err(TryLockError::WouldBlock))
}

/// 떠 있는 앱을 넘겨받는다. app.lock이 잡혀 있고 app.pid의 프로세스가 살아 있어야 한다. 잠금을 함께 보므로
/// 앱이 죽은 뒤 그 PID를 다른 프로세스가 물려받았어도 넘겨받지 않는다(엉뚱한 프로세스를 죽이지 않게).
pub fn adopt_running_app(dir: &Path) -> Option<Target> {
    if !lock_held(&dir.join(APP_LOCK)) {
        return None;
    }
    let pid = read_pid(&dir.join(APP_PID))?;
    Adopted::open(pid).map(Target::Adopted)
}

/// 넘겨받은 앱. macOS는 PID로만 살아 있는지 본다(스펙 8절: PID 재사용을 완전히 막지 못함 — 확인용이라 받아들인다).
#[cfg(unix)]
pub struct Adopted {
    pid: u32,
}

#[cfg(unix)]
impl Adopted {
    fn open(pid: u32) -> Option<Adopted> {
        pid_alive(pid).then_some(Adopted { pid })
    }

    fn try_exit(&mut self) -> Option<Option<i32>> {
        if pid_alive(self.pid) {
            None
        } else {
            Some(None)
        }
    }

    fn kill(&mut self) {
        use std::time::{Duration, Instant};
        unsafe {
            libc::kill(self.pid as libc::pid_t, libc::SIGKILL);
        }
        let end = Instant::now() + Duration::from_secs(5);
        while pid_alive(self.pid) && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// 신호 0은 보내지 않고 대상이 있는지만 본다. 권한이 없어도(EPERM) 프로세스는 있다. PID 0은 프로세스 그룹
/// 전체를 뜻하므로 받지 않는다.
#[cfg(unix)]
fn pid_alive(pid: u32) -> bool {
    if pid == 0 || pid > i32::MAX as u32 {
        return false;
    }
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// 넘겨받은 앱. 프로세스 핸들을 쥐고 있어서, 그 프로세스가 끝나 PID가 재사용돼도 핸들은 원래 프로세스를 가리킨다.
#[cfg(windows)]
pub struct Adopted {
    pid: u32,
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl Adopted {
    fn open(pid: u32) -> Option<Adopted> {
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
        };
        if pid == 0 {
            return None;
        }
        let access = PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE;
        let handle = unsafe { OpenProcess(access, 0, pid) };
        if handle.is_null() {
            return None;
        }
        let mut adopted = Adopted { pid, handle };
        // 이미 끝났으면 넘겨받지 않는다(drop이 핸들을 닫는다).
        adopted.try_exit().is_none().then_some(adopted)
    }

    fn try_exit(&mut self) -> Option<Option<i32>> {
        use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
        use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
        if unsafe { WaitForSingleObject(self.handle, 0) } != WAIT_OBJECT_0 {
            return None;
        }
        let mut code: u32 = 0;
        let ok = unsafe { GetExitCodeProcess(self.handle, &mut code) } != 0;
        Some(ok.then_some(code as i32))
    }

    fn kill(&mut self) {
        use windows_sys::Win32::System::Threading::{TerminateProcess, WaitForSingleObject};
        unsafe {
            TerminateProcess(self.handle, 1);
            WaitForSingleObject(self.handle, 5000);
        }
    }
}

#[cfg(windows)]
impl Drop for Adopted {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}
