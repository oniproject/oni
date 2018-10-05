use std::{
    io,
    net::{
        SocketAddr,
        ToSocketAddrs,
        UdpSocket,
    },
};

pub trait Socket {
    fn addr(&self) -> SocketAddr;
    fn send(&self, addr: SocketAddr, packet: &[u8]);
    fn recv(&self, packet: &mut [u8]) -> Option<(usize, SocketAddr)>;
}

pub struct Udp(UdpSocket);

impl Udp {
    pub fn new<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;
        Ok(Udp(socket))
    }
}

impl Socket for Udp {
    fn addr(&self) -> SocketAddr {
        self.0.local_addr().unwrap()
    }
    fn send(&self, addr: SocketAddr, packet: &[u8]) {
        self.0.send_to(packet, addr).unwrap();
    }
    fn recv(&self, packet: &mut [u8]) -> Option<(usize, SocketAddr)> {
        self.0.recv_from(packet).ok()
    }
}

#[test]
fn create() {
    let mut socket = Udp::new("127.0.0.1:0")
        .expect("couldn't bind to address");
    println!("addr: {:?}", socket.addr());
    let mut packet = [0u8; 8];
    assert_eq!(socket.recv(&mut packet[..]), None);
}
