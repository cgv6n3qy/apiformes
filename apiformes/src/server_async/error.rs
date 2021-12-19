use crate::packets::prelude::DataParseError;
use std::io;
#[derive(Debug)]
pub enum ServerError {
    Io(io::Error),
    Packet(DataParseError),
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
