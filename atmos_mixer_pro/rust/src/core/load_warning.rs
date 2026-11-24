//! 오디오 처리 부하 경고(화면 배너). 엔진 점검 루프가 10초 구간마다 집계한 값으로 판정한다.
//!
//! - 구간에 예산 초과(처리 시간이 버퍼 길이를 넘음)나 늦은 콜백이 있으면 `LoadHigh`를 보낸다.
//!   예산 70%를 넘은 콜백만 있으면 경고하지 않는다(로그만, 소리는 아직 안 끊긴다).
//! - 경고 뒤 연속 [`CLEAR_AFTER_WINDOWS`] 구간(1분) 정상이면 `LoadNormal`을 보낸다.
//! - 상태 스트림 값 계약(Rust → Dart): `LoadHigh:<최대ms>/<예산ms>/<초과회>/<늦은회>/<처리채널>/<평균ms>/<원인>`,
//!   `LoadNormal`. ms 값은 소수 첫째 자리까지. 처리 채널은 엔진 시작 로그의 "M채널 처리"와 같은 값이다. 원인은
//!   [`LoadCause`]: `processing`(처리량이 무거움) / `stall`(처리량은 정상인데 콜백 하나가 멈추거나 늦음 — 다른
//!   프로그램이 오디오를 잠깐 막은 경우, 2026-11-24 부팅 직후 시작 프로그램이 뜰 때 관찰). 화면 문구와 로그 끝 안내가
//!   원인에 따라 다르다(사용자 결정 2026-11-24 "알림 문구도 바꿔").
//! - **엔진 상태가 아니라 부하 표시다.** 상태 스트림을 함께 쓰지만, "마지막 상태 문자열 = 엔진 상태"로 보는
//!   코드를 만들지 않는다. OSC `status/engine` 대응표와 하트비트 상태값에도 넣지 않는다.
//! - "오디오 처리 부하" 로그 줄(측정 스크립트가 정규식으로 읽는다)은 바꾸지 않는다.
//!
//! 오디오 콜백에서는 부르지 않는다(문자열 할당).

/// 경고를 푸는 데 필요한 연속 정상 구간 수(10초 × 6 = 1분).
pub const CLEAR_AFTER_WINDOWS: u32 = 6;
/// 예산 70%를 넘은 콜백이 한 구간에 이만큼 있으면 평균이 낮아도 처리량 원인으로 본다(가끔 무거운 리버브·자동 보정).
pub const HEAVY_FOR_PROCESSING: u32 = 5;
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
    /// 구간 평균 처리 시간.
    pub avg_us: u32,
    /// 예산 70%를 넘은 콜백 수.
    pub heavy: u32,
}

/// 부하 경고의 원인.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadCause {
    /// 처리량이 무거움: 평균이 예산의 절반 이상이거나 무거운 콜백이 [`HEAVY_FOR_PROCESSING`]번 이상.
    Processing,
    /// 처리량은 정상인데 콜백 하나가 멈추거나 늦음(다른 프로그램·드라이버가 오디오를 잠깐 막음).
    Stall,
}

impl LoadCause {
    fn code(self) -> &'static str {
        match self {
            LoadCause::Processing => "processing",
            LoadCause::Stall => "stall",
        }
    }
}

/// 구간 값으로 원인을 가른다. 평균 3ms대에 콜백 하나만 60ms가 걸리는 것은 채널·리버브를 줄여도 그대로다.
pub fn load_cause(w: &LoadWindow) -> LoadCause {
    if u64::from(w.avg_us) * 2 >= u64::from(w.budget_us) || w.heavy >= HEAVY_FOR_PROCESSING {
        LoadCause::Processing
    } else {
        LoadCause::Stall
    }
}

/// "오디오 처리 부하 높음" 로그 줄 끝에 붙는 안내(앞부분 숫자는 측정 스크립트가 읽으므로 바꾸지 않는다).
pub fn log_hint(cause: LoadCause) -> &'static str {
    match cause {
        LoadCause::Processing => " — 쓰지 않는 출력 채널을 끄거나 리버브를 줄이면 줄어든다",
        LoadCause::Stall => " — 처리량은 정상, 다른 프로그램이나 드라이버가 오디오를 잠깐 막았을 수 있다",
    }
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
        "{}{}/{}/{}/{}/{}/{}/{}",
        LOAD_HIGH_PREFIX,
        ms(w.max_us),
        ms(w.budget_us),
        w.overruns,
        w.late,
        w.processing_channels,
        ms(w.avg_us),
        load_cause(&w).code()
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
            avg_us: 3_000,
            heavy: 0,
        }
    }

    fn win_avg(max_us: u32, overruns: u32, late: u32, avg_us: u32, heavy: u32) -> LoadWindow {
        LoadWindow { max_us, overruns, late, avg_us, heavy, ..win(0, 0, 0) }
    }

    #[test]
    fn status_format() {
        assert_eq!(
            load_high_status(win_avg(24_000, 3, 1, 18_000, 50)),
            "LoadHigh:24.0/21.3/3/1/64/18.0/processing"
        );
        assert_eq!(load_high_status(win_avg(67_700, 1, 6, 3_500, 1)), "LoadHigh:67.7/21.3/1/6/64/3.5/stall");
    }

    /// 2026-11-24 이 PC 기록: 94채널을 처리할 때(평균 16.9ms, 70% 넘음 470회)는 처리량, 부팅 직후 콜백 하나만
    /// 멈춘 때(평균 3.5ms·최대 67.7ms)와 앱을 다시 띄운 직후 늦은 콜백만 있던 때(평균 3.2ms)는 잠깐 멈춤이다.
    #[test]
    fn cause_tells_heavy_processing_from_a_short_stall() {
        assert_eq!(load_cause(&win_avg(28_000, 1, 0, 16_900, 470)), LoadCause::Processing);
        assert_eq!(load_cause(&win_avg(67_700, 1, 6, 3_500, 1)), LoadCause::Stall);
        assert_eq!(load_cause(&win_avg(9_800, 0, 2, 3_200, 0)), LoadCause::Stall);
        // 평균은 낮아도 무거운 콜백이 여러 번이면 처리량(리버브·자동 보정이 가끔 무거운 경우).
        assert_eq!(load_cause(&win_avg(23_000, 2, 0, 6_000, HEAVY_FOR_PROCESSING)), LoadCause::Processing);
        assert_eq!(load_cause(&win_avg(23_000, 2, 0, 6_000, HEAVY_FOR_PROCESSING - 1)), LoadCause::Stall);
        // 평균이 예산의 절반이면 처리량.
        assert_eq!(load_cause(&win_avg(22_000, 1, 0, 10_667, 0)), LoadCause::Processing);
    }

    #[test]
    fn log_hint_matches_the_cause() {
        assert!(log_hint(LoadCause::Processing).contains("리버브"));
        assert!(log_hint(LoadCause::Stall).contains("다른 프로그램"));
        assert!(!log_hint(LoadCause::Stall).contains("리버브"));
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
