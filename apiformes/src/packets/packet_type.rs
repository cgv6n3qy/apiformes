use super::parsable::DataParseError;
fn bits_u8(data: u8, start: usize, len: usize) -> u8 {
    (data >> start) & ((2 << (len - 1)) - 1)
}

///2.1.2 MQTT Control Packet type
#[repr(u8)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub enum PackeType {
    Reserved = 0b0000,
    Connect = 0b0001,
    ConnAck = 0b0010,
    Publish = 0b0011,
    PubAck = 0b0100,
    PubRec = 0b0101,
    PubRel = 0b0110,
    PubComp = 0b0111,
    Subscribe = 0b1000,
    SubAck = 0b1001,
    Unsubscribe = 0b1010,
    UnsubAck = 0b1011,
    PingReq = 0b1100,
    PingRes = 0b1101,
    Disconnect = 0b1110,
    Auth = 0b1111,
}

impl PackeType {
    fn flags10(&self, data: u8) -> Result<(), DataParseError> {
        if bits_u8(data, 0, 4) == 0b0010 {
            Ok(())
        } else {
            Err(DataParseError::BadPacketType)
        }
    }
    fn all_flags_zero(&self, data: u8) -> Result<(), DataParseError> {
        if bits_u8(data, 0, 4) == 0 {
            Ok(())
        } else {
            Err(DataParseError::BadPacketType)
        }
    }
    fn check_flags(&self, data: u8) -> Result<(), DataParseError> {
        match self {
            PackeType::Publish => Ok(()),
            PackeType::PubRel => self.flags10(data),
            PackeType::Subscribe => self.flags10(data),
            PackeType::Unsubscribe => self.flags10(data),
            _ => self.all_flags_zero(data),
        }
    }
    pub fn parse(data: u8) -> Result<Self, DataParseError> {
        let packet_type = match bits_u8(data, 4, 4) {
            0 => PackeType::Reserved,
            1 => PackeType::Connect,
            2 => PackeType::ConnAck,
            3 => PackeType::Publish,
            4 => PackeType::PubAck,
            5 => PackeType::PubRec,
            6 => PackeType::PubRel,
            7 => PackeType::PubComp,
            8 => PackeType::Subscribe,
            9 => PackeType::SubAck,
            10 => PackeType::Unsubscribe,
            11 => PackeType::UnsubAck,
            12 => PackeType::PingReq,
            13 => PackeType::PingRes,
            14 => PackeType::Disconnect,
            15 => PackeType::Auth,
            _ => unreachable!(),
        };
        packet_type.check_flags(data)?;
        Ok(packet_type)
    }
}

#[cfg(test)]
mod test {
    #[cfg(feature = "debug")]
    use super::*;

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_auth() {
        let byte = 0b1111_0000;
        assert_eq!(PackeType::parse(byte).unwrap(), PackeType::Auth);
        let byte = 0b1111_0011;
        match PackeType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_disconnect() {
        let byte = 0b1110_0000;
        assert_eq!(PackeType::parse(byte).unwrap(), PackeType::Disconnect);
        let byte = 0b1110_0011;
        match PackeType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_pingres() {
        let byte = 0b1101_0000;
        assert_eq!(PackeType::parse(byte).unwrap(), PackeType::PingRes);
        let byte = 0b1101_0011;
        match PackeType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_pingreq() {
        let byte = 0b1100_0000;
        assert_eq!(PackeType::parse(byte).unwrap(), PackeType::PingReq);
        let byte = 0b1100_0011;
        match PackeType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_unsuback() {
        let byte = 0b1011_0000;
        assert_eq!(PackeType::parse(byte).unwrap(), PackeType::UnsubAck);
        let byte = 0b1011_0011;
        match PackeType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_unsub() {
        let byte = 0b1010_0010;
        assert_eq!(PackeType::parse(byte).unwrap(), PackeType::Unsubscribe);
        let byte = 0b1010_0011;
        match PackeType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_suback() {
        let byte = 0b1001_0000;
        assert_eq!(PackeType::parse(byte).unwrap(), PackeType::SubAck);
        let byte = 0b1001_0011;
        match PackeType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_subscribe() {
        let byte = 0b1000_0010;
        assert_eq!(PackeType::parse(byte).unwrap(), PackeType::Subscribe);
        let byte = 0b1000_0011;
        match PackeType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_pubcomp() {
        let byte = 0b0111_0000;
        assert_eq!(PackeType::parse(byte).unwrap(), PackeType::PubComp);
        let byte = 0b0111_0011;
        match PackeType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_pubrel() {
        let byte = 0b0110_0010;
        assert_eq!(PackeType::parse(byte).unwrap(), PackeType::PubRel);
        let byte = 0b0110_0011;
        match PackeType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_pubrec() {
        let byte = 0b0101_0000;
        assert_eq!(PackeType::parse(byte).unwrap(), PackeType::PubRec);
        let byte = 0b0101_0001;
        match PackeType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_puback() {
        let byte = 0b0100_0000;
        assert_eq!(PackeType::parse(byte).unwrap(), PackeType::PubAck);
        let byte = 0b0100_0001;
        match PackeType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_publish() {
        for byte in 0b0011_0000..0b0100_0000 {
            assert_eq!(PackeType::parse(byte).unwrap(), PackeType::Publish);
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_connack() {
        let byte = 0b0010_0000;
        assert_eq!(PackeType::parse(byte).unwrap(), PackeType::ConnAck);
        let byte = 0b0010_0001;
        match PackeType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_connect() {
        let byte = 0b0001_0000;
        assert_eq!(PackeType::parse(byte).unwrap(), PackeType::Connect);
        let byte = 0b0001_0001;
        match PackeType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_reserved() {
        let byte = 0b0000_0000;
        assert_eq!(PackeType::parse(byte).unwrap(), PackeType::Reserved);
        let byte = 0b0000_0001;
        match PackeType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }
}
