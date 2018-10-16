#![feature(plugin)]
#![plugin(rocket_codegen)]

use oni::{crypto::{keygen, KEY}, token::{USER, PublicToken}, ServerList};
use rocket::State;
use rocket::response::content::Html;
use std::sync::RwLock;
use std::net::SocketAddr;

static SERVER: &str = "127.0.0.1:40000";

const CONNECT_TOKEN_EXPIRY: u32 = 45;
const CONNECT_TOKEN_TIMEOUT: u32 = 5;

type Servers = RwLock<Vec<SocketAddr>>;

#[get("/")]
fn index(private: State<[u8; KEY]>, servers: State<Servers>) -> Html<String> {
    let servers: &Vec<SocketAddr> = &servers.read().unwrap();
    Html(format!("<!doctype html>
<html>
<head>
</head>
<body>
Hello, <strong>world!</strong>
<h3>Servers:</h3>
<pre>
{:#?}
</pre>
</body>
</html>", servers))
}

#[post("/add-server")]
fn add_server(servers: State<Servers>, addr: SocketAddr) -> Vec<u8> {
    unimplemented!()
}

#[get("/<protocol>/<client>")]
fn gen(private: State<[u8; KEY]>, servers: State<Servers>, protocol: u64, client: u64) -> Vec<u8> {
    let mut server_list = ServerList::new();
    for addr in servers.read().unwrap().iter() {
        server_list.push(*addr).unwrap();
    }

    let data = server_list.serialize().unwrap();
    let user = [0u8; USER];
    //(&mut user[..]).write(b"some user data\0").unwrap();

    let connect_token = PublicToken::generate(
        data, user,
        CONNECT_TOKEN_EXPIRY,
        CONNECT_TOKEN_TIMEOUT,
        client, protocol,
        &private,
    );

    connect_token.into_vec()
}

fn main() {
    let private_key = keygen();
    let mut server_list = ServerList::new();
    server_list.push(SERVER.parse().unwrap()).unwrap();
    let servers: Servers = RwLock::new(vec![SERVER.parse().unwrap()]);
    rocket::ignite()
        .manage(private_key)
        .manage(servers)
        .mount("/", routes![index])
        .mount("/match", routes![gen])
        .launch();
}
