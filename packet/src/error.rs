#[derive(Debug, PartialEq)]
pub enum DataParseError {
    InsufficientBuffer { needed: usize, available: usize },
    BadAuthMessage,
    BadMqttUtf8String,
    BadMqttVariableBytesInt,
    BadMqttBinaryData,
    BadPacketType,
    BadProperty,
    BadReasonCode,
    BadConnectMessage,
    BadDisconnectMessage,
    BadPubAckMessage,
    BadPubCompMessage,
    BadPublishMessage,
    BadPubRecMessage,
    BadPubRelMessage,
    UnsupportedMqttVersion,
    BadQoS,
    BadTopic,
    BadRetainHandle,
    BadSubscribeMessage,
    BadSubAckMessage,
    BadUnsubscribeMessage,
    BadUnsubAckMessage,
    BadPing,
    BadConnAckMessage,
}
