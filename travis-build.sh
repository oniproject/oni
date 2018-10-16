#!/bin/sh

export LD_LIBRARY_PATH=${HOME}/libsodium/lib:${LD_LIBRARY_PATH}
export PKG_CONFIG_PATH=${HOME}/libsodium/lib/pkgconfig:${PKG_CONFIG_PATH}
export LDFLAGS="-L${HOME}/libsodium/lib"

cargo build
cargo test
cargo doc --no-deps

#cargo doc --no-deps -p oni_simulator
