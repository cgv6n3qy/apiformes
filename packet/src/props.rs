use super::{
    data::{
        MqttBinaryData, MqttFourBytesInt, MqttOneBytesInt, MqttTwoBytesInt, MqttUtf8String,
        MqttUtf8StringPair, MqttVariableBytesInt,
    },
    error::DataParseError,
    parsable::*,
};
use bitflags::bitflags;
use bytes::{Buf, BufMut, Bytes};
use std::collections::HashMap;
use std::sync::Arc;

bitflags! {
    pub struct PropOwner: u16 {
        const AUTH =            0b0000_0000_0000_0001;
        const CONNACK =         0b0000_0000_0000_0010;
        const CONNECT =         0b0000_0000_0000_0100;
        const DISCONNECT =      0b0000_0000_0000_1000;
        const PUBACK =          0b0000_0000_0001_0000;
        const PUBCOMP =         0b0000_0000_0010_0000;
        const PUBLISH =         0b0000_0000_0100_0000;
        const PUBREC =          0b0000_0000_1000_0000;
        const PUBREL =          0b0000_0001_0000_0000;
        const SUBACK =          0b0000_0010_0000_0000;
        const SUBSCRIBE =       0b0000_0100_0000_0000;
        const UNSUBACK =        0b0000_1000_0000_0000;
        const UNSUBSCRIBE =     0b0001_0000_0000_0000;
        const WILL =            0b0010_0000_0000_0000;

        const ALL_MESSAGES =    0b1111_1111_1111_1111;
    }
}

#[derive(Clone)]
pub struct Properties {
    size: usize,
    valid: PropOwner,
    props: HashMap<Property, Vec<MqttPropValue>>,
}
impl Default for Properties {
    fn default() -> Self {
        Self::new()
    }
}
impl Properties {
    pub fn new() -> Self {
        Properties {
            size: 0,
            valid: PropOwner::ALL_MESSAGES,
            props: HashMap::new(),
        }
    }
    pub fn insert(&mut self, key: Property, value: MqttPropValue) -> Result<(), DataParseError> {
        let (filter, prop_type, multiple) = key.auxiliary_data();
        if value.prop_type() != prop_type {
            return Err(DataParseError::BadProperty);
        }
        self.valid &= filter;
        match self.unchecked_insert(key, value, multiple) {
            Some(_) => Err(DataParseError::BadProperty),
            None => Ok(()),
        }
    }
    fn unchecked_insert(
        &mut self,
        key: Property,
        value: MqttPropValue,
        multiple: bool,
    ) -> Option<MqttPropValue> {
        if multiple {
            self.size += key.size() + value.size();
            let old = self.props.get_mut(&key);
            match old {
                Some(old) => old.push(value),
                None => drop(self.props.insert(key, vec![value])),
            }
            None
        } else {
            self.size += value.size();
            let old = self.props.insert(key, vec![value]);
            if let Some(v) = old.as_ref() {
                self.size -= v[0].size();
            } else {
                self.size += key.size();
            }
            old.map(|mut v| v.remove(0))
        }
    }
    pub fn checked_insert(
        &mut self,
        key: Property,
        value: MqttPropValue,
        ty: PropOwner,
    ) -> Result<(), DataParseError> {
        let (filter, prop_type, multiple) = key.auxiliary_data();
        if !filter.contains(ty) || prop_type != value.prop_type() {
            return Err(DataParseError::BadProperty);
        }
        self.valid &= filter;
        self.unchecked_insert(key, value, multiple);
        Ok(())
    }
    pub fn get(&self, key: Property) -> Option<&[MqttPropValue]> {
        self.props.get(&key).map(|v| v.as_ref())
    }
    pub fn is_valid_for(&self, message: PropOwner) -> bool {
        self.valid & message == message
    }
    pub fn iter(&self) -> impl Iterator<Item = (&Property, &MqttPropValue)> {
        self.props
            .iter()
            .map(|(key, value_vec)| value_vec.iter().map(move |value| (key, value)))
            .flatten()
    }
}

