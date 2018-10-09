/*
#![feature(try_blocks)]

use mio::net::UdpSocket;
use mio::{Events, Ready, Poll, PollOpt, Token};
use crossbeam_channel as channel;
use std::time::Duration;
use std::net::SocketAddr;
use std::io;
use std::sync::atomic::{Ordering, AtomicBool};


use oni_net::{
    crypto::{Key, keygen},
    generate_connect_token,
    TOKEN_DATA,
};


struct Connect {}
struct Accept {}

struct Transport {
    socket: UdpSocket,
    protocol_id: u64,
    is_client: bool,
}

impl Transport {
    pub fn bind(protocol_id: u64, addr: SocketAddr) -> io::Result<Self> {
        let socket = UdpSocket::bind(&addr)?;
        Ok(Self {
            protocol_id,
            socket,
            // ....
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.socket.local_addr().unwrap()
    }

    pub fn connect(self, addr: SocketAddr) -> io::Result<Connect> {
        self.socket.connect(addr)?;
        unimplemented!()
    }

    pub fn incoming(self, private_key: Key) -> Accept {
        unimplemented!()
    }
}


enum Error {
    NotConnected,
    Closed,
    IsFull,
    IsEmpty,
}

const MAX_PAYLOAD: usize = 1100;

type Packet = (usize, [u8; MAX_PAYLOAD]);
use slotmap::Key as Slot;

struct Stream {
    sender: channel::Sender<(Slot, usize, [u8; MAX_PAYLOAD])>,
    receiver: channel::Receiver<Packet>,

    key: Slot,
    closed: AtomicBool,
}

impl Stream {
    pub fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
    }
    pub fn send(&self, buf: &[u8]) -> Result<usize, Error> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(Error::Closed);
        }
        if self.sender.is_full() {
            return Err(Error::IsFull);
        }

        let mut buffer: [u8; MAX_PAYLOAD] = unsafe {
            std::mem::uninitialized()
        };

        let len = buf.len().min(buffer.len());
        &mut buffer[..len].copy_from_slice(buf);

        self.sender.send((self.key, len, buffer));

        Ok(len)
    }
    pub fn recv(&self, buf: &mut [u8]) -> Result<usize, Error> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(Error::Closed);
        }
        match self.receiver.recv() {
            Some((len, buffer)) => {
                let len = len.min(buf.len());
                buf.copy_from_slice(&buffer[..len]);
                Ok(len)
            }
            None => Err(Error::IsEmpty),
        }
    }
}

#[test]
fn xx() {
    //use std::net::TcpListener;
    //use tungstenite::server::accept;

    /*
    pub enum Poll<T> {
        Ready(T),
        Pending,
    }
    */
    const PROTOCOL: u64 = 0x1122334455667788;
    const CLIENT_ID: u64 = 666;

    const CONNECT_TOKEN_EXPIRY: u32 = 30;
    const CONNECT_TOKEN_TIMEOUT: u32 = 5;

    let private_key = keygen();
    let connect_token = generate_connect_token(
        [0u8; TOKEN_DATA], [0u8; TOKEN_DATA],
        CONNECT_TOKEN_EXPIRY, CONNECT_TOKEN_TIMEOUT,
        CLIENT_ID, PROTOCOL, &private_key,
    ).unwrap();

    let connect_token = parse_connect_token(connect_token).unwrap();

    let server_addr = "[::1]:40000".parse().unwrap();
    let client_addr = "::".parse().unwrap();

    let server = Transport::bind(PROTOCOL, server_addr).unwrap();
    let client = Transport::bind(PROTOCOL, client_addr).unwrap();
    let client = client.connect(connect_token, server.addr()).unwrap();

    for stream in server.incoming(private_key) {
        std::thread::spawn(move || {
            let mut buffer = [0; MAX_PAYLOAD];
            loop {
                let packet = match stream.read(msg) {
                    Ok(len) => &buffer[..len],
                    Err(NoMessage) => break,
                    Err(err) => break,
                };
            }
        });
    }
}

// SENDER -> sends a message.
// ECHOER -> listens and prints the message received.

#[test]
fn mio_example() {
    const SERVER: Token = Token(0);
    const ECHO: Token = Token(1);

    // This operation will fail if the address is in use,
    // so we select different ports for each socket.
    let addr = "127.0.0.1:40000".parse().unwrap();
    let socket = UdpSocket::bind(&addr).unwrap();

    let echo_addr = "127.0.0.1:0".parse().unwrap();
    let echo = UdpSocket::bind(&echo_addr).unwrap();
    let echo_addr = socket.local_addr().unwrap();
    echo.connect(echo_addr).unwrap();

    let poll = Poll::new().unwrap();

    let rw = Ready::readable() | Ready::writable();
    poll.register(&socket, SERVER, rw, PollOpt::edge()).unwrap();
    poll.register(&echo, ECHO, rw, PollOpt::edge()).unwrap();

    let read_timeout = Some(Duration::from_millis(100));

    let msg_to_send = [9; 9];
    let mut buffer = [0; 9];

    let mut ebuf = [0; 9];

    let mut events = Events::with_capacity(128);
    loop {
        poll.poll(&mut events, read_timeout).unwrap();

        for event in events.iter() {
            let err: std::io::Result<_> = try {
                match event.token() {
                    SERVER => {
                        if event.readiness().is_writable() {
                            println!("is_w");
                            let n = socket.send_to(&msg_to_send, &echo_addr)?;
                            assert_eq!(n, 9);
                            println!("sent {:?} -> {:?} bytes", msg_to_send, n);
                        }
                        if event.readiness().is_readable() {
                            println!("is_r");
                            let n = socket.recv_from(&mut buffer)?;
                            println!("echo {:?} -> {:?}", buffer, n);
                            buffer = [0; 9];
                        }
                    }
                    ECHO => {
                        if event.readiness().is_readable() {
                            let n = echo.recv(&mut ebuf)?;
                            println!("echo {:?}", &ebuf[..n]);
                            assert_eq!(echo.send(&ebuf[..n])?, 9);
                        }
                    }
                    _ => unreachable!()
                }
            };

            if let Err(err) = err {
                println!("err: {:?}", err);
            }
        }
    }
}


