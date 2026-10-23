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
