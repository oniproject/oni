use std::time::Duration;

use oni::{
    protocol::MAX_PAYLOAD,
    token::{PublicToken, USER},
    crypto::keygen,
    Server,
    Client, State,
    ServerList,
};

#[test]
fn client_server() {
    const CONNECT_TOKEN_EXPIRY: u32 = 30;
    const CONNECT_TOKEN_TIMEOUT: u32 = 5;
    const PROTOCOL_ID: u64 =  0x1122334455667788;
    const DELTA_TIME: Duration = Duration::from_millis(1000 / 60);

    println!("[client/server]");

    let (connect_token, mut server) = {
        use std::io::Write;

        let private_key = keygen();
        let client_id = 1345643;
        let server_addr = "[::1]:40000".parse().unwrap();

        let mut server_list = ServerList::new();
        server_list.push(server_addr).unwrap();

        let data = server_list.serialize().unwrap();
        let mut user = [0u8; USER];
        (&mut user[..]).write(b"some user data\0").unwrap();

        let connect_token = PublicToken::generate(
            data, user,
            CONNECT_TOKEN_EXPIRY,
            CONNECT_TOKEN_TIMEOUT,
            client_id,
            PROTOCOL_ID,
            &private_key,
        );

        (connect_token, Server::new(PROTOCOL_ID, private_key, server_addr).unwrap())
    };

    let mut client = {
        let client_addr = "[::1]:0".parse().unwrap();
        let mut client = Client::new(PROTOCOL_ID, &connect_token, client_addr).unwrap();
        client.connect(server.local_addr()).unwrap();
        client
    };

    let mut server_num_packets_received = 0;
    let mut client_num_packets_received = 0;

    let mut ref_packet = [0u8; MAX_PAYLOAD];
    for (i, v) in ref_packet.iter_mut().enumerate() {
        *v = (i & 0xFF) as u8;
    }

    let ref_packet = &ref_packet[..];

    let mut connected = Vec::new();

    let mut buf = [0u8; MAX_PAYLOAD];
    println!("[start]");
    loop {
        std::thread::sleep(DELTA_TIME);
        println!(" - - - - - - client recv: {}, server recv: {}",
                 client_num_packets_received, server_num_packets_received);

        client.update();
        match client.state() {
            State::Connecting(v) => println!("client {:?}", v),
            State::Connected => {
                let mut packet = [0u8; MAX_PAYLOAD];
                packet[..].copy_from_slice(ref_packet);
                client.send(&mut packet).unwrap();
                while let Some((len, payload)) = client.recv() {
                    assert_eq!(&payload[..len], ref_packet, "client packet");
                    client_num_packets_received += 1;
                }
            }
            State::Failed(err) => panic!("client error state: {:?}", err),
            State::Disconnected =>  {
                println!("client disconnected");
                break;
            }
        }

        server.update(|c, user| {
            let user = unsafe { std::ffi::CStr::from_ptr(user.as_ptr() as *const _) };
            println!("connected[{}] {:?} with data {:?}", c.id(), c.addr(), user);
            connected.push(c);
        });

        if let Some(conn) = connected.get(0) {
            let _ = conn.send(ref_packet);
            while let Ok(len) = conn.recv(&mut buf) {
                if len == 0 { break; }
                let payload = &buf[..len as usize];
                assert_eq!(payload, ref_packet, "server packet");
                server_num_packets_received += 1;
            }
        }

        if client_num_packets_received >= 10 && server_num_packets_received >= 10 {
            if connected.len() != 0 {
                let conn = connected.remove(0);
                println!("client and server successfully exchanged packets");
                conn.close();
            }
        }
    }

    println!("shutting down");
}
