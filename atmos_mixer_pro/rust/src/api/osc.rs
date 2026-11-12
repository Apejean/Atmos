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

/// OSC 시작 신호(`/awesome/status/boot`, 규격 §3.2)를 보낸다. 앱 시작 절차에서 이어 가기·테마 시작이 정해진 뒤
/// 한 번 부른다. 같은 실행에서 다시 불러도 한 번만 보낸다(그 안에서 1초 간격 3회).
/// `start_kind`: manual / logon / relaunch(모르는 값은 manual로 보고 로그). `app_version`: 앱 버전(pubspec).
pub fn api_osc_announce_boot(start_kind: String, app_version: String) {
    crate::osc::reply::announce_boot(&start_kind, &app_version);
}
