use cpal::Host;
use once_cell::sync::Lazy;

pub static AUDIO_HOST: Lazy<Host> = Lazy::new(|| cpal::default_host());
