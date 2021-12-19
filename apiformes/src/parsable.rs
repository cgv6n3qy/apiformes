use bytes::{Buf, BufMut};

#[derive(Debug, PartialEq)]
pub enum DataParseError {
    InsufficientBuffer { needed: usize, available: usize },
    BadMqttUtf8String,
    BadMqttVariableBytesInt,
    BadMqttBinaryData,
    BadPacketType,
    BadProperty,
    BadReasonCode,
    BadConnectMessage,
    UnsupportedMqttVersion,
    BadQoS,
    BadTopic,
    BadRetainHandle,
    BadSubscribeMessage,
    BadSubAckMessage,
    BadUnSubscribeMessage,
    BadUnSubAckMessage,
}

pub trait UncheckedParsable {
    fn unchecked_serialize<T: BufMut>(&self, buf: &mut T);
    fn unchecked_deserialize<T: Buf>(buf: &mut T) -> Self;
}

pub trait Parsable {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError>;
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError>
    where
        Self: Sized;
    fn size(&self) -> usize;
}
