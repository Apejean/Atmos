//! 다시 띄우기 전 대기 시간. 실패할 때마다 두 배로 늘리고 최대에서 멈춘다. 앱이 오래 잘 돌았으면 처음
//! 값으로 돌아간다. 시작하자마자 죽는 경우 장치를 계속 두드리지 않게 한다.
use std::time::Duration;

pub struct Backoff {
    min: Duration,
    max: Duration,
    stable: Duration,
    next: Duration,
}

impl Backoff {
    pub fn new(min: Duration, max: Duration, stable: Duration) -> Self {
        Self { min, max, stable, next: min }
    }

    /// 앱이 [uptime] 동안 돌고 끝났다(충돌·멈춤). 이번에 기다릴 시간을 돌려준다.
    pub fn next_delay(&mut self, uptime: Duration) -> Duration {
        if uptime >= self.stable {
            self.next = self.min;
        }
        let delay = self.next;
        self.next = (self.next * 2).min(self.max);
        delay
    }
}
