use super::{
    data::{MqttOneBytesInt, MqttTwoBytesInt, MqttVariableBytesInt},
    error::DataParseError,
    packet::Packet,
    parsable::*,
    props::{MqttPropValue, PropOwner, Properties, Property},
    qos::QoS,
    topic::MqttTopic,
};
use bitflags::bitflags;
use bytes::{Buf, BufMut};
use std::sync::Arc;

#[cfg_attr(feature = "debug", derive(Debug))]
#[repr(u8)]
#[derive(PartialEq)]
pub enum RetainHandling {
    Send = 0,
    SendIfNotExisting = 1,
    DoNotSend = 2,
}

impl From<RetainHandling> for SubscriptionOptions {
    fn from(r: RetainHandling) -> Self {
        match r {
            RetainHandling::Send => SubscriptionOptions::empty(),
            RetainHandling::SendIfNotExisting => SubscriptionOptions::RETAIN_HANDLING1,
            RetainHandling::DoNotSend => SubscriptionOptions::RETAIN_HANDLING2,
        }
    }
}

bitflags! {
    pub struct SubscriptionOptions: u8 {
        const QOS1                  = 0b0000_0001;
        const QOS2                  = 0b0000_0010;
        const NO_LOCAL              = 0b0000_0100;
        const RETAIN_AS_PUBLISHED   = 0b0000_1000;
        const RETAIN_HANDLING1      = 0b0001_0000;
        const RETAIN_HANDLING2      = 0b0010_0000;
        const NO_FLAGS              = 0x0000_0000;
    }
}

impl Parsable for SubscriptionOptions {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let flags = MqttOneBytesInt::new(self.bits());
        flags.serialize(buf);
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let raw_flags = MqttOneBytesInt::deserialize(buf)?;
        let flags = SubscriptionOptions::from_bits(raw_flags.inner())
            .ok_or(DataParseError::BadConnectMessage)?;
        // sanity check on qos
        let _: QoS = flags.try_into()?;
        // sanity check on retainhandling
        let _: RetainHandling = flags.try_into()?;
        Ok(flags)
    }
    fn size(&self) -> usize {
        1
    }
}

impl From<QoS> for SubscriptionOptions {
    fn from(q: QoS) -> Self {
        match q {
            QoS::QoS0 => SubscriptionOptions::empty(),
            QoS::QoS1 => SubscriptionOptions::QOS1,
            QoS::QoS2 => SubscriptionOptions::QOS2,
        }
    }
}

impl TryInto<RetainHandling> for SubscriptionOptions {
    type Error = DataParseError;
    fn try_into(self) -> Result<RetainHandling, Self::Error> {
        if self
            .contains(SubscriptionOptions::RETAIN_HANDLING1 | SubscriptionOptions::RETAIN_HANDLING2)
        {
            return Err(DataParseError::BadRetainHandle);
        }
        match self & (SubscriptionOptions::RETAIN_HANDLING1 | SubscriptionOptions::RETAIN_HANDLING2)
        {
            SubscriptionOptions::RETAIN_HANDLING1 => Ok(RetainHandling::SendIfNotExisting),
            SubscriptionOptions::RETAIN_HANDLING2 => Ok(RetainHandling::DoNotSend),
            SubscriptionOptions::NO_FLAGS => Ok(RetainHandling::Send),
            _ => unreachable!(),
        }
    }
}

impl TryInto<QoS> for SubscriptionOptions {
    type Error = DataParseError;
    fn try_into(self) -> Result<QoS, Self::Error> {
        if self.contains(SubscriptionOptions::QOS1 | SubscriptionOptions::QOS2) {
            return Err(DataParseError::BadQoS);
        }
        match self & (SubscriptionOptions::QOS1 | SubscriptionOptions::QOS2) {
            SubscriptionOptions::QOS1 => Ok(QoS::QoS1),
            SubscriptionOptions::QOS2 => Ok(QoS::QoS2),
            SubscriptionOptions::NO_FLAGS => Ok(QoS::QoS0),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone)]
pub struct Subscribe {
    // 2.2.1 Packet Identifier
    packet_identifier: MqttTwoBytesInt,
    // 3.8.2.1 SUBSCRIBE Properties
    props: Properties,
    /// 3.8.3 SUBSCRIBE Payload
    topics: Vec<(MqttTopic, SubscriptionOptions)>,
}

impl Subscribe {
    pub fn new(id: u16) -> Self {
        Subscribe {
            packet_identifier: MqttTwoBytesInt::new(id),
            props: Properties::new(),
            topics: Vec::new(),
        }
    }
    pub fn packet_identifier(&self) -> u16 {
        self.packet_identifier.inner()
    }
    pub fn topics_iter(&self) -> impl Iterator<Item = (&Arc<str>, &SubscriptionOptions)> {
        self.topics.iter().map(|(k, v)| (k.inner(), v))
    }