impl Parsable for Properties {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let size = MqttVariableBytesInt::new(self.size as u32)?;
        size.serialize(buf)?;
        for (key, value) in self.iter() {
            key.serialize(buf)?;
            value.serialize(buf)?;
        }
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let mut size = MqttVariableBytesInt::deserialize(buf)?.inner() as usize;
        let mut table = Properties::new();
        let mut buf = buf.take(size);
        while size != 0 {
            let key = Property::deserialize(&mut buf)?;
            let (_, ty, _) = key.auxiliary_data();
            let value = MqttPropValue::deserialize(&mut buf, ty)?;
            size -= key.size() + value.size();
            table.insert(key, value)?;
        }
        Ok(table)
    }
    fn size(&self) -> usize {
        MqttVariableBytesInt::new(self.size as u32).unwrap().size() + self.size
    }
}

#[derive(PartialEq)]
pub(crate) enum MqttPropValueType {
    Bool,
    Byte,
    FourBytesInt,
    String,
    StringPair,
    Data,
    VarInt,
    TwoBytesInt,
}

///2.2.2.2 Property
#[repr(u32)]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub enum Property {
    PayloadFormatIndicator = 0x1,
    MessageExpiryInterval = 0x2,
    ContentType = 0x3,
    ResponseTopic = 0x8,
    CorrelationData = 0x9,
    SubscriptionIdentifier = 0xb,
    SessionExpiryInterval = 0x11,
    AssignedClientIdentifier = 0x12,
    ServerKeepAlive = 0x13,
    AuthenticationMethod = 0x15,
    AuthenticationData = 0x16,
    RequestProblemInformation = 0x17,
    WillDelayInterval = 0x18,
    RequestResponseInformation = 0x19,
    ResponseInformation = 0x1a,
    ServerReference = 0x1c,
    ReasonString = 0x1f,
    ReceiveMaximum = 0x21,
    TopicAliasMaximum = 0x22,
    TopicAlias = 0x23,
    MaximumQoS = 0x24,
    RetainAvailable = 0x25,
    UserProperty = 0x26,
    MaximumPacketSize = 0x27,
    WildcardSubscriptionAvailable = 0x28,
    SubscriptionIdentifierAvailable = 0x29,
    SharedSubscriptionAvailable = 0x2a,
}

