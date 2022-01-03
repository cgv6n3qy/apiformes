use super::{
    data::{MqttTwoBytesInt, MqttVariableBytesInt},
    packet::Packet,
    parsable::{DataParseError, Parsable},
    props::{MqttPropValue, PropOwner, Properties, Property},
    topic::MqttTopic,
};
use bytes::{Buf, BufMut};
use std::sync::Arc;

#[derive(Clone)]
pub struct Unsubscribe {
    // 2.2.1 Packet Identifier
    packet_identifier: MqttTwoBytesInt,
    // 3.10.2.1 UNSUBSCRIBE Properties
    props: Properties,
    /// 3.8.3 UNSUBSCRIBE Payload
    topics: Vec<MqttTopic>,
}

impl Unsubscribe {
    pub fn new(id: u16) -> Self {
        Unsubscribe {
            packet_identifier: MqttTwoBytesInt::new(id),
            props: Properties::new(),
            topics: Vec::new(),
        }
    }
    pub fn topics_iter(&self) -> impl Iterator<Item = &Arc<str>> {
        self.topics.iter().map(|t| t.inner())
    }

    pub fn add_topic(&mut self, topic: Arc<str>) -> Result<(), DataParseError> {
        let topic = MqttTopic::new(topic)?;
        self.topics.push(topic);
        Ok(())
    }
    pub fn add_prop(&mut self, key: Property, value: MqttPropValue) -> Result<(), DataParseError> {
        self.props
            .checked_insert(key, value, PropOwner::UNSUBSCRIBE)
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
            + self.topics.iter().map(|t| t.size()).sum::<usize>()
    }
    pub fn build(self) -> Packet {
        Packet::Unsubscribe(self)
    }
}

impl Parsable for Unsubscribe {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let length = MqttVariableBytesInt::new(self.partial_size() as u32)?;
        length.serialize(buf)?;
        self.packet_identifier.serialize(buf)?;
        self.props.serialize(buf)?;
        for t in &self.topics {
            t.serialize(buf)?;
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
        if !props.is_valid_for(PropOwner::UNSUBSCRIBE) {
            return Err(DataParseError::BadProperty);
        }
        let mut topics = Vec::new();
        while buf.remaining() != 0 {
            let topic = MqttTopic::deserialize(&mut buf)?;
            topics.push(topic);
        }
        if topics.is_empty() {
            Err(DataParseError::BadUnsubscribeMessage)
        } else {
            Ok(Unsubscribe {
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
    fn test_unsubscribe() {
        let mut unsubscribe = Unsubscribe::new(123);
        unsubscribe.add_topic(Arc::from("foo")).unwrap();
        unsubscribe.add_topic(Arc::from("bar")).unwrap();
        let mut b = BytesMut::new();
        unsubscribe.serialize(&mut b).unwrap();
        assert_eq!(b.remaining(), unsubscribe.size());
        assert_eq!(
            b,
            &[
                0x0d, // size
                0x00, 0x7b, // packet identifier
                0x00, // props
                0x00, 0x03, 0x66, 0x6f, 0x6f, //subscription 1
                0x00, 0x03, 0x62, 0x61, 0x72 // subscription 2
            ][..]
        );
        let unsubscribe2 = Unsubscribe::deserialize(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        unsubscribe2.serialize(&mut b2).unwrap();
        assert_eq!(b, b2);
    }
}
