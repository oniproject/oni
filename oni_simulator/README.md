[![Latest version](https://img.shields.io/crates/v/oni_simulator.svg)](https://crates.io/crates/oni_simulator)
[![Documentation](https://docs.rs/oni_simulator/badge.svg)](https://docs.rs/oni_simulator)
[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](../COPYNG)

```rust
use oni_simulator::{Simulator, DefaultMTU};
use std::io;

let sim = Simulator::<DefaultMTU>::new();

let from = "[::1]:1111".parse().unwrap();
let to   = "[::1]:2222".parse().unwrap();

let from = sim.add_socket(from);
let to   = sim.add_socket(to);

from.send_to(&[1, 2, 3], to.local_addr()).unwrap();
sim.advance();

let mut buf = [0u8; 4];
let (bytes, addr) = to.recv_from(&mut buf[..]).unwrap();
assert_eq!(bytes, 3);
assert_eq!(addr, from.local_addr());
assert_eq!(&buf[..bytes], &[1, 2, 3]);

let err = to.recv_from(&mut buf[..]).unwrap_err();
assert_eq!(err.kind(), io::ErrorKind::WouldBlock)
```
