extern crate ws;

extern crate specs;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_cbor;

mod server;

fn main() {

    server::run("127.0.0.1:9000");
}