/*


const TEST_PROTOCOL_ID: u64 = 0x1122334455667788;

struct Scallback;

impl server::Callback for Scallback  {
    fn connect(&mut self, slot: Slot) {
        println!("connect[{:?}]", slot);
    }
    fn disconnect(&mut self, slot: Slot) {
        println!("disconnect[{:?}]", slot);
    }
    fn receive(&mut self, slot: Slot, payload: &[u8]) {
        println!("receive[{:?}]: {:?}", slot, payload);
    }
}

/*
fn server(addr: SocketAddr) {
    println!("[server]");

    let mut quit = false;

    let private_key = keygen();
    let time = 0.0;
    let delta_time = ;

    let server = Server::new(TEST_PROTOCOL_ID, private_key: Key, socket);
    //pub fn new(protocol_id: u64, pkey: Key, callback: C, socket: S) -> Self {


    while !quit {
        server.update();

        if server.client_connected(0) {
            server.send(0, packet_data, NETCODE_MAX_PACKET_SIZE);
        }

        for client in server.clients() {
            while let Some(packet) = client.recv() {
                println!("recv packet from {}: {:?}", client.addr(), packet);
            }
        }

        sleep(delta_time);
    }

    println!();
    println!("shutting down");
}
*/
*/
*/

use std::thread::sleep;
use std::time::Duration;

use oni_net::{
    crypto::keygen,
    token::generate_connect_token,
    token::{Public, USER},
    client::{self, Client, Event},
    server::{Server, MAX_PAYLOAD},
};

#[test]
#[ignore]
fn client_server() {
    const CONNECT_TOKEN_EXPIRY: u32 = 30;
    const CONNECT_TOKEN_TIMEOUT: u32 = 5;
    const PROTOCOL_ID: u64 =  0x1122334455667788;
    const DELTA_TIME: Duration = Duration::from_millis(1000 / 60);

    let private_key = keygen();

    println!("[client/server]");

    let client_id = 1345643;
    let connect_token = Public::generate(
        [0u8; 640],
        CONNECT_TOKEN_EXPIRY,
        CONNECT_TOKEN_TIMEOUT,
        client_id,
        PROTOCOL_ID,
        &private_key,
    );

    let mut client = Client::new(PROTOCOL_ID, connect_token, "[::1]:0".parse().unwrap()).unwrap();
    let mut server = Server::new(PROTOCOL_ID, private_key, "[::1]:40000".parse().unwrap()).unwrap();

    println!("client id is {}", client_id);

    let mut server_num_packets_received = 0;
    let mut client_num_packets_received = 0;

    let mut ref_packet = [0u8; MAX_PAYLOAD];
    for (i, v) in ref_packet.iter_mut().enumerate() {
        *v = (i & 0xFF) as u8;
    }

    let ref_packet = &ref_packet[..];

    let mut connected = Vec::new();

    let mut buf = [0u8; MAX_PAYLOAD];
    loop {
        client.update(|event| match event {
            Event::Connected => println!("client connected"),
            Event::Disconnected(err) => {
                println!("client disconnected: {:?}", err);
                return;
            }
            Event::Packet(payload) => {
                assert_eq!(payload, ref_packet, "client packet");
                client_num_packets_received += 1;
            }
        });

        if client.state() == client::State::Connected {
            client.send(ref_packet);
        }

        /* XXX
        if client.state().is_err()  {
            println!("client error state: {:?}", client.state());
            break;
        }
        */

        server.update(|c, user| {
            println!("connected {}:{:?} with data {:?}", c.id(), c.addr(), &user[..]);
            connected.push(c);
        });

        if let Some(client) = connected.get(0) {
            let _ = client.send(ref_packet);

            while let Ok(len) = client.recv(&mut buf) {
                if len == 0 { break; }
                let payload = &buf[..len as usize];
                assert_eq!(payload, ref_packet, "server packet");
                server_num_packets_received += 1;
            }
        }

        if client_num_packets_received >= 10 && server_num_packets_received >= 10 {
            if let Some(client) = connected.get(0) {
                println!("client and server successfully exchanged packets");
                client.close();
            }
        }

        sleep(DELTA_TIME);
    }

    println!("shutting down");
}
