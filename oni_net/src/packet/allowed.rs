use super::{
    REQUEST,
    DENIED,
    CHALLENGE,
    RESPONSE,
    KEEP_ALIVE,
    PAYLOAD,
    DISCONNECT,
};

bitflags! {
    pub struct Allowed: u8 {
        const REQUEST =     1 << REQUEST;
        const DENIED =      1 << DENIED;
        const CHALLENGE =   1 << CHALLENGE;
        const RESPONSE =    1 << RESPONSE;
        const KEEP_ALIVE =  1 << KEEP_ALIVE;
        const PAYLOAD =     1 << PAYLOAD;
        const DISCONNECT =  1 << DISCONNECT;

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
        else if p == PAYLOAD    { self.contains(Allowed::PAYLOAD)   }
        else if p == DISCONNECT { self.contains(Allowed::DISCONNECT)}
        else { false }
    }
}
