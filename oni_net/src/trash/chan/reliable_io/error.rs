#[derive(Debug)]
pub enum Error {
    PacketTooLarge,
    PacketStale,

    PacketTooLargeToSend,
    PacketTooLargeToRecv,
    FragmentHeaderInvalid,
    FragmentInvalid,
    FragmentAlreadyReceived,
    Io(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}
