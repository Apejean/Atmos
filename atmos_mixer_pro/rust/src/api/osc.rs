use rosc::OscPacket;
use tokio::net::UdpSocket;

#[flutter_rust_bridge::frb(ignore)]
pub fn validate_osc_packet(sender_addr: &std::net::SocketAddr, whitelist: &[std::net::IpAddr], raw_bytes: &[u8]) -> bool {
    // 1. IP 화이트리스트 검사 (비어있으면 전체 허용으로 간주하거나, 기본적으로 허용 목록 확인)
    if !whitelist.is_empty() && !whitelist.contains(&sender_addr.ip()) {
        println!("⚠️ 허용되지 않은 IP 패킷 차단: {}", sender_addr.ip());
        return false;
    }
    // 2. OSC 매직 헤더 ("/" 로 시작하는지) 스키마 검사
    if raw_bytes.is_empty() || raw_bytes[0] != b'/' {
        return false;
    }
    true
}

#[flutter_rust_bridge::frb(ignore)]
pub fn create_high_capacity_osc_socket(port: u16) -> std::io::Result<std::net::UdpSocket> {
    use socket2::{Socket, Domain, Type, Protocol};
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    // OS UDP 수신 버퍼 2MB로 상향 확장!
    let _ = socket.set_recv_buffer_size(2_048_576); 
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
            eprintln!("Failed to create high capacity OSC socket on port {}: {}", port, e);
            return;
        }
    };
    
    let socket = match UdpSocket::from_std(std_socket) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to convert std socket to tokio socket: {}", e);
            return;
        }
    };
    println!("OSC server listening on 0.0.0.0:{} with 2MB buffer", port);

    let mut buf = [0u8; 65536];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((size, peer)) => {
                let whitelist: Vec<std::net::IpAddr> = Vec::new(); // TODO: Load from config
                if validate_osc_packet(&peer, &whitelist, &buf[..size]) {
                    if let Ok((_, packet)) = rosc::decoder::decode_udp(&buf[..size]) {
                        handle_osc_packet(packet);
                    }
                }
            }
            Err(e) => {
                eprintln!("OSC recv error: {}", e);
            }
        }
    }
}

fn handle_osc_packet(packet: OscPacket) {
    match packet {
        OscPacket::Message(msg) => {
            println!("OSC message received: {} {:?}", msg.addr, msg.args);
            // Implement mapping logic here to map OSC messages to AudioCommands
            // e.g. /play room_id track_id
        }
        OscPacket::Bundle(bundle) => {
            for packet in bundle.content {
                handle_osc_packet(packet);
            }
        }
    }
}
