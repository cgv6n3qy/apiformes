use super::{data::MqttUtf8String, error::DataParseError, parsable::*};
use bytes::{Buf, BufMut};
use std::sync::Arc;

#[derive(Clone)]
pub struct MqttTopic(MqttUtf8String);

fn is_valid_topic(topic: &str) -> bool {
    let mut iter = topic.chars().peekable();
    let mut prev = '/';
    while let Some(c) = iter.next() {
        match c {
            '#' => {
                // must be last character
                // previous character must be `/` or non existant
                if prev != '/' || iter.peek().is_some() {
                    return false;
                }
            }
            '+' => {
                if prev != '/' || *iter.peek().unwrap_or(&'/') != '/' {
                    return false;
                }
            }
            _ => (),
        }
        prev = c;
    }
    true
}

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
    pub fn new(topic: Arc<str>) -> Result<MqttTopic, DataParseError> {
        if !is_valid_topic(&topic) {
            Err(DataParseError::BadTopic)
        } else {
            Ok(MqttTopic(MqttUtf8String::new(topic)?))
        }
    }
    pub fn unwrap(self) -> Arc<str> {
        self.0.unwrap()
    }

    pub fn inner(&self) -> &Arc<str> {
        self.0.inner()
    }
}

impl MqttSerialize for MqttTopic {
    fn serialize<T: BufMut>(&self, buf: &mut T) {
        self.0.serialize(buf);
    }
}
impl MqttDeserialize for MqttTopic {
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let string = MqttUtf8String::deserialize(buf)?;
        if !is_valid_topic(string.inner()) {
            Err(DataParseError::BadTopic)
        } else {
            Ok(MqttTopic(string))
        }
    }
}
impl MqttSize for MqttTopic {
    fn min_size() -> usize {
        MqttUtf8String::min_size()
    }
    fn size(&self) -> usize {
        self.0.size()
    }
}
