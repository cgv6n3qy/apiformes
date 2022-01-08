use super::{data::MqttOneBytesInt, error::DataParseError, parsable::*};
use bytes::{Buf, BufMut};

//2.4 Reason Code
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum ConnAckReasonCode {
    Success = 0x0,
    UnspecifiedError = 0x80,
    MalformedPacket = 0x81,
    ProtocolError = 0x82,
    ImplementationSpecificError = 0x83,
    UnsupportedProtocolVersion = 0x84,
    ClientIdentifierNotValid = 0x85,
    BadUserNameOrPassword = 0x86,
    NotAuthorized = 0x87,
    ServerUnavailable = 0x88,
    ServerBusy = 0x89,
    Banned = 0x8a,
    BadAuthenicationMethod = 0x8c,
    TopicNameInvalid = 0x90,
    PacketTooLarge = 0x95,
    QuotaExceeded = 0x97,
    PayloadFormatInvalid = 0x99,
    RetainNotSupported = 0x9a,
    QoSNotSupported = 0x9b,
    UseAnotherServer = 0x9c,
    ServerMoved = 0x9d,
    ConnectionRateExceeded = 0x9f,
}

impl Parsable for ConnAckReasonCode {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let b = MqttOneBytesInt::new(*self as u8);
        b.serialize(buf);
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let b = MqttOneBytesInt::deserialize(buf)?;
        match b.inner() {
            0x0 => Ok(ConnAckReasonCode::Success),
            0x80 => Ok(ConnAckReasonCode::UnspecifiedError),
            0x81 => Ok(ConnAckReasonCode::MalformedPacket),
            0x82 => Ok(ConnAckReasonCode::ProtocolError),
            0x83 => Ok(ConnAckReasonCode::ImplementationSpecificError),
            0x84 => Ok(ConnAckReasonCode::UnsupportedProtocolVersion),
            0x85 => Ok(ConnAckReasonCode::ClientIdentifierNotValid),
            0x86 => Ok(ConnAckReasonCode::BadUserNameOrPassword),
            0x87 => Ok(ConnAckReasonCode::NotAuthorized),
            0x88 => Ok(ConnAckReasonCode::ServerUnavailable),
            0x89 => Ok(ConnAckReasonCode::ServerBusy),
            0x8a => Ok(ConnAckReasonCode::Banned),
            0x8c => Ok(ConnAckReasonCode::BadAuthenicationMethod),
            0x90 => Ok(ConnAckReasonCode::TopicNameInvalid),
            0x95 => Ok(ConnAckReasonCode::PacketTooLarge),
            0x97 => Ok(ConnAckReasonCode::QuotaExceeded),
            0x99 => Ok(ConnAckReasonCode::PayloadFormatInvalid),
            0x9a => Ok(ConnAckReasonCode::RetainNotSupported),
            0x9b => Ok(ConnAckReasonCode::QoSNotSupported),
            0x9c => Ok(ConnAckReasonCode::UseAnotherServer),
            0x9d => Ok(ConnAckReasonCode::ServerMoved),
            0x9f => Ok(ConnAckReasonCode::ConnectionRateExceeded),
            _ => Err(DataParseError::BadReasonCode),
        }
    }
    fn size(&self) -> usize {
        1
    }
}

//2.4 Reason Code
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum PubAckReasonCode {
    Success = 0x0,
    NoMatchingSubscribers = 0x10,
    UnspecifiedError = 0x80,
    ImplementationSpecificError = 0x83,
    NotAuthorized = 0x87,
    TopicNameInvalid = 0x90,
    PacketIdentifierInUse = 0x91,
    QuotaExceeded = 0x97,
    PayloadFormatInvalid = 0x99,
}

impl Parsable for PubAckReasonCode {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let b = MqttOneBytesInt::new(*self as u8);
        b.serialize(buf);
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let b = MqttOneBytesInt::deserialize(buf)?;
        match b.inner() {
            0x0 => Ok(PubAckReasonCode::Success),
            0x10 => Ok(PubAckReasonCode::NoMatchingSubscribers),
            0x80 => Ok(PubAckReasonCode::UnspecifiedError),
            0x83 => Ok(PubAckReasonCode::ImplementationSpecificError),
            0x87 => Ok(PubAckReasonCode::NotAuthorized),
            0x90 => Ok(PubAckReasonCode::TopicNameInvalid),
            0x91 => Ok(PubAckReasonCode::PacketIdentifierInUse),
            0x97 => Ok(PubAckReasonCode::QuotaExceeded),
            0x99 => Ok(PubAckReasonCode::PayloadFormatInvalid),
            _ => Err(DataParseError::BadReasonCode),
        }
    }
    fn size(&self) -> usize {
        1
    }
}
//2.4 Reason Code
pub type PubRecReasonCode = PubAckReasonCode;

