use cpal::traits::{DeviceTrait, HostTrait};
fn main() {
    let host = cpal::default_host();
    let devices = host.output_devices().unwrap();
    for d in devices {
        if let Ok(name) = d.name() {
            let mut max_ch = 0;
            if let Ok(configs) = d.supported_output_configs() {
                for c in configs {
                    if c.channels() > max_ch {
                        max_ch = c.channels();
                    }
                }
            }
            println!("Device: {}, Max channels: {}", name, max_ch);
        }
    }
}
