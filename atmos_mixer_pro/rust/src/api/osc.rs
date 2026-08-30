use rosc::{OscPacket, OscType, OscMessage};
use tokio::net::UdpSocket;
use std::sync::Arc;

#[flutter_rust_bridge::frb(ignore)]
pub fn validate_osc_packet(sender_addr: &std::net::SocketAddr, whitelist: &[std::net::IpAddr], raw_bytes: &[u8]) -> bool {
    if !whitelist.is_empty() && !whitelist.contains(&sender_addr.ip()) {
        println!("⚠️ 허용되지 않은 IP 패킷 차단: {}", sender_addr.ip());
        return false;
    }
    if raw_bytes.is_empty() || raw_bytes[0] != b'/' {
        return false;
    }
    true
}

#[flutter_rust_bridge::frb(ignore)]
pub fn create_high_capacity_osc_socket(port: u16) -> std::io::Result<std::net::UdpSocket> {
    use socket2::{Socket, Domain, Type, Protocol};
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    let _ = socket.set_recv_buffer_size(2_048_576); 
    let _ = socket.set_reuse_address(true);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    socket.bind(&addr.into())?;
    socket.set_nonblocking(true)?;
    Ok(socket.into())
}

// Removed secondary start_osc_server to prevent port 8000/8001 conflict
// The main OSC listener handles everything now.

fn handle_osc_packet(packet: OscPacket) {
    match packet {
        OscPacket::Message(msg) => {
            // println!("OSC message received: {} {:?}", msg.addr, msg.args);
            
            // Handle Head Tracking for HRTF
            if msg.addr == "/hrtf/tracking" && msg.args.len() == 3 {
                if let (Some(yaw), Some(pitch), Some(roll)) = (msg.args[0].clone().float(), msg.args[1].clone().float(), msg.args[2].clone().float()) {
                    let yaw_bits = yaw.to_bits();
                    let pitch_bits = pitch.to_bits();
                    let roll_bits = roll.to_bits();
                    
                    crate::core::state::GLOBAL_STATE.hrtf_yaw.store(yaw_bits, std::sync::atomic::Ordering::Relaxed);
                    crate::core::state::GLOBAL_STATE.hrtf_pitch.store(pitch_bits, std::sync::atomic::Ordering::Relaxed);
                    crate::core::state::GLOBAL_STATE.hrtf_roll.store(roll_bits, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
        OscPacket::Bundle(bundle) => {
            for packet in bundle.content {
                handle_osc_packet(packet);
            }
        }
    }
}
