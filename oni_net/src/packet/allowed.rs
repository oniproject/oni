use super::{
    REQUEST,
    DENIED,
    CHALLENGE,
    RESPONSE,
    KEEP_ALIVE,
    DISCONNECT,
    _RESERVED_0,
    _RESERVED_1,
    PAYLOAD,
};

bitflags! {
    pub struct Allowed: u16 {
        const REQUEST =     1 << REQUEST;
        const DENIED =      1 << DENIED;
        const CHALLENGE =   1 << CHALLENGE;
        const RESPONSE =    1 << RESPONSE;
        const KEEP_ALIVE =  1 << KEEP_ALIVE;
        const DISCONNECT =  1 << DISCONNECT;

        const _RESERVED_0 = 1 << _RESERVED_0;
        const _RESERVED_1 = 1 << _RESERVED_1;

        const PAYLOAD =     1 << PAYLOAD;

        const CLIENT_CONNECTED = Self::PAYLOAD.bits | Self::KEEP_ALIVE.bits | Self::DISCONNECT.bits;
        const CLIENT_SENDING_RESPONSE = Self::DENIED.bits | Self::KEEP_ALIVE.bits;
        const CLIENT_SENDING_REQUEST = Self::DENIED.bits | Self::CHALLENGE.bits;
    }
}

impl Allowed {
    pub fn packet_type(self, p: u8) -> bool {
        if      p == REQUEST    { self.contains(Allowed::REQUEST)   }
        else if p == DENIED     { self.contains(Allowed::DENIED)    }
        else if p == CHALLENGE  { self.contains(Allowed::CHALLENGE) }
        else if p == RESPONSE   { self.contains(Allowed::RESPONSE)  }
        else if p == KEEP_ALIVE { self.contains(Allowed::KEEP_ALIVE)}
        else if p == DISCONNECT { self.contains(Allowed::DISCONNECT)}
        else if p >= PAYLOAD    { self.contains(Allowed::PAYLOAD)   }
        else { false }
    }
}