impl Property {
    // First value is who is the owner properties,
    // second value is what value can be stored in there
    // third value is if it is allowed to be stored multiple times
    fn auxiliary_data(&self) -> (PropOwner, MqttPropValueType, bool) {
        match self {
            Property::PayloadFormatIndicator => (
                PropOwner::PUBLISH | PropOwner::WILL,
                MqttPropValueType::Byte,
                false,
            ),
            Property::MessageExpiryInterval => (
                PropOwner::PUBLISH | PropOwner::WILL,
                MqttPropValueType::FourBytesInt,
                false,
            ),
            Property::ContentType => (
                PropOwner::PUBLISH | PropOwner::WILL,
                MqttPropValueType::String,
                false,
            ),
            Property::ResponseTopic => (
                PropOwner::PUBLISH | PropOwner::WILL,
                MqttPropValueType::String,
                false,
            ),
            Property::CorrelationData => (
                PropOwner::PUBLISH | PropOwner::WILL,
                MqttPropValueType::Data,
                false,
            ),
            Property::SubscriptionIdentifier => (
                PropOwner::PUBLISH | PropOwner::SUBSCRIBE,
                MqttPropValueType::VarInt,
                false,
            ),
            Property::SessionExpiryInterval => (
                PropOwner::CONNECT | PropOwner::CONNACK | PropOwner::DISCONNECT,
                MqttPropValueType::FourBytesInt,
                false,
            ),
            Property::AssignedClientIdentifier => {
                (PropOwner::CONNACK, MqttPropValueType::String, false)
            }
            Property::ServerKeepAlive => {
                (PropOwner::CONNACK, MqttPropValueType::TwoBytesInt, false)
            }
            Property::AuthenticationMethod => (
                PropOwner::CONNECT | PropOwner::CONNACK | PropOwner::AUTH,
                MqttPropValueType::String,
                false,
            ),
            Property::AuthenticationData => (
                PropOwner::CONNECT | PropOwner::CONNACK | PropOwner::AUTH,
                MqttPropValueType::Data,
                false,
            ),
            Property::RequestProblemInformation => {
                (PropOwner::CONNECT, MqttPropValueType::Bool, false)
            }
            Property::WillDelayInterval => {
                (PropOwner::WILL, MqttPropValueType::FourBytesInt, false)
            }
            Property::RequestResponseInformation => {
                (PropOwner::CONNECT, MqttPropValueType::Bool, false)
            }
            Property::ResponseInformation => (PropOwner::CONNACK, MqttPropValueType::String, false),
            Property::ServerReference => (
                PropOwner::CONNACK | PropOwner::DISCONNECT,
                MqttPropValueType::String,
                false,
            ),
            Property::ReasonString => (
                PropOwner::CONNACK
                    | PropOwner::PUBACK
                    | PropOwner::PUBREC
                    | PropOwner::PUBREL
                    | PropOwner::PUBCOMP
                    | PropOwner::SUBACK
                    | PropOwner::UNSUBACK
                    | PropOwner::DISCONNECT
                    | PropOwner::AUTH,
                MqttPropValueType::String,
                false,
            ),
            Property::ReceiveMaximum => (
                PropOwner::CONNECT | PropOwner::CONNACK,
                MqttPropValueType::TwoBytesInt,
                false,
            ),
            Property::TopicAliasMaximum => (
                PropOwner::CONNECT | PropOwner::CONNACK,
                MqttPropValueType::TwoBytesInt,
                false,
            ),
            Property::TopicAlias => (PropOwner::PUBLISH, MqttPropValueType::TwoBytesInt, false),
            Property::MaximumQoS => (PropOwner::CONNACK, MqttPropValueType::Byte, false),
            Property::RetainAvailable => (PropOwner::CONNACK, MqttPropValueType::Byte, false),
            Property::UserProperty => (
                PropOwner::CONNECT
                    | PropOwner::CONNACK
                    | PropOwner::PUBLISH
                    | PropOwner::WILL
                    | PropOwner::PUBACK
                    | PropOwner::PUBREC
                    | PropOwner::PUBREL
                    | PropOwner::PUBCOMP
                    | PropOwner::SUBSCRIBE
                    | PropOwner::SUBACK
                    | PropOwner::UNSUBSCRIBE
                    | PropOwner::UNSUBACK
                    | PropOwner::DISCONNECT
                    | PropOwner::AUTH,
                MqttPropValueType::String,
                true,
            ),
            Property::MaximumPacketSize => (
                PropOwner::CONNECT | PropOwner::CONNACK,
                MqttPropValueType::FourBytesInt,
                false,
            ),
            Property::WildcardSubscriptionAvailable => {
                (PropOwner::CONNACK, MqttPropValueType::Bool, false)
            }
            Property::SubscriptionIdentifierAvailable => {
                (PropOwner::CONNACK, MqttPropValueType::Bool, false)
            }
            Property::SharedSubscriptionAvailable => {
                (PropOwner::CONNACK, MqttPropValueType::Bool, false)
            }
        }
    }
}

impl Parsable for Property {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let i = MqttVariableBytesInt::new(*self as u32)?;
        i.serialize(buf)
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let i = MqttVariableBytesInt::deserialize(buf)?.inner();
        match i {
            0x1 => Ok(Property::PayloadFormatIndicator),
            0x2 => Ok(Property::MessageExpiryInterval),
            0x3 => Ok(Property::ContentType),
            0x8 => Ok(Property::ResponseTopic),
            0x9 => Ok(Property::CorrelationData),
            0xb => Ok(Property::SubscriptionIdentifier),
            0x11 => Ok(Property::SessionExpiryInterval),
            0x12 => Ok(Property::AssignedClientIdentifier),
            0x13 => Ok(Property::ServerKeepAlive),
            0x15 => Ok(Property::AuthenticationMethod),
            0x16 => Ok(Property::AuthenticationData),
            0x17 => Ok(Property::RequestProblemInformation),
            0x18 => Ok(Property::WillDelayInterval),
            0x19 => Ok(Property::RequestResponseInformation),
            0x1a => Ok(Property::ResponseInformation),
            0x1c => Ok(Property::ServerReference),
            0x1f => Ok(Property::ReasonString),
            0x21 => Ok(Property::ReceiveMaximum),
            0x22 => Ok(Property::TopicAliasMaximum),
            0x23 => Ok(Property::TopicAlias),
            0x24 => Ok(Property::MaximumQoS),
            0x25 => Ok(Property::RetainAvailable),
            0x26 => Ok(Property::UserProperty),
            0x27 => Ok(Property::MaximumPacketSize),
            0x28 => Ok(Property::WildcardSubscriptionAvailable),
            0x29 => Ok(Property::SubscriptionIdentifierAvailable),
            0x2a => Ok(Property::SharedSubscriptionAvailable),
            _ => Err(DataParseError::BadProperty),
        }
    }
    fn size(&self) -> usize {
        // unwrap here is justified because all the values are hardcoded
        // if error may only trigger incase of stupid typo
        MqttVariableBytesInt::new(*self as u32).unwrap().size()
    }
}

