use super::{
    data::MqttVariableBytesInt,
    packet::Packet,
    parsable::{DataParseError, Parsable},
    props::{MqttPropValue, PropOwner, Properties, Property},
    reason::AuthReasonCode,
};
use bytes::{Buf, BufMut};

#[derive(Clone)]
pub struct Auth {
    // 3.14.2.1 Auth Reason Code
    reason_code: AuthReasonCode,
    // 3.14.2.2 AUTH Properties
    props: Properties,
}

impl Auth {
    pub fn new(reason_code: AuthReasonCode) -> Auth {
        Auth {
            reason_code,
            props: Properties::new(),
        }
    }
    pub fn reason_code(&self) -> AuthReasonCode {
        self.reason_code
    }
    pub fn add_prop(&mut self, key: Property, value: MqttPropValue) -> Result<(), DataParseError> {
        self.props.checked_insert(key, value, PropOwner::AUTH)
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
        Packet::Auth(self)
    }
}

impl Parsable for Auth {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let length = MqttVariableBytesInt::new(self.partial_size() as u32)?;
        length.serialize(buf)?;
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
        let reason_code = AuthReasonCode::deserialize(&mut buf)?;
        let props = Properties::deserialize(&mut buf)?;
        if !props.is_valid_for(PropOwner::AUTH) {
            return Err(DataParseError::BadProperty);
        }
        Ok(Auth { reason_code, props })
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
    fn test_auth() {
        let auth = Auth::new(AuthReasonCode::ReAuthenticate);
        let mut b = BytesMut::new();
        auth.serialize(&mut b).unwrap();
        assert_eq!(b.remaining(), auth.size());
        assert_eq!(
            b,
            &[
                0x02, // size
                0x19, // reasoncode
                0x00
            ][..]
        );
        let auth2 = Auth::deserialize(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        auth2.serialize(&mut b2).unwrap();
        assert_eq!(b, b2);
    }
}