//2.4 Reason Code
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum PubRelReasonCode {
    Success = 0x0,
    PacketIdentifierNotFound = 0x92,
}

impl Parsable for PubRelReasonCode {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let b = MqttOneBytesInt::new(*self as u8);
        b.serialize(buf);
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let b = MqttOneBytesInt::deserialize(buf)?;
        match b.inner() {
            0x0 => Ok(PubRelReasonCode::Success),
            0x92 => Ok(PubRelReasonCode::PacketIdentifierNotFound),
            _ => Err(DataParseError::BadReasonCode),
        }
    }
    fn size(&self) -> usize {
        1
    }
}

//2.4 Reason Code
pub type PubCompReasonCode = PubRelReasonCode;

//2.4 Reason Code
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum UnsubAckReasonCode {
    Success = 0x0,
    NoSubscriptionExisted = 0x11,
    UnspecifiedError = 0x80,
    ImplementationSpecificError = 0x83,
    NotAuthorized = 0x87,
    TopicFilterInvalid = 0x8f,
    PacketIdentifierInUse = 0x91,
}

impl Parsable for UnsubAckReasonCode {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let b = MqttOneBytesInt::new(*self as u8);
        b.serialize(buf);
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let b = MqttOneBytesInt::deserialize(buf)?;
        match b.inner() {
            0x0 => Ok(UnsubAckReasonCode::Success),
            0x11 => Ok(UnsubAckReasonCode::NoSubscriptionExisted),
            0x80 => Ok(UnsubAckReasonCode::UnspecifiedError),
            0x83 => Ok(UnsubAckReasonCode::ImplementationSpecificError),
            0x87 => Ok(UnsubAckReasonCode::NotAuthorized),
            0x8f => Ok(UnsubAckReasonCode::TopicFilterInvalid),
            0x91 => Ok(UnsubAckReasonCode::PacketIdentifierInUse),
            _ => Err(DataParseError::BadReasonCode),
        }
    }
    fn size(&self) -> usize {
        1
    }
}

//2.4 Reason Code
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum AuthReasonCode {
    Success = 0x0,
    ContinueAuthentication = 0x18,
    ReAuthenticate = 0x19,
}

impl Parsable for AuthReasonCode {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let b = MqttOneBytesInt::new(*self as u8);
        b.serialize(buf);
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let b = MqttOneBytesInt::deserialize(buf)?;
        match b.inner() {
            0x0 => Ok(AuthReasonCode::Success),
            0x18 => Ok(AuthReasonCode::ContinueAuthentication),
            0x19 => Ok(AuthReasonCode::ReAuthenticate),
            _ => Err(DataParseError::BadReasonCode),
        }
    }
    fn size(&self) -> usize {
        1
    }
}

//2.4 Reason Code
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum DisconnectReasonCode {
    NormalDisconnection = 0x0,
    DisconnectWithWillMessage = 0x04,
    UnspecifiedError = 0x80,
    MalformedPacket = 0x81,
    ProtocolError = 0x82,
    ImplementationSpecificError = 0x83,
    NotAuthorized = 0x87,
    ServerBusy = 0x89,
    ServerShuttingDown = 0x8b,
    BadAuthenicationMethod = 0x8c,
    KeepAliveTimeout = 0x8d,
    SessionTakenOver = 0x8e,
    TopicFilterInvalid = 0x8f,
    TopicNameInvalid = 0x90,
    ReceiveMaximumExceeded = 0x93,
    TopicAliasInvalid = 0x94,
    PacketTooLarge = 0x95,
    MessageRateTooHigh = 0x96,
    QuotaExceeded = 0x97,
    AdministrativeAction = 0x98,
    PayloadFormatInvalid = 0x99,
    RetainNotSupported = 0x9a,
    QoSNotSupported = 0x9b,
    UseAnotherServer = 0x9c,
    ServerMoved = 0x9d,
    SharedSubscriptionsNotSupported = 0x9e,
    ConnectionRateExceeded = 0x9f,
    MaximumConnectTime = 0xa0,
    SubscriptionIdentifiersNotSupported = 0xa1,
    WildcardSubscriptionsNotSupported = 0xa2,
}

