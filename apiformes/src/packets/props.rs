use crate::data::{
    MqttBinaryData, MqttFourBytesInt, MqttOneBytesInt, MqttTwoBytesInt, MqttUtf8String,
    MqttUtf8StringPair, MqttVariableBytesInt,
};
use crate::parsable::{DataParseError, Parsable};
use bitflags::bitflags;
use bytes::{Buf, BufMut};
use std::collections::{hash_map::Iter, HashMap};
bitflags! {
    pub struct ValidMessage: u16 {
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

pub struct Properties {
    size: usize,
    valid: ValidMessage,
    props: HashMap<Property, MqttPropValue>,
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
            valid: ValidMessage::ALL_MESSAGES,
            props: HashMap::new(),
        }
    }
    pub fn insert(&mut self, key: Property, value: MqttPropValue) -> Result<(), &'static str> {
        let (filter, prop_type) = key.auxiliary_data();
        if value.prop_type() != prop_type {
            return Err("Invalid value type for given key");
        }
        self.size += value.size();
        self.valid &= filter;
        let old = self.props.insert(key, value);
        if let Some(v) = old {
            self.size -= v.size();
        }
        Ok(())
    }
    pub fn get(&self, key: Property) -> Option<&MqttPropValue> {
        self.props.get(&key)
    }
    pub fn is_valid_for(&self, message: ValidMessage) -> bool {
        self.valid & message == message
    }
    pub fn iter(&self) -> Iter<'_, Property, MqttPropValue> {
        self.props.iter()
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
        while size != 0 {
            let key = Property::deserialize(buf)?;
            let (_, ty) = key.auxiliary_data();
            let value = MqttPropValue::deserialize(buf, ty)?;
            size -= key.size() + value.size();
            // the unwrap is justified here because we just checked that the
            // deserialization type is based on auxiliary data
            table.insert(key, value).unwrap();
        }
        Ok(table)
    }
    fn size(&self) -> usize {
        self.size
    }
}

