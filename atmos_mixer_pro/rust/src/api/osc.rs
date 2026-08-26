use rosc::{OscPacket, OscType, OscMessage};
use tokio::net::UdpSocket;
use std::sync::Arc;

#[flutter_rust_bridge::frb(ignore)]
pub fn validate_osc_packet(sender_addr: &std::net::SocketAddr, whitelist: &[std::net::IpAddr], raw_bytes: &[u8]) -> bool {
    if !whitelist.is_empty() && !whitelist.contains(&sender_addr.ip()) {
        crate::log_print!("⚠️ 허용되지 않은 IP 패킷 차단: {}", sender_addr.ip());
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

#[flutter_rust_bridge::frb(ignore)]
pub async fn start_osc_server() {
    let port = 8000;
    let std_socket = match create_high_capacity_osc_socket(port) {
        Ok(s) => s,
        Err(e) => {
            crate::log_print!("Failed to create high capacity OSC socket on port {}: {}", port, e);
            return;
        }
    };
    
    let socket = match UdpSocket::from_std(std_socket) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            crate::log_print!("Failed to convert std socket to tokio socket: {}", e);
            return;
        }
    };
    crate::log_print!("OSC server listening on 0.0.0.0:{} with 2MB buffer", port);

    // Spawn a feedback task for OSC
    let tx_socket = socket.clone();
    tokio::spawn(async move {
        // Assume TouchDesigner or client is listening on port 9000
        let target_addr = "127.0.0.1:9000";
        loop {
            // Read telemetry from GLOBAL_STATE and send it
            let lufs = f32::from_bits(crate::core::state::GLOBAL_STATE.current_master_lufs.load(std::sync::atomic::Ordering::Relaxed));
            
            let lufs_msg = OscMessage {
                addr: "/atmos/telemetry/lufs".to_string(),
                args: vec![OscType::Float(lufs)],
            };
            let packet = OscPacket::Message(lufs_msg);
            let buf = rosc::encoder::encode(&packet).unwrap();
            
            let _ = tx_socket.send_to(&buf, target_addr).await;
            
            tokio::time::sleep(std::time::Duration::from_millis(33)).await; // ~30fps feedback
        }
    });

    let mut buf = [0u8; 65536];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((size, peer)) => {
                let whitelist: Vec<std::net::IpAddr> = {
                    let config_guard = crate::core::state::GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner());
                    if let Some(ref _config) = *config_guard {
                        vec![]
                    } else {
                        vec![]
                    }
                };
                if validate_osc_packet(&peer, &whitelist, &buf[..size]) {
                    if let Ok((_, packet)) = rosc::decoder::decode_udp(&buf[..size]) {
                        handle_osc_packet(packet);
                    }
                }
            }
            Err(e) => {
                crate::log_print!("OSC recv error: {}", e);
            }
        }
    }
}

fn handle_osc_packet(packet: OscPacket) {
    match packet {
        OscPacket::Message(msg) => {
            // crate::log_print!("OSC message received: {} {:?}", msg.addr, msg.args);
            
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
