//! 재생 중인 인스턴스의 재생 위치를 오디오 스레드 밖으로 알리는 고정 크기 표.
//!
//! 오디오 스레드는 블록마다 원자 저장만 한다(할당·잠금 없음). 칸은 믹서 인스턴스 풀의 칸과 1:1이다.
//! 엔진 밖에 있어서 엔진이 재시작으로 사라져도 남는다 — 재시작 직전 위치를 여기서 읽는다
//! (core::restart_resume).
//!
//! 전제 두 가지: 칸마다 쓰는 쪽은 한 번에 하나다(지금 엔진의 오디오 스레드, 또는 그 엔진이 생기기
//! 전의 `clear_all`). 인스턴스 id는 고유하다 — 한 번 칸을 떠난 id가 다시 들어오지 않으므로 읽는 쪽은
//! id를 앞뒤로 확인하는 것만으로 짝이 섞이지 않는다.
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};

/// 믹서 인스턴스 풀 크기(mixer.rs `instances`)와 같다.
pub const SLOT_COUNT: usize = 4096;

struct Slot {
    /// 0이면 빈 칸.
    instance_id: AtomicU64,
    seconds_bits: AtomicU64,
}

pub struct CursorTable {
    slots: Vec<Slot>,
}

impl CursorTable {
    pub fn new(len: usize) -> Self {
        Self {
            slots: (0..len)
                .map(|_| Slot { instance_id: AtomicU64::new(0), seconds_bits: AtomicU64::new(0) })
                .collect(),
        }
    }

    /// 오디오 스레드: 칸 `idx`에 있는 인스턴스의 위치를 적는다. 칸 주인이 바뀌면 id를 먼저
    /// 비우고 위치를 쓴 뒤 새 id를 적는다 — 읽는 쪽이 옛 주인의 위치를 새 주인과 짝짓지 않게.
    pub fn publish(&self, idx: usize, instance_id: u64, seconds: f64) {
        let Some(slot) = self.slots.get(idx) else { return };
        if slot.instance_id.load(Ordering::Relaxed) != instance_id {
            slot.instance_id.store(0, Ordering::Release);
            slot.seconds_bits.store(seconds.to_bits(), Ordering::Release);
            slot.instance_id.store(instance_id, Ordering::Release);
        } else {
            slot.seconds_bits.store(seconds.to_bits(), Ordering::Release);
        }
    }

    /// 오디오 스레드: 끝난 인스턴스의 칸을 비운다.
    pub fn clear(&self, idx: usize) {
        if let Some(slot) = self.slots.get(idx) {
            slot.instance_id.store(0, Ordering::Release);
        }
    }

    /// 오디오 스레드 밖에서 부른다(새 믹서 생성, 재시작 스냅샷).
    pub fn clear_all(&self) {
        for slot in &self.slots {
            slot.instance_id.store(0, Ordering::Release);
        }
    }

    /// 오디오 스레드 밖에서 읽는다. 읽는 사이 칸 주인이 바뀌면 그 칸은 건너뛴다.
    /// 다시 읽는 건 읽는 쪽뿐이고 오디오 스레드는 기다리지 않는다.
    pub fn snapshot(&self) -> Vec<(u64, f64)> {
        let mut out = Vec::new();
        for slot in &self.slots {
            let id1 = slot.instance_id.load(Ordering::Acquire);
            if id1 == 0 {
                continue;
            }
            let seconds = f64::from_bits(slot.seconds_bits.load(Ordering::Acquire));
            if slot.instance_id.load(Ordering::Acquire) == id1 {
                out.push((id1, seconds));
            }
        }
        out
    }
}

/// 엔진이 쓰는 표. 오디오 스레드가 처음 쓰기 전에 `AudioMixer::new`에서 초기화된다(할당이 거기서 일어난다).
pub static CURSOR_TABLE: Lazy<CursorTable> = Lazy::new(|| CursorTable::new(SLOT_COUNT));