#[derive(Clone)]
enum MqttPropValueInner {
    Bool(MqttOneBytesInt),
    Byte(MqttOneBytesInt),
    FourBytesInt(MqttFourBytesInt),
    String(MqttUtf8String),
    StringPair(MqttUtf8StringPair),
    Data(MqttBinaryData),
    VarInt(MqttVariableBytesInt),
    TwoBytesInt(MqttTwoBytesInt),
}

#[derive(Clone)]
pub struct MqttPropValue(MqttPropValueInner);
impl MqttPropValue {
    pub fn into_bool(&self) -> Option<bool> {
        if let MqttPropValueInner::Byte(i) = &self.0 {
            Some(i.inner() == 1)
        } else {
            None
        }
    }
    pub fn into_u8(&self) -> Option<u8> {
        if let MqttPropValueInner::Byte(i) = &self.0 {
            Some(i.inner())
        } else {
            None
        }
    }
    pub fn into_str(&self) -> Option<&str> {
        if let MqttPropValueInner::String(s) = &self.0 {
            Some(s.inner())
        } else {
            None
        }
    }
    pub fn into_str_pair(&self) -> Option<(&Arc<str>, &Arc<str>)> {
        if let MqttPropValueInner::StringPair(d) = &self.0 {
            Some(d.inner())
        } else {
            None
        }
    }
    pub fn into_data(&self) -> Option<&Bytes> {
        if let MqttPropValueInner::Data(d) = &self.0 {
            Some(d.inner())
        } else {
            None
        }
    }

