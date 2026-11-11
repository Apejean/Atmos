//! 오디오 처리 부하 경고(화면 배너). 엔진 점검 루프가 10초 구간마다 집계한 값으로 판정한다.
//!
//! - 구간에 예산 초과(처리 시간이 버퍼 길이를 넘음)나 늦은 콜백이 있으면 `LoadHigh`를 보낸다.
//!   예산 70%를 넘은 콜백만 있으면 경고하지 않는다(로그만, 소리는 아직 안 끊긴다).
//! - 경고 뒤 연속 [`CLEAR_AFTER_WINDOWS`] 구간(1분) 정상이면 `LoadNormal`을 보낸다.
//! - 상태 스트림 값 계약(Rust → Dart): `LoadHigh:<최대ms>/<예산ms>/<초과회>/<늦은회>/<처리채널>`, `LoadNormal`.
//!   ms 값은 소수 첫째 자리까지. 처리 채널은 엔진 시작 로그의 "M채널 처리"와 같은 값이다.
//! - **엔진 상태가 아니라 부하 표시다.** 상태 스트림을 함께 쓰지만, "마지막 상태 문자열 = 엔진 상태"로 보는
//!   코드를 만들지 않는다. OSC `status/engine` 대응표와 하트비트 상태값에도 넣지 않는다.
//! - "오디오 처리 부하" 로그 줄(측정 스크립트가 정규식으로 읽는다)은 바꾸지 않는다.
//!
//! 오디오 콜백에서는 부르지 않는다(문자열 할당).

/// 경고를 푸는 데 필요한 연속 정상 구간 수(10초 × 6 = 1분).
pub const CLEAR_AFTER_WINDOWS: u32 = 6;
pub const LOAD_HIGH_PREFIX: &str = "LoadHigh:";
pub const LOAD_NORMAL: &str = "LoadNormal";

/// 10초 구간 하나의 집계(µs·횟수).
#[derive(Debug, Clone, Copy, Default)]
pub struct LoadWindow {
    pub max_us: u32,
    pub budget_us: u32,
    pub overruns: u32,
    pub late: u32,
    pub processing_channels: u32,
}

/// 경고 상태. 엔진 점검 루프가 하나 들고 구간마다 [`LoadWarningState::update`]를 부른다.
#[derive(Debug, Default)]
pub struct LoadWarningState {
    warning: bool,
    normal_streak: u32,
}

impl LoadWarningState {
    /// 이 구간 뒤 보낼 상태 문자열(바뀔 때만 `Some`). 경고가 이어지는 동안은 값이 바뀌어도 다시 보낸다.
    pub fn update(&mut self, w: LoadWindow) -> Option<String> {
        if w.budget_us == 0 {
            return None; // 엔진이 아직 예산을 모른다
        }
        if w.overruns > 0 || w.late > 0 {
            self.warning = true;
            self.normal_streak = 0;
            return Some(load_high_status(w));
        }
        if !self.warning {
            return None;
        }
        self.normal_streak += 1;
        if self.normal_streak >= CLEAR_AFTER_WINDOWS {
            self.warning = false;
            self.normal_streak = 0;
            return Some(LOAD_NORMAL.to_string());
        }
        None
    }
}

fn ms(us: u32) -> String {
    format!("{:.1}", f64::from(us) / 1000.0)
}

pub fn load_high_status(w: LoadWindow) -> String {
    format!(
        "{}{}/{}/{}/{}/{}",
        LOAD_HIGH_PREFIX,
        ms(w.max_us),
        ms(w.budget_us),
        w.overruns,
        w.late,
        w.processing_channels
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(max_ms: u32, overruns: u32, late: u32) -> LoadWindow {
        LoadWindow {
            max_us: max_ms * 1000,
            budget_us: 21_333,
            overruns,
            late,
            processing_channels: 64,
        }
    }

    #[test]
    fn status_format() {
        assert_eq!(load_high_status(win(24, 3, 1)), "LoadHigh:24.0/21.3/3/1/64");
    }

    #[test]
    fn quiet_when_normal_or_only_heavy() {
        let mut s = LoadWarningState::default();
        for _ in 0..20 {
            assert_eq!(s.update(win(16, 0, 0)), None);
        }
    }

    #[test]
    fn warns_on_overrun_or_late_and_repeats_while_high() {
        let mut s = LoadWarningState::default();
        assert!(s.update(win(24, 1, 0)).unwrap().starts_with(LOAD_HIGH_PREFIX));
        assert!(s.update(win(10, 0, 2)).unwrap().starts_with(LOAD_HIGH_PREFIX));
    }

    #[test]
    fn clears_after_one_minute_of_normal() {
        let mut s = LoadWarningState::default();
        s.update(win(24, 1, 0));
        for _ in 0..(CLEAR_AFTER_WINDOWS - 1) {
            assert_eq!(s.update(win(10, 0, 0)), None);
        }
        assert_eq!(s.update(win(10, 0, 0)).as_deref(), Some(LOAD_NORMAL));
        assert_eq!(s.update(win(10, 0, 0)), None);
    }

    #[test]
    fn new_overrun_resets_normal_streak() {
        let mut s = LoadWarningState::default();
        s.update(win(24, 1, 0));
        for _ in 0..(CLEAR_AFTER_WINDOWS - 1) {
            s.update(win(10, 0, 0));
        }
        assert!(s.update(win(24, 1, 0)).is_some());
        for _ in 0..(CLEAR_AFTER_WINDOWS - 1) {
            assert_eq!(s.update(win(10, 0, 0)), None);
        }
    }

    #[test]
    fn unknown_budget_is_ignored() {
        let mut s = LoadWarningState::default();
        let mut w = win(24, 5, 5);
        w.budget_us = 0;
        assert_eq!(s.update(w), None);
    }
}
