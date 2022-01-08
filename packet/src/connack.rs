use super::{
    data::{MqttOneBytesInt, MqttVariableBytesInt},
    error::DataParseError,
    packet::Packet,
    parsable::*,
    props::{MqttPropValue, PropOwner, Properties, Property},
    reason::ConnAckReasonCode,
};
use bitflags::bitflags;
use bytes::{Buf, BufMut};

bitflags! {
    pub struct ConnAckFlags: u8 {
        const SESSION_PRESENT = 0b0000_0001;
    }
}
impl Parsable for ConnAckFlags {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let flags = MqttOneBytesInt::new(self.bits());
        flags.serialize(buf);
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let raw_flags = MqttOneBytesInt::deserialize(buf)?;
        ConnAckFlags::from_bits(raw_flags.inner()).ok_or(DataParseError::BadConnectMessage)
    }
    fn size(&self) -> usize {
        1
    }
}

#[derive(Clone)]
pub struct ConnAck {
    // 3.2.2.1 Connect Acknowledge Flags
    flags: ConnAckFlags,
    // 3.2.2.2 Connect Reason Code
    reason_code: ConnAckReasonCode,
    // 3.2.2.3 CONNACK Properties
    props: Properties,
}

impl Default for ConnAck {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnAck {
    pub fn new() -> Self {
        ConnAck {
            flags: ConnAckFlags::from_bits_truncate(0),
            reason_code: ConnAckReasonCode::Success,
            props: Properties::new(),
        }
    }
    pub fn flags(&self) -> ConnAckFlags {
        self.flags
    }
    pub fn set_session_present(&mut self) {
        self.flags |= ConnAckFlags::SESSION_PRESENT;
    }
    pub fn reason_code(&self) -> ConnAckReasonCode {
        self.reason_code
    }
    pub fn set_reason_code(&mut self, reason_code: ConnAckReasonCode) {
        self.reason_code = reason_code
    }
    pub fn add_prop(&mut self, key: Property, value: MqttPropValue) -> Result<(), DataParseError> {
        self.props.checked_insert(key, value, PropOwner::CONNACK)
    }
    pub fn get_prop(&self, key: Property) -> Option<&[MqttPropValue]> {
        self.props.get(key)
    }
    pub fn props_iter(&self) -> impl Iterator<Item = (&Property, &MqttPropValue)> {
        self.props.iter()
    }
    fn partial_size(&self) -> usize {
        self.flags.size() + self.reason_code.size() + self.props.size()
    }
    pub fn build(self) -> Packet {
        Packet::ConnAck(self)
    }
}

impl Parsable for ConnAck {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let length = MqttVariableBytesInt::new(self.partial_size() as u32)?;
        length.serialize(buf);
        self.flags.serialize(buf)?;
        self.reason_code.serialize(buf)?;
        self.props.serialize(buf)
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let length = MqttVariableBytesInt::deserialize(buf)?.inner() as usize;
        if buf.remaining() < length {
            return Err(DataParseError::InsufficientBuffer {
                needed: length,
                available: buf.remaining(),
            });
        }
        let mut buf = buf.take(length);
        let flags = ConnAckFlags::deserialize(&mut buf)?;
        let reason_code = ConnAckReasonCode::deserialize(&mut buf)?;
        let props = Properties::deserialize(&mut buf)?;
        if !props.is_valid_for(PropOwner::CONNACK) {
            return Err(DataParseError::BadProperty);
        }
        let packet = ConnAck {
            flags,
            reason_code,
            props,
        };
        if packet.partial_size() == length {
            Ok(packet)
        } else {
            Err(DataParseError::BadConnectMessage)
        }
    }
    fn size(&self) -> usize {
        let size = self.partial_size();
        MqttVariableBytesInt::new(size as u32).unwrap().size() + size
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bytes::BytesMut;
    use std::sync::Arc;
    #[test]
    fn test_connack() {
        let mut connack = ConnAck::new();
        connack.set_session_present();
        connack.set_reason_code(ConnAckReasonCode::Banned);
        connack
            .add_prop(
                Property::ReasonString,
                MqttPropValue::new_string(Arc::from("Déjà vu")).unwrap(),
            )
            .unwrap();
        let mut b = BytesMut::new();
        connack.serialize(&mut b).unwrap();
        assert_eq!(b.remaining(), connack.size());
        assert_eq!(
            b,
            &[
                0x0f, // size
                0x01, // flag
                0x8a, // reasoncode
                0x0c, // props size
                0x1f, // props code
                0x00, 0x09, 0x44, 0xc3, 0xa9, 0x6a, 0xc3, 0xa0, 0x20, 0x76, 0x75 // string
            ][..]
        );
        let connack2 = ConnAck::deserialize(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        connack2.serialize(&mut b2).unwrap();
        assert_eq!(b, b2);
    }
}
