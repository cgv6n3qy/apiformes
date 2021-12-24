use super::{
    data::{MqttOneBytesInt, MqttTwoBytesInt, MqttVariableBytesInt},
    packet::Packet,
    parsable::{DataParseError, Parsable},
    props::{MqttPropValue, PropOwner, Properties, Property},
    qos::QoS,
    topic::MqttTopic,
};
use bitflags::bitflags;
use bytes::{Buf, BufMut, Bytes};
use std::convert::TryInto;

bitflags! {
    pub struct PublishFlags: u8 {
        const NO_FLAGS  = 0;
        const RETAIN    = 0b0000_0001;
        const QOS1      = 0b0000_0010;
        const QOS2      = 0b0000_0100;
        const DUP       = 0b0000_1000;
    }
}

impl Parsable for PublishFlags {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let flags = MqttOneBytesInt::new(self.bits());
        flags.serialize(buf)
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let raw_flags = MqttOneBytesInt::deserialize(buf)?;
        let flags =
            PublishFlags::from_bits(raw_flags.inner()).ok_or(DataParseError::BadConnectMessage)?;
        // sanity check on qos
        let _: QoS = flags.try_into()?;
        Ok(flags)
    }
    fn size(&self) -> usize {
        1
    }
}

impl From<QoS> for PublishFlags {
    fn from(q: QoS) -> Self {
        match q {
            QoS::QoS0 => PublishFlags::from_bits_truncate(0),
            QoS::QoS1 => PublishFlags::QOS1,
            QoS::QoS2 => PublishFlags::QOS2,
        }
    }
}

impl TryInto<QoS> for PublishFlags {
    type Error = DataParseError;
    fn try_into(self) -> Result<QoS, Self::Error> {
        if self.contains(PublishFlags::QOS1 | PublishFlags::QOS2) {
            return Err(DataParseError::BadQoS);
        }
        match self & (PublishFlags::QOS1 | PublishFlags::QOS2) {
            PublishFlags::NO_FLAGS => Ok(QoS::QoS0),
            PublishFlags::QOS1 => Ok(QoS::QoS1),
            PublishFlags::QOS2 => Ok(QoS::QoS2),
            _ => unreachable!(),
        }
    }
}

//TODO check flag is equivalent to good QoS
pub struct Publish {
    // 2.1.3 Flags & 3.3.1 PUBLISH Fixed Header.
    // Note this is part of the fixed header and not serializable
    // but it is injected so we can deserialize it here
    flags: PublishFlags,

    // 3.3.2.1 Topic Name
    topic_name: MqttTopic,

    // 3.3.2.2 Packet Identifier
    packet_identifier: Option<MqttTwoBytesInt>,

    // 3.3.2.3 PUBLISH Properties
    props: Properties,

    //3.3.3 PUBLISH Payload
    // Why did they not use MqttBinaryData :(
    // to save stupid two bytes ... :(
    payload: Bytes,
}

