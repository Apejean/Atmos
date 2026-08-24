use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use lazy_static::lazy_static;

pub struct OscMetricsTracker {
    total_packets: AtomicU64,
    total_bytes: AtomicU64,
    valid_messages: AtomicU64,
    decode_errors: AtomicU64,

    // Window tracking for PPS / KBps
    window_start: Mutex<Instant>,
    window_packets: AtomicU64,
    window_bytes: AtomicU64,
    last_pps: AtomicU64,
    last_kbps: AtomicU64,

    // Latest message info
    last_address: Mutex<String>,
}

lazy_static! {
    pub static ref GLOBAL_OSC_METRICS: OscMetricsTracker = OscMetricsTracker::new();
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OscMetricsDto {
    pub total_packets: u64,
    pub total_bytes: u64,
    pub valid_messages: u64,
    pub decode_errors: u64,
    pub pps: u64,
    pub kbps: u64,
    pub last_address: String,
}

impl Default for OscMetricsTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl OscMetricsTracker {
    pub fn new() -> Self {
        Self {
            total_packets: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            valid_messages: AtomicU64::new(0),
            decode_errors: AtomicU64::new(0),

            window_start: Mutex::new(Instant::now()),
            window_packets: AtomicU64::new(0),
            window_bytes: AtomicU64::new(0),
            last_pps: AtomicU64::new(0),
            last_kbps: AtomicU64::new(0),

            last_address: Mutex::new("None".to_string()),
        }
    }

    pub fn record_packet(&self, bytes: usize, is_valid: bool, address: Option<&str>) {
        self.total_packets.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(bytes as u64, Ordering::Relaxed);

        if is_valid {
            self.valid_messages.fetch_add(1, Ordering::Relaxed);
        } else {
            self.decode_errors.fetch_add(1, Ordering::Relaxed);
        }

        self.window_packets.fetch_add(1, Ordering::Relaxed);
        self.window_bytes.fetch_add(bytes as u64, Ordering::Relaxed);

        if let Some(addr) = address {
            if let Ok(mut guard) = self.last_address.lock() {
                *guard = addr.to_string();
            }
        }

        self.update_window_if_needed();
    }

    fn update_window_if_needed(&self) {
        if let Ok(mut start) = self.window_start.try_lock() {
            let elapsed = start.elapsed();
            if elapsed.as_millis() >= 1000 {
                let packets = self.window_packets.swap(0, Ordering::Relaxed);
                let bytes = self.window_bytes.swap(0, Ordering::Relaxed);
                let secs = elapsed.as_secs_f64().max(0.001);

                self.last_pps.store((packets as f64 / secs) as u64, Ordering::Relaxed);
                self.last_kbps.store(((bytes as f64 / 1024.0) / secs) as u64, Ordering::Relaxed);

                *start = Instant::now();
            }
        }
    }

    pub fn get_metrics(&self) -> OscMetricsDto {
        self.update_window_if_needed();

        let address = self.last_address.lock().unwrap_or_else(|e| e.into_inner()).clone();

        OscMetricsDto {
            total_packets: self.total_packets.load(Ordering::Relaxed),
            total_bytes: self.total_bytes.load(Ordering::Relaxed),
            valid_messages: self.valid_messages.load(Ordering::Relaxed),
            decode_errors: self.decode_errors.load(Ordering::Relaxed),
            pps: self.last_pps.load(Ordering::Relaxed),
            kbps: self.last_kbps.load(Ordering::Relaxed),
            last_address: address,
        }
    }

    pub fn reset(&self) {
        self.total_packets.store(0, Ordering::Relaxed);
        self.total_bytes.store(0, Ordering::Relaxed);
        self.valid_messages.store(0, Ordering::Relaxed);
        self.decode_errors.store(0, Ordering::Relaxed);
        self.window_packets.store(0, Ordering::Relaxed);
        self.window_bytes.store(0, Ordering::Relaxed);
        self.last_pps.store(0, Ordering::Relaxed);
        self.last_kbps.store(0, Ordering::Relaxed);
        if let Ok(mut addr) = self.last_address.lock() {
            *addr = "None".to_string();
        }
    }
}
