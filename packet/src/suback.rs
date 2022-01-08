use super::{
    data::{MqttTwoBytesInt, MqttVariableBytesInt},
    error::DataParseError,
    packet::Packet,
    parsable::*,
    props::{MqttPropValue, PropOwner, Properties, Property},
    reason::SubAckReasonCode,
};
use bytes::{Buf, BufMut};

#[derive(Clone)]
pub struct SubAck {
    // 2.2.1 Packet Identifier
    packet_identifier: MqttTwoBytesInt,

    // 3.9.2.1 SUBACK Properties
    props: Properties,

    // 3.9.3 SUBACK Payload
    reason_codes: Vec<SubAckReasonCode>,
}

impl SubAck {
    pub fn new(id: u16) -> SubAck {
        SubAck {
            packet_identifier: MqttTwoBytesInt::new(id),
            props: Properties::new(),
            reason_codes: Vec::new(),
        }
    }
    pub fn identifier(&self) -> u16 {
        self.packet_identifier.inner()
    }
    pub fn reason_codes(&self) -> &[SubAckReasonCode] {
        &self.reason_codes
    }
    pub fn add_reason_code(&mut self, reason_code: SubAckReasonCode) {
        self.reason_codes.push(reason_code);
    }
    pub fn add_prop(&mut self, key: Property, value: MqttPropValue) -> Result<(), DataParseError> {
        self.props.checked_insert(key, value, PropOwner::SUBACK)
    }
    pub fn get_prop(&self, key: Property) -> Option<&[MqttPropValue]> {
        self.props.get(key)
    }
    pub fn props_iter(&self) -> impl Iterator<Item = (&Property, &MqttPropValue)> {
        self.props.iter()
    }
    fn partial_size(&self) -> usize {
        self.packet_identifier.size()
            + self.props.size()
            + self.reason_codes.iter().map(|r| r.size()).sum::<usize>()
    }
    pub fn build(self) -> Packet {
        Packet::SubAck(self)
    }
}

impl Parsable for SubAck {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let length = MqttVariableBytesInt::new(self.partial_size() as u32)?;
        length.serialize(buf)?;
        self.packet_identifier.serialize(buf);
        self.props.serialize(buf)?;
        for r in &self.reason_codes {
            r.serialize(buf)?;
        }
        Ok(())
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
        let props = Properties::deserialize(&mut buf)?;
        if !props.is_valid_for(PropOwner::SUBACK) {
            return Err(DataParseError::BadProperty);
        }
        let mut reason_codes = Vec::new();
        while buf.remaining() > 0 {
            let r = SubAckReasonCode::deserialize(&mut buf)?;
            reason_codes.push(r);
        }
        if reason_codes.is_empty() {
            Err(DataParseError::BadSubAckMessage)
        } else {
            Ok(SubAck {
                packet_identifier,
                props,
                reason_codes,
            })
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
    #[test]
    fn test_suback() {
        let mut suback = SubAck::new(123);
        suback.add_reason_code(SubAckReasonCode::GrantedQoS1);
        suback.add_reason_code(SubAckReasonCode::GrantedQoS1);
        let mut b = BytesMut::new();
        suback.serialize(&mut b).unwrap();
        assert_eq!(b.remaining(), suback.size());
        assert_eq!(
            b,
            &[
                0x05, // size
                0x00, 0x7b, // packet identifier
                0x00, //props
                0x01, // reasoncode
                0x01, // reasoncode
            ][..]
        );
        let suback2 = SubAck::deserialize(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        suback2.serialize(&mut b2).unwrap();
        assert_eq!(b, b2);
    }
}
