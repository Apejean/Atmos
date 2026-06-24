use cpal::traits::{DeviceTrait, HostTrait};
fn main() {
    let hosts = cpal::available_hosts();
    for host_id in hosts {
        println!("Host ID name: {}", host_id.name());
        if let Ok(host) = cpal::host_from_id(host_id) {
            if let Ok(devices) = host.output_devices() {
                for d in devices {
                    if let Ok(name) = d.name() {
                        println!("  Device name: {}", name);
                    }
                }
            }
        }
    }
}
