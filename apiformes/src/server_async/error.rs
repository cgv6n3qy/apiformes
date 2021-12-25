use crate::packets::prelude::DataParseError;
use std::io;
#[derive(Debug)]
pub enum ServerError {
    Io(io::Error),
    Packet(DataParseError),

    #[cfg(feature = "noise")]
    Noise(snow::Error),

    FirstPacketNotConnect,
    Misc(String),
}

impl From<io::Error> for ServerError {
    fn from(err: io::Error) -> ServerError {
        ServerError::Io(err)
    }
}

impl From<DataParseError> for ServerError {
    fn from(err: DataParseError) -> ServerError {
        ServerError::Packet(err)
    }
}

#[cfg(feature = "noise")]
impl From<snow::Error> for ServerError {
    fn from(err: snow::Error) -> ServerError {
        ServerError::Noise(err)
    }
}
