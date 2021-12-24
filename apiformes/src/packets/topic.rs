use super::{
    data::MqttUtf8String,
    parsable::{DataParseError, Parsable},
};
use bytes::{Buf, BufMut};

#[derive(Clone)]
pub struct MqttTopic(MqttUtf8String);

impl MqttTopic {
    pub fn is_wildcard(&self) -> bool {
        self.inner().chars().any(|c| "#+".contains(c))
    }
    pub fn is_system(&self) -> bool {
        self.inner()
            .chars()
            .next()
            .map(|c| c == '$')
            .unwrap_or(false)
    }
    pub fn new(topic: &str) -> Result<MqttTopic, DataParseError> {
        Ok(MqttTopic(MqttUtf8String::new(topic.to_owned())?))
    }
    pub fn unwrap(self) -> String {
        self.0.unwrap()
    }

    pub fn inner(&self) -> &str {
        self.0.inner()
    }

    pub fn inner_mut(&mut self) -> &mut String {
        self.0.inner_mut()
    }
}

impl Parsable for MqttTopic {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        self.0.serialize(buf)
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        Ok(MqttTopic(MqttUtf8String::deserialize(buf)?))
    }
    fn size(&self) -> usize {
        self.0.size()
    }
}