impl Parsable for DisconnectReasonCode {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let b = MqttOneBytesInt::new(*self as u8);
        b.serialize(buf);
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let b = MqttOneBytesInt::deserialize(buf)?;
        match b.inner() {
            0x0 => Ok(DisconnectReasonCode::NormalDisconnection),
            0x04 => Ok(DisconnectReasonCode::DisconnectWithWillMessage),
            0x80 => Ok(DisconnectReasonCode::UnspecifiedError),
            0x81 => Ok(DisconnectReasonCode::MalformedPacket),
            0x82 => Ok(DisconnectReasonCode::ProtocolError),
            0x83 => Ok(DisconnectReasonCode::ImplementationSpecificError),
            0x87 => Ok(DisconnectReasonCode::NotAuthorized),
            0x89 => Ok(DisconnectReasonCode::ServerBusy),
            0x8b => Ok(DisconnectReasonCode::ServerShuttingDown),
            0x8c => Ok(DisconnectReasonCode::BadAuthenicationMethod),
            0x8d => Ok(DisconnectReasonCode::KeepAliveTimeout),
            0x8e => Ok(DisconnectReasonCode::SessionTakenOver),
            0x8f => Ok(DisconnectReasonCode::TopicFilterInvalid),
            0x90 => Ok(DisconnectReasonCode::TopicNameInvalid),
            0x93 => Ok(DisconnectReasonCode::ReceiveMaximumExceeded),
            0x94 => Ok(DisconnectReasonCode::TopicAliasInvalid),
            0x95 => Ok(DisconnectReasonCode::PacketTooLarge),
            0x96 => Ok(DisconnectReasonCode::MessageRateTooHigh),
            0x97 => Ok(DisconnectReasonCode::QuotaExceeded),
            0x98 => Ok(DisconnectReasonCode::AdministrativeAction),
            0x99 => Ok(DisconnectReasonCode::PayloadFormatInvalid),
            0x9a => Ok(DisconnectReasonCode::RetainNotSupported),
            0x9b => Ok(DisconnectReasonCode::QoSNotSupported),
            0x9c => Ok(DisconnectReasonCode::UseAnotherServer),
            0x9d => Ok(DisconnectReasonCode::ServerMoved),
            0x9e => Ok(DisconnectReasonCode::SharedSubscriptionsNotSupported),
            0x9f => Ok(DisconnectReasonCode::ConnectionRateExceeded),
            0xa0 => Ok(DisconnectReasonCode::MaximumConnectTime),
            0xa1 => Ok(DisconnectReasonCode::SubscriptionIdentifiersNotSupported),
            0xa2 => Ok(DisconnectReasonCode::WildcardSubscriptionsNotSupported),
            _ => Err(DataParseError::BadReasonCode),
        }
    }
    fn size(&self) -> usize {
        1
    }
}

//2.4 Reason Code
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum SubAckReasonCode {
    GrantedQoS0 = 0x0,
    GrantedQoS1 = 0x1,
    GrantedQoS2 = 0x2,
    UnspecifiedError = 0x80,
    ImplementationSpecificError = 0x83,
    NotAuthorized = 0x87,
    TopicFilterInvalid = 0x8f,
    PacketIdentifierInUse = 0x91,
    QuotaExceeded = 0x97,
    SharedSubscriptionsNotSupported = 0x9e,
    SubscriptionIdentifiersNotSupported = 0xa1,
    WildcardSubscriptionsNotSupported = 0xa2,
}

impl Parsable for SubAckReasonCode {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let b = MqttOneBytesInt::new(*self as u8);
        b.serialize(buf);
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let b = MqttOneBytesInt::deserialize(buf)?;
        match b.inner() {
            0x0 => Ok(SubAckReasonCode::GrantedQoS0),
            0x1 => Ok(SubAckReasonCode::GrantedQoS1),
            0x2 => Ok(SubAckReasonCode::GrantedQoS2),
            0x80 => Ok(SubAckReasonCode::UnspecifiedError),
            0x83 => Ok(SubAckReasonCode::ImplementationSpecificError),
            0x87 => Ok(SubAckReasonCode::NotAuthorized),
            0x8f => Ok(SubAckReasonCode::TopicFilterInvalid),
            0x91 => Ok(SubAckReasonCode::PacketIdentifierInUse),
            0x97 => Ok(SubAckReasonCode::QuotaExceeded),
            0x9e => Ok(SubAckReasonCode::SharedSubscriptionsNotSupported),
            0xa1 => Ok(SubAckReasonCode::SubscriptionIdentifiersNotSupported),
            0xa2 => Ok(SubAckReasonCode::WildcardSubscriptionsNotSupported),
            _ => Err(DataParseError::BadReasonCode),
        }
    }
    fn size(&self) -> usize {
        1
    }
}
