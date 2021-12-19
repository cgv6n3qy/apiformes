use crate::data::MqttVariableBytesInt;
use crate::parsable::{DataParseError, Parsable};
use bytes::{Buf, BufMut};

pub struct PingReq {}
impl Default for PingReq {
    fn default() -> Self {
        Self::new()
    }
}
impl PingReq {
    pub fn new() -> PingReq {
        PingReq {}
    }
}

impl Parsable for PingReq {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let length = MqttVariableBytesInt::new(0)?;
        length.serialize(buf)?;
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let length = MqttVariableBytesInt::deserialize(buf)?.inner() as usize;
        match length {
            0 => Ok(PingReq {}),
            _ => Err(DataParseError::BadPing),
        }
    }
    fn size(&self) -> usize {
        1
    }
}

pub type PingRes = PingReq;
