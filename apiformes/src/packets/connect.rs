use super::{
    props::{MqttPropValue, PropOwner, Properties, Property},
    qos::QoS,
};
use crate::data::{
    MqttBinaryData, MqttOneBytesInt, MqttTwoBytesInt, MqttUtf8String, MqttVariableBytesInt,
};
use crate::parsable::{DataParseError, Parsable};
use bitflags::bitflags;
use bytes::{Buf, BufMut, Bytes};

bitflags! {
    pub struct ConnectFlags: u8 {
        // this must be commented, because bitflags will return and error
        // once it find the RESERVED bit used .. which is exactly what we want
        //const RESRVED =     0b0000_0001;
        const CLEAN_START = 0b0000_0010;
        const WILL =        0b0000_0100;
        const WILL_QOS1 =   0b0000_1000;
        const WILL_QOS2 =   0b0001_0000;
        const WILL_RETAIN = 0b0010_0000;
        const PASSWORD =    0b0100_0000;
        const USERNAME =    0b1000_0000;
    }
}

impl Parsable for ConnectFlags {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let flags = MqttOneBytesInt::new(self.bits());
        flags.serialize(buf)
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let raw_flags = MqttOneBytesInt::deserialize(buf)?;
        ConnectFlags::from_bits(raw_flags.inner()).ok_or(DataParseError::BadConnectMessage)
    }
    fn size(&self) -> usize {
        1
    }
}

impl From<QoS> for ConnectFlags {
    fn from(q: QoS) -> Self {
        match q {
            QoS::QoS0 => ConnectFlags::from_bits_truncate(0),
            QoS::QoS1 => ConnectFlags::WILL_QOS1,
            QoS::QoS2 => ConnectFlags::WILL_QOS2,
        }
    }
}

pub struct Will {
    // 3.1.3.2 Will Properties
    props: Properties,
    // 3.1.3.3 Will Topic
    topic: MqttUtf8String,
    // 3.1.3.4 Will Payload
    payload: MqttBinaryData,
}

impl Will {
    pub fn new<T: Buf>(topic: &str, payload: T) -> Result<Self, DataParseError> {
        Ok(Will {
            props: Properties::new(),
            topic: MqttUtf8String::new(topic.to_owned())?,
            payload: MqttBinaryData::new(payload)?,
        })
    }
    pub fn add_prop(&mut self, key: Property, value: MqttPropValue) -> Result<(), DataParseError> {
        self.props.checked_insert(key, value, PropOwner::WILL)
    }
    pub fn get_prop(&self, key: Property) -> Option<&[MqttPropValue]> {
        self.props.get(key)
    }
    pub fn props_iter(&self) -> impl Iterator<Item = (&Property, &MqttPropValue)> {
        self.props.iter()
    }
    pub fn topic(&self) -> &str {
        self.topic.inner()
    }
    pub fn set_topic(&mut self, topic: &str) -> Result<(), DataParseError> {
        self.topic = MqttUtf8String::new(topic.to_owned())?;
        Ok(())
    }
    pub fn payload(&mut self) -> &Bytes {
        self.payload.inner()
    }
    pub fn set_payload(&mut self) -> &Bytes {
        self.payload.inner()
    }
}
impl Parsable for Will {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        self.props.serialize(buf)?;
        self.topic.serialize(buf)?;
        self.payload.serialize(buf)
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let props = Properties::deserialize(buf)?;
        if props.is_valid_for(PropOwner::WILL) {
            Ok(Will {
                props,
                topic: MqttUtf8String::deserialize(buf)?,
                payload: MqttBinaryData::deserialize(buf)?,
            })
        } else {
            Err(DataParseError::BadProperty)
        }
    }
    fn size(&self) -> usize {
        self.props.size() + self.topic.size() + self.payload.size()
    }
}

// 3.1 CONNECT – Connection Request
pub struct Connect {
    // 3.1.2.3 Connect Flags
    flags: ConnectFlags,
    // 3.1.2.10 Keep Alive
    keep_alive: MqttTwoBytesInt,
    // 3.1.2.11 CONNECT Properties
    props: Properties,

    // 3.1.3 CONNECT Payload

    // 3.1.3.1 Client Identifier (ClientID)
    clientid: MqttUtf8String,

    // 3.1.3.2 till 3.1.3.4
    will_info: Option<Will>,

    // 3.1.3.5 User Name
    username: Option<MqttUtf8String>,

    // 3.1.3.6 Password
    password: Option<MqttBinaryData>,
}

