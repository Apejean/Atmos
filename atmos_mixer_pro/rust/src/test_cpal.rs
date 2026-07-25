use cpal::traits::HostTrait;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref ASIO_HOST_CACHE: Mutex<Option<cpal::Host>> = Mutex::new(None);
}

fn test() {
    let host = cpal::host_from_id(cpal::HostId::Asio).unwrap();
    let mut guard = ASIO_HOST_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(host);
}
