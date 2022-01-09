use super::{data::MqttOneBytesInt, error::DataParseError, packet::Packet, parsable::*};
use bytes::{Buf, BufMut};

#[derive(Clone)]
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

impl MqttSerialize for Ping {
    fn serialize<T: BufMut>(&self, buf: &mut T) {
        let length = MqttOneBytesInt::new(0);
        length.serialize(buf);
    }
}

impl MqttUncheckedDeserialize for Ping {
    fn unchecked_deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let length = MqttOneBytesInt::unchecked_deserialize(buf)?.inner() as usize;
        match length {
            0 => Ok(Ping {}),
            _ => Err(DataParseError::BadPing),
        }
    }
    fn fixed_size() -> usize {
        1
    }
}
