use super::{
    data::{MqttTwoBytesInt, MqttVariableBytesInt},
    error::DataParseError,
    packet::Packet,
    parsable::*,
    props::{MqttPropValue, PropOwner, Properties, Property},
    reason::PubCompReasonCode,
};

use bytes::{Buf, BufMut};

#[derive(Clone)]
pub struct PubComp {
    // 2.2.1 Packet Identifier
    packet_identifier: MqttTwoBytesInt,
    // 3.7.2.1 PUBCOMP Reason Code
    reason_code: PubCompReasonCode,
    // 3.7.2.2 PUBCOMP Properties
    props: Properties,
}

impl PubComp {
    pub fn new(id: u16) -> Self {
        PubComp {
            packet_identifier: MqttTwoBytesInt::new(id),
            reason_code: PubCompReasonCode::Success,
            props: Properties::new(),
        }
    }
    pub fn identifier(&self) -> u16 {
        self.packet_identifier.inner()
    }
    pub fn reason_code(&self) -> PubCompReasonCode {
        self.reason_code
    }
    pub fn set_reason_code(&mut self, reason_code: PubCompReasonCode) {
        self.reason_code = reason_code;
    }
    pub fn add_prop(&mut self, key: Property, value: MqttPropValue) -> Result<(), DataParseError> {
        self.props.checked_insert(key, value, PropOwner::PUBCOMP)
    }
    pub fn get_prop(&self, key: Property) -> Option<&[MqttPropValue]> {
        self.props.get(key)
    }
    pub fn props_iter(&self) -> impl Iterator<Item = (&Property, &MqttPropValue)> {
        self.props.iter()
    }
    fn partial_size(&self) -> usize {
        self.packet_identifier.size() + self.reason_code.size() + self.props.size()
    }
    pub fn build(self) -> Packet {
        Packet::PubComp(self)
    }
}

impl Parsable for PubComp {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let length = MqttVariableBytesInt::new(self.partial_size() as u32)?;
        length.serialize(buf);
        self.packet_identifier.serialize(buf);
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
        let packet_identifier = MqttTwoBytesInt::deserialize(&mut buf)?;
        let reason_code = PubCompReasonCode::deserialize(&mut buf)?;
        let props = Properties::deserialize(&mut buf)?;
        if !props.is_valid_for(PropOwner::PUBCOMP) {
            return Err(DataParseError::BadProperty);
        }
        Ok(PubComp {
            packet_identifier,
            reason_code,
            props,
        })
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
    #[test]
    fn test_pubcomp() {
        let mut pubcomp = PubComp::new(123);
        pubcomp.set_reason_code(PubCompReasonCode::PacketIdentifierNotFound);
        let mut b = BytesMut::new();
        pubcomp.serialize(&mut b).unwrap();
        assert_eq!(b.remaining(), pubcomp.size());
        assert_eq!(
            b,
            &[
                0x04, // size
                0x00, 0x7b, // packet identifier
                0x92, // reasoncode
                0x00  // props
            ][..]
        );
        let pubcomp2 = PubComp::deserialize(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        pubcomp2.serialize(&mut b2).unwrap();
        assert_eq!(b, b2);
    }
}
