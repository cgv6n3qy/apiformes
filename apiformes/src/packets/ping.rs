use super::{
    data::MqttVariableBytesInt,
    packet::Packet,
    parsable::{DataParseError, Parsable},
};
use bytes::{Buf, BufMut};

pub struct Ping {}
impl Default for Ping {
    fn default() -> Self {
        Self::new()
    }
}
impl Ping {
    pub fn new() -> Ping {
        Ping {}
    }
    pub fn build_req(self) -> Packet {
        Packet::PingReq(self)
    }
    pub fn build_res(self) -> Packet {
        Packet::PingRes(self)
    }
}

impl Parsable for Ping {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let length = MqttVariableBytesInt::new(0)?;
        length.serialize(buf)?;
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let length = MqttVariableBytesInt::deserialize(buf)?.inner() as usize;
        match length {
            0 => Ok(Ping {}),
            _ => Err(DataParseError::BadPing),
        }
    }
    fn size(&self) -> usize {
        1
    }
}