impl Connect {
    pub fn new(clientid: String) -> Result<Self, DataParseError> {
        Ok(Connect {
            flags: ConnectFlags::from_bits_truncate(0),
            keep_alive: MqttTwoBytesInt::new(0),
            props: Properties::new(),
            clientid: MqttUtf8String::new(clientid)?,
            will_info: None,
            username: None,
            password: None,
        })
    }
    pub fn clientid(&self) -> &str {
        self.clientid.inner()
    }
    pub fn flags(&self) -> ConnectFlags {
        self.flags
    }
    pub fn username(&self) -> Option<&str> {
        self.username.as_ref().map(|s| s.inner())
    }
    pub fn set_username(&mut self, username: &str) -> Result<(), DataParseError> {
        self.flags |= ConnectFlags::USERNAME;
        self.username = Some(MqttUtf8String::new(username.to_owned())?);
        Ok(())
    }
    pub fn password(&self) -> Option<&Bytes> {
        self.password.as_ref().map(|p| p.inner())
    }
    pub fn set_password<T: Buf>(&mut self, password: T) -> Result<(), DataParseError> {
        self.flags |= ConnectFlags::PASSWORD;
        self.password = Some(MqttBinaryData::new(password)?);
        Ok(())
    }
    pub fn keep_alive(&self) -> u16 {
        self.keep_alive.inner()
    }
    pub fn set_keep_alive(&mut self, keep_alive: u16) {
        self.keep_alive = MqttTwoBytesInt::new(keep_alive);
    }
    pub fn will(&self) -> Option<&Will> {
        self.will_info.as_ref()
    }
    pub fn set_will(&mut self, will_info: Will) {
        self.flags |= ConnectFlags::WILL;
        self.will_info = Some(will_info);
    }
    pub fn set_will_qos(&mut self, qos: QoS) {
        self.flags -= ConnectFlags::WILL_QOS1 | ConnectFlags::WILL_QOS2;
        self.flags |= qos.into();
    }
    pub fn add_prop(&mut self, key: Property, value: MqttPropValue) -> Result<(), DataParseError> {
        self.props.checked_insert(key, value, PropOwner::CONNECT)
    }
    pub fn get_prop(&self, key: Property) -> Option<&[MqttPropValue]> {
        self.props.get(key)
    }
    pub fn props_iter(&self) -> impl Iterator<Item = (&Property, &MqttPropValue)> {
        self.props.iter()
    }
}

impl Parsable for Connect {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        let length = MqttVariableBytesInt::new(self.size() as u32 - 2)?;
        length.serialize(buf)?;

        let protocol_name = MqttUtf8String::new("MQTT".to_string())?;
        protocol_name.serialize(buf)?;

        let protocol_version = MqttOneBytesInt::new(5);
        protocol_version.serialize(buf)?;

        self.flags.serialize(buf)?;

        self.keep_alive.serialize(buf)?;

        self.props.serialize(buf)?;

        self.clientid.serialize(buf)?;

        if let Some(will) = self.will_info.as_ref() {
            will.serialize(buf)?;
        }

        if let Some(username) = self.username.as_ref() {
            username.serialize(buf)?;
        }

        if let Some(password) = self.password.as_ref() {
            password.serialize(buf)?;
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
        let protocol_name = MqttUtf8String::deserialize(buf)?;
        if protocol_name.inner() != "MQTT" {
            return Err(DataParseError::BadConnectMessage);
        }
        let protocol_version = MqttOneBytesInt::deserialize(buf)?;
        if protocol_version.inner() != 5 {
            return Err(DataParseError::UnsupportedMqttVersion);
        }
        let flags = ConnectFlags::deserialize(buf)?;
        let keep_alive = MqttTwoBytesInt::deserialize(buf)?;
        let props = Properties::deserialize(buf)?;
        if !props.is_valid_for(PropOwner::CONNECT) {
            return Err(DataParseError::BadProperty);
        }
        let clientid = MqttUtf8String::deserialize(buf)?;
        let will_info = if flags.contains(ConnectFlags::WILL) {
            Some(Will::deserialize(buf)?)
        } else {
            None
        };
        let username = if flags.contains(ConnectFlags::USERNAME) {
            Some(MqttUtf8String::deserialize(buf)?)
        } else {
            None
        };
        let password = if flags.contains(ConnectFlags::PASSWORD) {
            Some(MqttBinaryData::deserialize(buf)?)
        } else {
            None
        };
        let packet = Connect {
            flags,
            keep_alive,
            props,
            clientid,
            will_info,
            username,
            password,
        };
        if packet.size() == length {
            Ok(packet)
        } else {
            Err(DataParseError::BadConnectMessage)
        }
    }
    fn size(&self) -> usize {
        // 7 = 6 for "MQTT" string + 1 for the version
        let size = 7
            + self.flags.size()
            + self.keep_alive.size()
            + self.props.size()
            + self.clientid.size()
            + self.will_info.as_ref().map(|w| w.size()).unwrap_or(0)
            + self.username.as_ref().map(|u| u.size()).unwrap_or(0)
            + self.password.as_ref().map(|p| p.size()).unwrap_or(0);
        MqttVariableBytesInt::new(size as u32).unwrap().size() + size
    }
}
