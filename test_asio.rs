fn main() {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
        unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }
        let host = cpal::host_from_id(cpal::HostId::Asio).unwrap();
        println!("Host: {}", host.id().name());
        let devices = host.output_devices().unwrap();
        for d in devices {
            use cpal::traits::DeviceTrait;
            let d_name = d.name().unwrap();
            println!("Device Name: '{}'", d_name);
            println!("Device Name bytes: {:?}", d_name.as_bytes());
            let trimmed = d_name.trim_matches(char::from(0)).trim();
            println!("Trimmed: '{}'", trimmed);
        }
    }
}
