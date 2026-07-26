use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const DEBOUNCE_MS: u64 = 250;

pub struct OscDebouncer {
    last_triggers: Mutex<HashMap<String, Instant>>,
}

impl Default for OscDebouncer {
    fn default() -> Self {
        Self::new()
    }
}

impl OscDebouncer {
    pub fn new() -> Self {
        Self {
            last_triggers: Mutex::new(HashMap::new()),
        }
    }

    pub fn should_process(&self, address: &str) -> bool {
        // Fast-fail with try_lock to prevent blocking under UDP flooding
        let mut map = match self.last_triggers.try_lock() {
            Ok(guard) => guard,
            Err(_) => return false, // If lock is contended, drop the packet
        };
        let now = Instant::now();
        if let Some(&last_time) = map.get(address) {
            if now.duration_since(last_time) < Duration::from_millis(DEBOUNCE_MS) {
                return false; // Drop it
            }
        }
        map.insert(address.to_string(), now);
        true
    }
}
