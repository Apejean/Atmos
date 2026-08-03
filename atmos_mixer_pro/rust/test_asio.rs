use cpal::traits::{DeviceTrait, HostTrait};
fn main() {
    let host = cpal::host_from_id(cpal::HostId::Asio).unwrap();
    let device = host.default_output_device().unwrap();
    // try to call show_control_panel
    if let Err(e) = cpal::traits::DeviceTrait::name(&device) {
        println!("{:?}", e);
    }
}
