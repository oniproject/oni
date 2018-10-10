use std::thread::sleep;
use std::time::Duration;
use std::io::Write;

use oni_net::{
    keygen, crypto_random,
    PublicToken, USER, DATA,
    Server,
    Client, State,
    MAX_PAYLOAD,
};

#[test]
fn client_server() {
    const CONNECT_TOKEN_EXPIRY: u32 = 30;
    const CONNECT_TOKEN_TIMEOUT: u32 = 5;
    const PROTOCOL_ID: u64 =  0x1122334455667788;
    const DELTA_TIME: Duration = Duration::from_millis(1000 / 60);

    let private_key = keygen();

    let mut data = [0u8; DATA];
    let mut user = [0u8; USER];
    crypto_random(&mut data[..]);
    //crypto_random(&mut user[..]);
    (&mut user[..]).write(b"some user data\0").unwrap();

    println!("[client/server]");

    let client_id = 1345643;
    let connect_token = PublicToken::generate(
        data, user,
        CONNECT_TOKEN_EXPIRY,
        CONNECT_TOKEN_TIMEOUT,
        client_id,
        PROTOCOL_ID,
        &private_key,
    );

    let mut client = Client::new(PROTOCOL_ID, &connect_token, "[::1]:0".parse().unwrap()).unwrap();
    let mut server = Server::new(PROTOCOL_ID, private_key, "[::1]:40000".parse().unwrap()).unwrap();

    client.connect(server.local_addr()).unwrap();

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
    println!("[start]");
    loop {
        println!(" - - - - - -");

        client.update();
        server.update(|c, user| {
            let user = unsafe { std::ffi::CStr::from_ptr(user.as_ptr() as *const _) };
            println!("connected[{}] {:?} with data {:?}", c.id(), c.addr(), user);
            connected.push(c);
        });

        match client.state() {
            State::Connecting(v) => println!("client {:?}", v),
            State::Connected => {
                client.send(ref_packet);

                while let Some((len, payload)) = client.recv() {
                    assert_eq!(&payload[..len], ref_packet, "client packet");
                    client_num_packets_received += 1;
                }
            }
            State::Failed(err) =>  {
                println!("client error state: {:?}", err);
                break;
            }
            State::Disconnected =>  {
                println!("client disconnected");
                break;
            }
        }

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
            if connected.len() != 0 {
                let client = connected.remove(0);
                println!("client and server successfully exchanged packets");
                client.close();
            }
        }

        sleep(DELTA_TIME);
    }

    println!("shutting down");
}
