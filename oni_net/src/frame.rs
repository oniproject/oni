/*
enum Frame<'a> {
    StopWaiting(u64),
    Unreliable {
        message_num: u64,
        offset: Option<std::num::NonZeroU64>,
        payload: &'a [u8],
    },
}

fn decode_payload(buf: &[u8]) -> std::io::Result<Frame> {
    use byteorder::{LE, ReadBytesExt};

    use std::num::NonZeroU64;
    use std::io::{Error, ErrorKind::{
        InvalidData,
    }};

    let mut p = &buf[..];

    let prefix = p.read_u8()?;

    match prefix & 0b11000000 {
        0b00000000 => {
            let e = (prefix & 0b100000) != 0;
            let m = (prefix & 0b010000) != 0;
            let o = (prefix & 0b001000) != 0;
            let sss = (prefix & 0b111) as usize;

            let message_num = unimplemented!(); //buf.read_varint();

            let offset = if o {
                //Some(buf.read_varint())
                NonZeroU64::new(unimplemented!())
            } else {
                None
            };

            let size = match sss {
                0b111 => buf.len() - (buf.len() - p.len()),
                0b101 | 0b110 =>
                    return Err(Error::new(InvalidData, "reserved size")),
                _ => {
                    (p.read_u8()? as usize) | sss << 8
                }
            };

            let is_first = offset.is_none();
            let is_last = e && offset.is_none();

            // FIXME: may panic
            let payload = &buf[p.len()..size];

            Ok(Frame::Unreliable { message_num, offset, payload })
        }
        0b01000000 => {
            unimplemented!("reliable")
        }
        0b10000000 => {
            // control

            //if prefix & 0b1000_0000 {
            //}
            let size = match prefix & 0b11 {
                0b00 => 1,
                0b01 => 2,
                0b10 => 3,
                0b11 => 8,
                _ => unsafe { std::hint::unreachable_unchecked() },
            };
            Ok(Frame::StopWaiting(p.read_uint::<LE>(size)?))
        }
        _ => Err(Error::new(InvalidData, "reserved prefix")),
    }
}
*/

/*
    00emosss [message_num] [offset] [size] data

    e: 0: There's more data after ths in the unreliable message.
          (Will be sent in another packet.)
       1: This is the last segment in the unreliable message.
    m: encoded size of message_num
       First segment in packet: message_num is absolute.  Only bottom N bits are sent.
           0: 16-bits
           1: 32-bits
       Subsequent segments: message number field is relative to previous
           0: no message number field follows, assume 1 greater than previous segment
           1: Var-int encoded offset from previous follows
           (NOTE: while encoding/decoding a packet, any reliable segment frames sent after unreliable data
           will *also* increment the current message number, even though the message number is *not*
           guaranteed to match that reliable segment.  Since in practice the message number often will
           match, making this encode/decode rule affords a small optimization.)
    o:  offset of this segment within message
        If first segment in packet, or message number differs from previous segment in packet:
            0: Zero offset, segment is first in message.  No offset field follows.
            1: varint-encoded offset follows
    sss: Size of data
        000-100: Append upper three bits to lower 8 bits in explicit size field,
                 which follows  (Max value is 0x4ff = 1279, which is larger than our MTU)
        101,110: Reserved
        111: This is the last frame, so message data extends to the end of the packet.
*/



