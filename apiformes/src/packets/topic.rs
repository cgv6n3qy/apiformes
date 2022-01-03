use super::{
    data::MqttUtf8String,
    parsable::{DataParseError, Parsable},
};
use bytes::{Buf, BufMut};

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
    pub fn new(topic: &str) -> Result<MqttTopic, DataParseError> {
        if !is_valid_topic(topic) {
            Err(DataParseError::BadTopic)
        } else {
            Ok(MqttTopic(MqttUtf8String::new(topic.to_owned())?))
        }
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
        let string = MqttUtf8String::deserialize(buf)?;
        if !is_valid_topic(string.inner()) {
            Err(DataParseError::BadTopic)
        } else {
            Ok(MqttTopic(string))
        }
    }
    fn size(&self) -> usize {
        self.0.size()
    }
}