    pub fn into_u16(&self) -> Option<u16> {
        if let MqttPropValueInner::TwoBytesInt(i) = &self.0 {
            Some(i.inner())
        } else {
            None
        }
    }
    pub fn into_u32(&self) -> Option<u32> {
        match &self.0 {
            MqttPropValueInner::VarInt(i) => Some(i.inner()),
            MqttPropValueInner::FourBytesInt(i) => Some(i.inner()),
            _ => None,
        }
    }
    pub fn new_bool(b: bool) -> MqttPropValue {
        MqttPropValue(MqttPropValueInner::Bool(MqttOneBytesInt::new(if b {
            1
        } else {
            0
        })))
    }
    pub fn new_u8(u: u8) -> MqttPropValue {
        MqttPropValue(MqttPropValueInner::Byte(MqttOneBytesInt::new(u)))
    }
    pub fn new_u32(u: u32) -> MqttPropValue {
        MqttPropValue(MqttPropValueInner::FourBytesInt(MqttFourBytesInt::new(u)))
    }
    pub fn new_string(buf: Arc<str>) -> Result<MqttPropValue, DataParseError> {
        Ok(MqttPropValue(MqttPropValueInner::String(
            MqttUtf8String::new(buf)?,
        )))
    }
    pub fn new_string_pair(k: Arc<str>, v: Arc<str>) -> Result<MqttPropValue, DataParseError> {
        Ok(MqttPropValue(MqttPropValueInner::StringPair(
            MqttUtf8StringPair::new(k, v)?,
        )))
    }
    pub fn new_data<T: Buf>(buf: T) -> Result<MqttPropValue, DataParseError> {
        Ok(MqttPropValue(MqttPropValueInner::Data(
            MqttBinaryData::new(buf)?,
        )))
    }
    pub fn new_varint(u: u32) -> Result<MqttPropValue, DataParseError> {
        Ok(MqttPropValue(MqttPropValueInner::VarInt(
            MqttVariableBytesInt::new(u)?,
        )))
    }
    pub fn new_u16(u: u16) -> MqttPropValue {
        MqttPropValue(MqttPropValueInner::TwoBytesInt(MqttTwoBytesInt::new(u)))
    }
    fn prop_type(&self) -> MqttPropValueType {
        match &self.0 {
            MqttPropValueInner::Bool(_) => MqttPropValueType::Bool,
            MqttPropValueInner::Byte(_) => MqttPropValueType::Byte,
            MqttPropValueInner::FourBytesInt(_) => MqttPropValueType::FourBytesInt,
            MqttPropValueInner::String(_) => MqttPropValueType::String,
            MqttPropValueInner::StringPair(_) => MqttPropValueType::StringPair,
            MqttPropValueInner::Data(_) => MqttPropValueType::Data,
            MqttPropValueInner::VarInt(_) => MqttPropValueType::VarInt,
            MqttPropValueInner::TwoBytesInt(_) => MqttPropValueType::TwoBytesInt,
        }
    }
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        match &self.0 {
            MqttPropValueInner::Bool(v) => {
                v.serialize(buf);
                Ok(())
            }
            MqttPropValueInner::Byte(v) => {
                v.serialize(buf);
                Ok(())
            }
            MqttPropValueInner::FourBytesInt(v) => {
                v.serialize(buf);
                Ok(())
            }
            MqttPropValueInner::String(v) => {
                v.serialize(buf);
                Ok(())
            }
            MqttPropValueInner::StringPair(v) => v.serialize(buf),
            MqttPropValueInner::Data(v) => v.serialize(buf),
            MqttPropValueInner::VarInt(v) => v.serialize(buf),
            MqttPropValueInner::TwoBytesInt(v) => {
                v.serialize(buf);
                Ok(())
            }
        }
    }
    fn deserialize<T: Buf>(buf: &mut T, ty: MqttPropValueType) -> Result<Self, DataParseError> {
        let res = match ty {
            MqttPropValueType::Bool => MqttPropValueInner::Bool(MqttOneBytesInt::deserialize(buf)?),
            MqttPropValueType::Byte => MqttPropValueInner::Byte(MqttOneBytesInt::deserialize(buf)?),
            MqttPropValueType::FourBytesInt => {
                MqttPropValueInner::FourBytesInt(MqttFourBytesInt::deserialize(buf)?)
            }
            MqttPropValueType::String => {
                MqttPropValueInner::String(MqttUtf8String::deserialize(buf)?)
            }
            MqttPropValueType::StringPair => {
                MqttPropValueInner::StringPair(MqttUtf8StringPair::deserialize(buf)?)
            }
            MqttPropValueType::Data => MqttPropValueInner::Data(MqttBinaryData::deserialize(buf)?),
            MqttPropValueType::VarInt => {
                MqttPropValueInner::VarInt(MqttVariableBytesInt::deserialize(buf)?)
            }
            MqttPropValueType::TwoBytesInt => {
                MqttPropValueInner::TwoBytesInt(MqttTwoBytesInt::deserialize(buf)?)
            }
        };
        Ok(MqttPropValue(res))
    }
    fn size(&self) -> usize {
        match &self.0 {
            MqttPropValueInner::Bool(_) => 1,
            MqttPropValueInner::Byte(_) => 1,
            MqttPropValueInner::FourBytesInt(v) => v.size(),
            MqttPropValueInner::String(v) => v.size(),
            MqttPropValueInner::StringPair(v) => v.size(),
            MqttPropValueInner::Data(v) => v.size(),
            MqttPropValueInner::VarInt(v) => v.size(),
            MqttPropValueInner::TwoBytesInt(v) => v.size(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bytes::{Bytes, BytesMut};

    #[test]
    #[cfg(feature = "debug")]
    fn test_failing_props() {
        let mut props = Properties::new();
        let res = props
            .checked_insert(
                Property::ReasonString,
                MqttPropValue::new_u8(5),
                PropOwner::CONNACK,
            )
            .err()
            .unwrap();
        assert_eq!(res, DataParseError::BadProperty);
        let res = props
            .checked_insert(
                Property::ReasonString,
                MqttPropValue::new_string(Arc::from("Hello")).unwrap(),
                PropOwner::CONNECT,
            )
            .err()
            .unwrap();
        assert_eq!(res, DataParseError::BadProperty);
        let mut b = BytesMut::new();
        props.serialize(&mut b).unwrap();
        assert_eq!(b, &[0x0][..]);
    }

    #[test]
    fn test_props() {
        let mut props = Properties::new();
        props
            .checked_insert(
                Property::ReasonString,
                MqttPropValue::new_string(Arc::from("Hello")).unwrap(),
                PropOwner::CONNACK,
            )
            .unwrap();
        let mut b = BytesMut::new();
        props.serialize(&mut b).unwrap();
        assert_eq!(
            b,
            &[0x08, 0x1f, 0x00, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f][..]
        );
        assert_eq!(b.remaining(), props.size());

        let props2 = Properties::deserialize(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        props2.serialize(&mut b2).unwrap();
        assert_eq!(b, b2);
    }

    #[test]
    fn test_props_no_duplicate_insert() {
        let mut props = Properties::new();
        props
            .checked_insert(
                Property::ReasonString,
                MqttPropValue::new_string(Arc::from("Hello")).unwrap(),
                PropOwner::CONNACK,
            )
            .unwrap();
        let mut b = BytesMut::new();
        props.serialize(&mut b).unwrap();
        assert_eq!(
            b,
            &[0x08, 0x1f, 0x00, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f][..]
        );
        assert_eq!(b.remaining(), props.size());

        props
            .checked_insert(
                Property::ReasonString,
                MqttPropValue::new_string(Arc::from("World")).unwrap(),
                PropOwner::CONNACK,
            )
            .unwrap();
        let mut b = BytesMut::new();
        props.serialize(&mut b).unwrap();
        assert_eq!(
            b,
            &[0x08, 0x1f, 0x00, 0x05, 0x57, 0x6f, 0x72, 0x6c, 0x64][..]
        );
        assert_eq!(b.remaining(), props.size());
    }
    #[test]
    fn test_props_truncated() {
        let mut b = Bytes::from(
            &[
                0x09, 0x1f, 0x00, 0x05, 0x57, 0x6f, 0x72, 0x6c, 0x64, 0x1f, 0x00, 0x05, 0x48, 0x65,
                0x6c, 0x6c, 0x6f,
            ][..],
        );
        assert_eq!(
            Properties::deserialize(&mut b).err().unwrap(),
            DataParseError::InsufficientBuffer {
                needed: 2,
                available: 0
            }
        );
    }
    #[test]
    fn test_props_no_duplicate_deserialize() {
        let mut b = Bytes::from(
            &[
                0x10, 0x1f, 0x00, 0x05, 0x57, 0x6f, 0x72, 0x6c, 0x64, 0x1f, 0x00, 0x05, 0x48, 0x65,
                0x6c, 0x6c, 0x6f,
            ][..],
        );
        assert_eq!(
            Properties::deserialize(&mut b).err().unwrap(),
            DataParseError::BadProperty
        );
    }

    #[test]
    fn test_props_duplicate() {
        let mut props = Properties::new();
        props
            .checked_insert(
                Property::UserProperty,
                MqttPropValue::new_string(Arc::from("Hello")).unwrap(),
                PropOwner::CONNACK,
            )
            .unwrap();
        props
            .checked_insert(
                Property::UserProperty,
                MqttPropValue::new_string(Arc::from("World")).unwrap(),
                PropOwner::CONNACK,
            )
            .unwrap();
        let mut b = BytesMut::new();
        props.serialize(&mut b).unwrap();
        assert_eq!(
            b,
            &[
                0x10, 0x26, 0x00, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x26, 0x00, 0x05, 0x57, 0x6f,
                0x72, 0x6c, 0x64
            ][..]
        );
        assert_eq!(b.remaining(), props.size());
        let props2 = Properties::deserialize(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        props2.serialize(&mut b2).unwrap();
        assert_eq!(b, b2);
    }
}
