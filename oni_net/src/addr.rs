use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::{
    io::{self, prelude::*, Error, ErrorKind},
    net::{
        Ipv4Addr,
        Ipv6Addr,
        SocketAddr,
        SocketAddrV4,
        SocketAddrV6,
    },
};

pub const MAX_SERVERS_PER_CONNECT: usize = 32;

impl<T: Read> ReadIps for T {}
impl<T: Write> WriteIps for T {}

pub trait ReadIps: Read {
    fn read_ips(&mut self) -> io::Result<Vec<SocketAddr>> {
        let count = self.read_u32::<LE>()?;
        if count == 0 || count > MAX_SERVERS_PER_CONNECT as u32 {
            return Err(Error::new(ErrorKind::InvalidInput, "num_server_addresses not in [1,32]"));
        }
        let mut ips = Vec::with_capacity(count as usize);
        for _ in 0..count {
            // value of 1 = IPv4 address, 2 = IPv6 address.
            ips.push(match self.read_u8()? {
                // for a given IPv4 address: a.b.c.d:port
                0 => SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(
                    self.read_u8()?,
                    self.read_u8()?,
                    self.read_u8()?,
                    self.read_u8()?,
                ), self.read_u16::<LE>()?)),
                // for a given IPv6 address: [a:b:c:d:e:f:g:h]:port
                1 => SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::new(
                    self.read_u16::<LE>()?,
                    self.read_u16::<LE>()?,
                    self.read_u16::<LE>()?,
                    self.read_u16::<LE>()?,
                    self.read_u16::<LE>()?,
                    self.read_u16::<LE>()?,
                    self.read_u16::<LE>()?,
                    self.read_u16::<LE>()?,
                ), self.read_u16::<LE>()?, 0, 0)),
                // error
                _ => return Err(Error::new(ErrorKind::InvalidInput, "address type invalid")),
            });
        }
        Ok(ips)
    }
}

pub trait WriteIps: Write {
    fn write_ips(&mut self, ips: &[SocketAddr]) -> io::Result<()> {
        assert!(ips.len() > 0 && ips.len() < MAX_SERVERS_PER_CONNECT);
        self.write_u32::<LE>(ips.len() as u32)?;
        for ip in ips.iter() {
            match ip {
                SocketAddr::V4(addr) => {
                    self.write_u8(0)?;
                    self.write_all(&addr.ip().octets()[..])?;
                    self.write_u16::<LE>(addr.port())?;
                }
                SocketAddr::V6(addr) => {
                    self.write_u8(1)?;
                    self.write_all(&addr.ip().octets()[..])?;
                    self.write_u16::<LE>(addr.port())?;
                }
            }
        }
        Ok(())
    }
}
