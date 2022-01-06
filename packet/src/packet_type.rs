use super::{helpers::bits_u8, parsable::DataParseError};

///2.1.2 MQTT Control Packet type
#[repr(u8)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub(crate) enum PacketType {
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

impl PacketType {
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
            PacketType::Publish => Ok(()),
            PacketType::PubRel => self.flags10(data),
            PacketType::Subscribe => self.flags10(data),
            PacketType::Unsubscribe => self.flags10(data),
            _ => self.all_flags_zero(data),
        }
    }
    pub(super) fn parse(data: u8) -> Result<Self, DataParseError> {
        let packet_type = match bits_u8(data, 4, 4) {
            0 => PacketType::Reserved,
            1 => PacketType::Connect,
            2 => PacketType::ConnAck,
            3 => PacketType::Publish,
            4 => PacketType::PubAck,
            5 => PacketType::PubRec,
            6 => PacketType::PubRel,
            7 => PacketType::PubComp,
            8 => PacketType::Subscribe,
            9 => PacketType::SubAck,
            10 => PacketType::Unsubscribe,
            11 => PacketType::UnsubAck,
            12 => PacketType::PingReq,
            13 => PacketType::PingRes,
            14 => PacketType::Disconnect,
            15 => PacketType::Auth,
            _ => unreachable!(),
        };
        packet_type.check_flags(data)?;
        Ok(packet_type)
    }
    pub(super) fn fixed_flags(&self) -> u8 {
        match self {
            PacketType::PubRel => 0b0010,
            PacketType::Subscribe => 0b0010,
            PacketType::Unsubscribe => 0b0010,
            _ => 0,
        }
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
        assert_eq!(PacketType::parse(byte).unwrap(), PacketType::Auth);
        let byte = 0b1111_0011;
        match PacketType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_disconnect() {
        let byte = 0b1110_0000;
        assert_eq!(PacketType::parse(byte).unwrap(), PacketType::Disconnect);
        let byte = 0b1110_0011;
        match PacketType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_pingres() {
        let byte = 0b1101_0000;
        assert_eq!(PacketType::parse(byte).unwrap(), PacketType::PingRes);
        let byte = 0b1101_0011;
        match PacketType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_pingreq() {
        let byte = 0b1100_0000;
        assert_eq!(PacketType::parse(byte).unwrap(), PacketType::PingReq);
        let byte = 0b1100_0011;
        match PacketType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_unsuback() {
        let byte = 0b1011_0000;
        assert_eq!(PacketType::parse(byte).unwrap(), PacketType::UnsubAck);
        let byte = 0b1011_0011;
        match PacketType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_unsub() {
        let byte = 0b1010_0010;
        assert_eq!(PacketType::parse(byte).unwrap(), PacketType::Unsubscribe);
        let byte = 0b1010_0011;
        match PacketType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_suback() {
        let byte = 0b1001_0000;
        assert_eq!(PacketType::parse(byte).unwrap(), PacketType::SubAck);
        let byte = 0b1001_0011;
        match PacketType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_subscribe() {
        let byte = 0b1000_0010;
        assert_eq!(PacketType::parse(byte).unwrap(), PacketType::Subscribe);
        let byte = 0b1000_0011;
        match PacketType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_pubcomp() {
        let byte = 0b0111_0000;
        assert_eq!(PacketType::parse(byte).unwrap(), PacketType::PubComp);
        let byte = 0b0111_0011;
        match PacketType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_pubrel() {
        let byte = 0b0110_0010;
        assert_eq!(PacketType::parse(byte).unwrap(), PacketType::PubRel);
        let byte = 0b0110_0011;
        match PacketType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_pubrec() {
        let byte = 0b0101_0000;
        assert_eq!(PacketType::parse(byte).unwrap(), PacketType::PubRec);
        let byte = 0b0101_0001;
        match PacketType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_puback() {
        let byte = 0b0100_0000;
        assert_eq!(PacketType::parse(byte).unwrap(), PacketType::PubAck);
        let byte = 0b0100_0001;
        match PacketType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_publish() {
        for byte in 0b0011_0000..0b0100_0000 {
            assert_eq!(PacketType::parse(byte).unwrap(), PacketType::Publish);
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_connack() {
        let byte = 0b0010_0000;
        assert_eq!(PacketType::parse(byte).unwrap(), PacketType::ConnAck);
        let byte = 0b0010_0001;
        match PacketType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_connect() {
        let byte = 0b0001_0000;
        assert_eq!(PacketType::parse(byte).unwrap(), PacketType::Connect);
        let byte = 0b0001_0001;
        match PacketType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_packet_type_reserved() {
        let byte = 0b0000_0000;
        assert_eq!(PacketType::parse(byte).unwrap(), PacketType::Reserved);
        let byte = 0b0000_0001;
        match PacketType::parse(byte) {
            Err(DataParseError::BadPacketType) => (),
            _ => panic!("Expected DataParseError::BadPacketType"),
        }
    }
}