#[derive(PartialEq)]
pub enum MqttPropValueType {
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
    fn auxiliary_data(&self) -> (ValidMessage, MqttPropValueType) {
        match self {
            Property::PayloadFormatIndicator => (
                ValidMessage::PUBLISH | ValidMessage::WILL,
                MqttPropValueType::Byte,
            ),
            Property::MessageExpiryInterval => (
                ValidMessage::PUBLISH | ValidMessage::WILL,
                MqttPropValueType::FourBytesInt,
            ),
            Property::ContentType => (
                ValidMessage::PUBLISH | ValidMessage::WILL,
                MqttPropValueType::String,
            ),
            Property::ResponseTopic => (
                ValidMessage::PUBLISH | ValidMessage::WILL,
                MqttPropValueType::String,
            ),
            Property::CorrelationData => (
                ValidMessage::PUBLISH | ValidMessage::WILL,
                MqttPropValueType::Data,
            ),
            Property::SubscriptionIdentifier => (
                ValidMessage::PUBLISH | ValidMessage::SUBSCRIBE,
                MqttPropValueType::VarInt,
            ),
            Property::SessionExpiryInterval => (
                ValidMessage::CONNECT | ValidMessage::CONNACK | ValidMessage::DISCONNECT,
                MqttPropValueType::FourBytesInt,
            ),
            Property::AssignedClientIdentifier => {
                (ValidMessage::CONNACK, MqttPropValueType::String)
            }
            Property::ServerKeepAlive => (ValidMessage::CONNACK, MqttPropValueType::TwoBytesInt),
            Property::AuthenticationMethod => (
                ValidMessage::CONNECT | ValidMessage::CONNACK | ValidMessage::AUTH,
                MqttPropValueType::String,
            ),
            Property::AuthenticationData => (
                ValidMessage::CONNECT | ValidMessage::CONNACK | ValidMessage::AUTH,
                MqttPropValueType::Data,
            ),
            Property::RequestProblemInformation => (ValidMessage::CONNECT, MqttPropValueType::Byte),
            Property::WillDelayInterval => (ValidMessage::WILL, MqttPropValueType::FourBytesInt),
            Property::RequestResponseInformation => {
                (ValidMessage::CONNECT, MqttPropValueType::Byte)
            }
            Property::ResponseInformation => (ValidMessage::CONNACK, MqttPropValueType::String),
            Property::ServerReference => (
                ValidMessage::CONNACK | ValidMessage::DISCONNECT,
                MqttPropValueType::String,
            ),
            Property::ReasonString => (
                ValidMessage::CONNACK
                    | ValidMessage::PUBACK
                    | ValidMessage::PUBREC
                    | ValidMessage::PUBREL
                    | ValidMessage::PUBCOMP
                    | ValidMessage::SUBACK
                    | ValidMessage::UNSUBACK
                    | ValidMessage::DISCONNECT
                    | ValidMessage::AUTH,
                MqttPropValueType::String,
            ),
            Property::ReceiveMaximum => (
                ValidMessage::CONNECT | ValidMessage::CONNACK,
                MqttPropValueType::TwoBytesInt,
            ),
            Property::TopicAliasMaximum => (
                ValidMessage::CONNECT | ValidMessage::CONNACK,
                MqttPropValueType::TwoBytesInt,
            ),
            Property::TopicAlias => (ValidMessage::PUBLISH, MqttPropValueType::TwoBytesInt),
            Property::MaximumQoS => (ValidMessage::CONNACK, MqttPropValueType::Byte),
            Property::RetainAvailable => (ValidMessage::CONNACK, MqttPropValueType::Byte),
            Property::UserProperty => (
                ValidMessage::CONNECT
                    | ValidMessage::CONNACK
                    | ValidMessage::PUBLISH
                    | ValidMessage::WILL
                    | ValidMessage::PUBACK
                    | ValidMessage::PUBREC
                    | ValidMessage::PUBREL
                    | ValidMessage::PUBCOMP
                    | ValidMessage::SUBSCRIBE
                    | ValidMessage::SUBACK
                    | ValidMessage::UNSUBSCRIBE
                    | ValidMessage::UNSUBACK
                    | ValidMessage::DISCONNECT
                    | ValidMessage::AUTH,
                MqttPropValueType::String,
            ),
            Property::MaximumPacketSize => (
                ValidMessage::CONNECT | ValidMessage::CONNACK,
                MqttPropValueType::FourBytesInt,
            ),
            Property::WildcardSubscriptionAvailable => {
                (ValidMessage::CONNACK, MqttPropValueType::Byte)
            }
            Property::SubscriptionIdentifierAvailable => {
                (ValidMessage::CONNACK, MqttPropValueType::Byte)
            }
            Property::SharedSubscriptionAvailable => {
                (ValidMessage::CONNACK, MqttPropValueType::Byte)
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

pub enum MqttPropValue {
    Byte(MqttOneBytesInt),
    FourBytesInt(MqttFourBytesInt),
    String(MqttUtf8String),
    StringPair(MqttUtf8StringPair),
    Data(MqttBinaryData),
    VarInt(MqttVariableBytesInt),
    TwoBytesInt(MqttTwoBytesInt),
}

impl MqttPropValue {
    fn prop_type(&self) -> MqttPropValueType {
        match self {
            MqttPropValue::Byte(_) => MqttPropValueType::Byte,
            MqttPropValue::FourBytesInt(_) => MqttPropValueType::FourBytesInt,
            MqttPropValue::String(_) => MqttPropValueType::String,
            MqttPropValue::StringPair(_) => MqttPropValueType::StringPair,
            MqttPropValue::Data(_) => MqttPropValueType::Data,
            MqttPropValue::VarInt(_) => MqttPropValueType::VarInt,
            MqttPropValue::TwoBytesInt(_) => MqttPropValueType::TwoBytesInt,
        }
    }
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        match self {
            MqttPropValue::Byte(v) => v.serialize(buf),
            MqttPropValue::FourBytesInt(v) => v.serialize(buf),
            MqttPropValue::String(v) => v.serialize(buf),
            MqttPropValue::StringPair(v) => v.serialize(buf),
            MqttPropValue::Data(v) => v.serialize(buf),
            MqttPropValue::VarInt(v) => v.serialize(buf),
            MqttPropValue::TwoBytesInt(v) => v.serialize(buf),
        }
    }
    fn deserialize<T: Buf>(buf: &mut T, ty: MqttPropValueType) -> Result<Self, DataParseError> {
        let res = match ty {
            MqttPropValueType::Byte => MqttPropValue::Byte(MqttOneBytesInt::deserialize(buf)?),
            MqttPropValueType::FourBytesInt => {
                MqttPropValue::FourBytesInt(MqttFourBytesInt::deserialize(buf)?)
            }
            MqttPropValueType::String => MqttPropValue::String(MqttUtf8String::deserialize(buf)?),
            MqttPropValueType::StringPair => {
                MqttPropValue::StringPair(MqttUtf8StringPair::deserialize(buf)?)
            }
            MqttPropValueType::Data => MqttPropValue::Data(MqttBinaryData::deserialize(buf)?),
            MqttPropValueType::VarInt => {
                MqttPropValue::VarInt(MqttVariableBytesInt::deserialize(buf)?)
            }
            MqttPropValueType::TwoBytesInt => {
                MqttPropValue::TwoBytesInt(MqttTwoBytesInt::deserialize(buf)?)
            }
        };
        Ok(res)
    }
    fn size(&self) -> usize {
        match self {
            MqttPropValue::Byte(_) => 1,
            MqttPropValue::FourBytesInt(v) => v.size(),
            MqttPropValue::String(v) => v.size(),
            MqttPropValue::StringPair(v) => v.size(),
            MqttPropValue::Data(v) => v.size(),
            MqttPropValue::VarInt(v) => v.size(),
            MqttPropValue::TwoBytesInt(v) => v.size(),
        }
    }
}
