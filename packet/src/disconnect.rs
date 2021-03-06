use super::{
    data::MqttVariableBytesInt,
    error::DataParseError,
    packet::Packet,
    parsable::*,
    props::{MqttPropValue, PropOwner, Properties, Property},
    reason::DisconnectReasonCode,
};
use bytes::{Buf, BufMut};

#[derive(Clone)]
pub struct Disconnect {
    // 3.14.2.1 Disconnect Reason Code
    reason_code: DisconnectReasonCode,
    // 3.14.2.2 DISCONNECT Properties
    props: Properties,
}

impl Disconnect {
    pub fn new(reason_code: DisconnectReasonCode) -> Disconnect {
        Disconnect {
            reason_code,
            props: Properties::new(),
        }
    }
    pub fn reason_code(&self) -> DisconnectReasonCode {
        self.reason_code
    }
    pub fn add_prop(&mut self, key: Property, value: MqttPropValue) -> Result<(), DataParseError> {
        self.props.checked_insert(key, value, PropOwner::DISCONNECT)
    }
    pub fn get_prop(&self, key: Property) -> Option<&[MqttPropValue]> {
        self.props.get(key)
    }
    pub fn props_iter(&self) -> impl Iterator<Item = (&Property, &MqttPropValue)> {
        self.props.iter()
    }
    fn partial_size(&self) -> usize {
        self.reason_code.size() + self.props.size()
    }
    pub fn build(self) -> Packet {
        Packet::Disconnect(self)
    }
}

impl MqttSerialize for Disconnect {
    fn serialize<T: BufMut>(&self, buf: &mut T) {
        let length = MqttVariableBytesInt::new(self.partial_size() as u32)
            .expect("Somehow you allocated a packet that is larger than the allowed size");
        length.serialize(buf);
        self.reason_code.serialize(buf);
        self.props.serialize(buf);
    }
}
impl MqttDeserialize for Disconnect {
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let length = MqttVariableBytesInt::deserialize(buf)?.inner() as usize;
        if length < Disconnect::min_size() - MqttVariableBytesInt::min_size() {
            return Err(DataParseError::BadDisconnectMessage);
        }
        if buf.remaining() < length {
            return Err(DataParseError::InsufficientBuffer {
                needed: length,
                available: buf.remaining(),
            });
        }
        let mut buf = buf.take(length);
        let reason_code = DisconnectReasonCode::unchecked_deserialize(&mut buf)?;
        let props = Properties::deserialize(&mut buf)?;
        if !props.is_valid_for(PropOwner::DISCONNECT) {
            return Err(DataParseError::BadProperty);
        }
        Ok(Disconnect { reason_code, props })
    }
}
impl MqttSize for Disconnect {
    fn min_size() -> usize {
        MqttVariableBytesInt::min_size() + DisconnectReasonCode::min_size() + Properties::min_size()
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
    fn test_disconnect() {
        let disconnect = Disconnect::new(DisconnectReasonCode::UnspecifiedError);
        let mut b = BytesMut::new();
        disconnect.serialize(&mut b);
        assert_eq!(b.remaining(), disconnect.size());
        assert_eq!(
            b,
            &[
                0x02, // size
                0x80, // reasoncode
                0x00
            ][..]
        );
        let disconnect2 = Disconnect::deserialize(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        disconnect2.serialize(&mut b2);
        assert_eq!(b, b2);
    }
}