    pub fn add_topic(
        &mut self,
        topic: Arc<str>,
        options: SubscriptionOptions,
    ) -> Result<(), DataParseError> {
        let _: QoS = options.try_into()?;
        let _: RetainHandling = options.try_into()?;
        let topic = MqttTopic::new(topic)?;
        self.topics.push((topic, options));
        Ok(())
    }
    pub fn add_prop(&mut self, key: Property, value: MqttPropValue) -> Result<(), DataParseError> {
        self.props.checked_insert(key, value, PropOwner::SUBSCRIBE)
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
            + self
                .topics
                .iter()
                .map(|(k, v)| k.size() + v.size())
                .sum::<usize>()
    }
    pub fn build(self) -> Packet {
        Packet::Subscribe(self)
    }
}

impl Parsable for Subscribe {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let length = MqttVariableBytesInt::new(self.partial_size() as u32)?;
        length.serialize(buf)?;
        self.packet_identifier.serialize(buf)?;
        self.props.serialize(buf)?;
        for (k, v) in &self.topics {
            k.serialize(buf)?;
            v.serialize(buf)?;
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
        if !props.is_valid_for(PropOwner::SUBSCRIBE) {
            return Err(DataParseError::BadProperty);
        }
        let mut topics = Vec::new();
        while buf.remaining() != 0 {
            let topic = MqttTopic::deserialize(&mut buf)?;
            let options = SubscriptionOptions::deserialize(&mut buf)?;
            topics.push((topic, options));
        }
        if topics.is_empty() {
            Err(DataParseError::BadSubscribeMessage)
        } else {
            Ok(Subscribe {
                packet_identifier,
                props,
                topics,
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
    #[cfg(feature = "debug")]
    fn test_retain() {
        let r: RetainHandling = SubscriptionOptions::empty().try_into().unwrap();
        assert_eq!(r, RetainHandling::Send);
        let r: RetainHandling = SubscriptionOptions::empty().try_into().unwrap();
        assert_eq!(r, RetainHandling::Send);
        let r: RetainHandling = SubscriptionOptions::RETAIN_HANDLING1.try_into().unwrap();
        assert_eq!(r, RetainHandling::SendIfNotExisting);
        let r: RetainHandling = SubscriptionOptions::RETAIN_HANDLING2.try_into().unwrap();
        assert_eq!(r, RetainHandling::DoNotSend);
        let r: Result<RetainHandling, _> = (SubscriptionOptions::RETAIN_HANDLING1
            | SubscriptionOptions::RETAIN_HANDLING2)
            .try_into();
        assert_eq!(r.err().unwrap(), DataParseError::BadRetainHandle);
    }
    #[test]
    fn test_subscribe() {
        let mut subscribe = Subscribe::new(123);
        subscribe
            .add_topic(Arc::from("foo"), SubscriptionOptions::NO_LOCAL)
            .unwrap();
        subscribe
            .add_topic(
                Arc::from("bar"),
                SubscriptionOptions::QOS1 | SubscriptionOptions::RETAIN_HANDLING2,
            )
            .unwrap();
        let mut b = BytesMut::new();
        subscribe.serialize(&mut b).unwrap();
        assert_eq!(b.remaining(), subscribe.size());
        assert_eq!(
            b,
            &[
                0x0f, // size
                0x00, 0x7b, // packet identifier
                0x00, // props
                0x00, 0x03, 0x66, 0x6f, 0x6f, 0x04, //subscription 1
                0x00, 0x03, 0x62, 0x61, 0x72, 0x21 // subscription 2
            ][..]
        );
        let subscribe2 = Subscribe::deserialize(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        subscribe2.serialize(&mut b2).unwrap();
        assert_eq!(b, b2);
    }
}
