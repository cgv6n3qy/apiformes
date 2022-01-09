use super::{
    data::{MqttTwoBytesInt, MqttVariableBytesInt},
    error::DataParseError,
    packet::Packet,
    parsable::*,
    props::{MqttPropValue, PropOwner, Properties, Property},
    reason::UnsubAckReasonCode,
};
use bytes::{Buf, BufMut};

#[derive(Clone)]
pub struct UnsubAck {
    // 2.2.1 Packet Identifier
    packet_identifier: MqttTwoBytesInt,

    // 3.11.2.1 UNSUBACK Properties
    props: Properties,

    // 3.11.3 UNSUBACK Payload
    reason_codes: Vec<UnsubAckReasonCode>,
}

impl UnsubAck {
    pub fn new(id: u16) -> UnsubAck {
        UnsubAck {
            packet_identifier: MqttTwoBytesInt::new(id),
            props: Properties::new(),
            reason_codes: Vec::new(),
        }
    }
    pub fn identifier(&self) -> u16 {
        self.packet_identifier.inner()
    }
    pub fn reason_codes(&self) -> &[UnsubAckReasonCode] {
        &self.reason_codes
    }
    pub fn add_reason_code(&mut self, reason_code: UnsubAckReasonCode) {
        self.reason_codes.push(reason_code);
    }
    pub fn add_prop(&mut self, key: Property, value: MqttPropValue) -> Result<(), DataParseError> {
        self.props.checked_insert(key, value, PropOwner::UNSUBACK)
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
        Packet::UnsubAck(self)
    }
}

impl MqttSerialize for UnsubAck {
    fn serialize<T: BufMut>(&self, buf: &mut T) {
        let length = MqttVariableBytesInt::new(self.partial_size() as u32)
            .expect("Somehow you allocated a packet that is larger than the allowed size");
        length.serialize(buf);
        self.packet_identifier.serialize(buf);
        self.props.serialize(buf);
        for r in &self.reason_codes {
            r.serialize(buf);
        }
    }
}
impl MqttDeserialize for UnsubAck {
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let length = MqttVariableBytesInt::deserialize(buf)?.inner() as usize;
        if length < UnsubAck::min_size() - MqttVariableBytesInt::min_size() {
            return Err(DataParseError::BadUnsubAckMessage);
        }
        if buf.remaining() < length {
            return Err(DataParseError::InsufficientBuffer {
                needed: length,
                available: buf.remaining(),
            });
        }
        let mut buf = buf.take(length);
        let packet_identifier = MqttTwoBytesInt::unchecked_deserialize(&mut buf)?;
        let props = Properties::deserialize(&mut buf)?;
        if !props.is_valid_for(PropOwner::UNSUBACK) {
            return Err(DataParseError::BadProperty);
        }
        let mut reason_codes = Vec::new();
        while buf.remaining() > 0 {
            let r = UnsubAckReasonCode::unchecked_deserialize(&mut buf)?;
            reason_codes.push(r);
        }
        if reason_codes.is_empty() {
            Err(DataParseError::BadUnsubAckMessage)
        } else {
            Ok(UnsubAck {
                packet_identifier,
                props,
                reason_codes,
            })
        }
    }
}

impl MqttSize for UnsubAck {
    fn min_size() -> usize {
        MqttVariableBytesInt::min_size()
            + MqttTwoBytesInt::min_size()
            + Properties::min_size()
            + UnsubAckReasonCode::min_size() // at least one unsubscribe
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
    fn test_unsuback() {
        let mut unsuback = UnsubAck::new(123);
        unsuback.add_reason_code(UnsubAckReasonCode::UnspecifiedError);
        unsuback.add_reason_code(UnsubAckReasonCode::UnspecifiedError);
        let mut b = BytesMut::new();
        unsuback.serialize(&mut b);
        assert_eq!(b.remaining(), unsuback.size());
        assert_eq!(
            b,
            &[
                0x05, // size
                0x00, 0x7b, // packet identifier
                0x00, //props
                0x80, // reasoncode
                0x80, // reasoncode
            ][..]
        );
        let unsuback2 = UnsubAck::deserialize(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        unsuback2.serialize(&mut b2);
        assert_eq!(b, b2);
    }
}