impl Publish {
    pub fn new(topic_name: &str) -> Result<Publish, DataParseError> {
        let topic = MqttTopic::new(topic_name)?;
        if topic.is_wildcard() {
            Err(DataParseError::BadTopic)
        } else {
            Ok(Publish {
                flags: PublishFlags::empty(),
                topic_name: topic,
                packet_identifier: None,
                props: Properties::new(),
                payload: Bytes::new(),
            })
        }
    }
    pub fn topic_name(&self) -> &str {
        &self.topic_name.inner()
    }
    pub fn packet_identifier(&self) -> Option<u16> {
        self.packet_identifier.as_ref().map(|i| i.inner())
    }
    pub fn set_retain(&mut self) {
        self.flags |= PublishFlags::RETAIN;
    }
    pub fn set_dup(&mut self) {
        self.flags |= PublishFlags::DUP;
    }
    pub fn set_packet_identifier(&mut self, i: u16) -> Result<(), DataParseError> {
        // unwrap here is justified because there are only safe wayt to set flags
        match self.flags.try_into().unwrap() {
            QoS::QoS0 => return Err(DataParseError::BadQoS),
            _ => self.packet_identifier = Some(MqttTwoBytesInt::new(i)),
        }
        Ok(())
    }
    pub fn flags(&self) -> PublishFlags {
        self.flags
    }
    pub fn set_qos(&mut self, qos: QoS) {
        self.flags -= PublishFlags::QOS1 | PublishFlags::QOS2;
        self.flags |= qos.into();
    }
    pub fn qos(&self) -> QoS {
        // why unwrap is justified here,
        // because Qos was only inserted using set_qos which should be safe (or deserialez which should be checked when deserialized)
        self.flags.try_into().unwrap()
    }
    pub fn set_payload<T: Buf>(&mut self, mut buf: T) {
        self.payload = buf.copy_to_bytes(buf.remaining());
    }
    pub fn payload(&self) -> Bytes {
        self.payload.clone()
    }
    pub fn add_prop(&mut self, key: Property, value: MqttPropValue) -> Result<(), DataParseError> {
        self.props.checked_insert(key, value, PropOwner::PUBLISH)
    }
    pub fn get_prop(&self, key: Property) -> Option<&[MqttPropValue]> {
        self.props.get(key)
    }
    pub fn props_iter(&self) -> impl Iterator<Item = (&Property, &MqttPropValue)> {
        self.props.iter()
    }
    fn partial_size(&self) -> usize {
        self.topic_name.size()
            + self
                .packet_identifier
                .as_ref()
                .map(|i| i.size())
                .unwrap_or(0)
            + self.props.size()
            + self.payload.remaining()
    }
    pub fn build(self) -> Packet {
        Packet::Publish(self)
    }
}

impl Parsable for Publish {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let length = MqttVariableBytesInt::new(self.partial_size() as u32)?;
        length.serialize(buf)?;
        self.topic_name.serialize(buf)?;
        if let Some(packet_identifier) = &self.packet_identifier {
            packet_identifier.serialize(buf)?;
        }
        self.props.serialize(buf)?;
        buf.put(self.payload.clone());
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let flags = PublishFlags::deserialize(buf)?;
        let length = MqttVariableBytesInt::deserialize(buf)?.inner() as usize;
        if buf.remaining() < length {
            return Err(DataParseError::InsufficientBuffer {
                needed: length,
                available: buf.remaining(),
            });
        }
        let mut buf = buf.take(length);
        let topic_name = MqttTopic::deserialize(&mut buf)?;
        if topic_name.is_wildcard() {
            return Err(DataParseError::BadTopic);
        }
        let packet_identifier = match flags.try_into()? {
            QoS::QoS0 => None,
            _ => Some(MqttTwoBytesInt::deserialize(&mut buf)?),
        };
        let props = Properties::deserialize(&mut buf)?;
        if !props.is_valid_for(PropOwner::PUBLISH) {
            return Err(DataParseError::BadProperty);
        }
        Ok(Publish {
            flags,
            topic_name,
            packet_identifier,
            props,
            payload: buf.copy_to_bytes(buf.remaining()),
        })
    }

    fn size(&self) -> usize {
        let size = self.partial_size();
        MqttVariableBytesInt::new(size as u32).unwrap().size() + size
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    #[test]
    fn test_bad_publish() {
        assert_eq!(
            Publish::new("topic/#").err().unwrap(),
            DataParseError::BadTopic
        );
    }
    #[test]
    fn test_publish() {
        let mut publish = Publish::new("/my/topic/").unwrap();
        assert!(publish.set_packet_identifier(123).is_err());
        publish.set_qos(QoS::QoS1);
        publish.set_packet_identifier(123).unwrap();
        publish.set_payload(Bytes::from(&[1, 2, 3][..]));
        let mut b = BytesMut::new();
        publish.serialize(&mut b).unwrap();
        assert_eq!(b.remaining(), publish.size());
        assert_eq!(
            b,
            &[
                0x12, 0x00, 0x0a, 0x2f, 0x6d, 0x79, 0x2f, 0x74, 0x6f, 0x70, 0x69, 0x63,
                0x2f, // topic
                0x00, 0x7b, // packer identifier
                0x00, // props
                0x01, 0x02, 0x03 // payload
            ][..]
        );
        let mut new_b = Bytes::from(&[0b0000_0010][..]).chain(b.clone());
        let publish2 = Publish::deserialize(&mut new_b).unwrap();
        let mut b2 = BytesMut::new();
        publish2.serialize(&mut b2).unwrap();
        assert_eq!(b, b2);
    }
}
