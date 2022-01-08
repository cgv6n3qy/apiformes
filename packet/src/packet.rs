use super::{
    auth::Auth, connack::ConnAck, connect::Connect, data::MqttOneBytesInt, disconnect::Disconnect,
    error::DataParseError, helpers::bits_u8, packet_type::PacketType, parsable::*, ping::Ping,
    puback::PubAck, pubcomp::PubComp, publish::Publish, pubrec::PubRec, pubrel::PubRel,
    suback::SubAck, subscribe::Subscribe, unsuback::UnsubAck, unsubscribe::Unsubscribe,
};
use bytes::{Buf, BufMut};

#[derive(Clone)]
pub enum Packet {
    Connect(Connect),
    ConnAck(ConnAck),
    Publish(Publish),
    PubAck(PubAck),
    PubRec(PubRec),
    PubRel(PubRel),
    PubComp(PubComp),
    Subscribe(Subscribe),
    SubAck(SubAck),
    Unsubscribe(Unsubscribe),
    UnsubAck(UnsubAck),
    PingReq(Ping),
    PingRes(Ping),
    Disconnect(Disconnect),
    Auth(Auth),
}
impl Packet {
    pub fn to_bytes<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        self.serialize(buf)
    }
    pub fn from_bytes<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        Packet::deserialize(buf)
    }
    pub fn frame_len(&self) -> usize {
        self.size()
    }
}
impl Parsable for Packet {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        match self {
            Packet::Connect(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::Connect as u8) << 4) | PacketType::Connect.fixed_flags(),
                );
                b.serialize(buf);
                p.serialize(buf)?;
            }
            Packet::ConnAck(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::ConnAck as u8) << 4) | PacketType::ConnAck.fixed_flags(),
                );
                b.serialize(buf);
                p.serialize(buf);
            }
            Packet::Publish(p) => {
                let b = MqttOneBytesInt::new(((PacketType::Publish as u8) << 4) | p.flags().bits());
                b.serialize(buf);
                p.serialize(buf)?;
            }
            Packet::PubAck(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::PubAck as u8) << 4) | PacketType::PubAck.fixed_flags(),
                );
                b.serialize(buf);
                p.serialize(buf)?;
            }
            Packet::PubRec(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::PubRec as u8) << 4) | PacketType::PubRec.fixed_flags(),
                );
                b.serialize(buf);
                p.serialize(buf)?;
            }
            Packet::PubRel(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::PubRel as u8) << 4) | PacketType::PubRel.fixed_flags(),
                );
                b.serialize(buf);
                p.serialize(buf)?;
            }
            Packet::PubComp(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::PubComp as u8) << 4) | PacketType::PubComp.fixed_flags(),
                );
                b.serialize(buf);
                p.serialize(buf)?;
            }
            Packet::Subscribe(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::Subscribe as u8) << 4) | PacketType::Subscribe.fixed_flags(),
                );
                b.serialize(buf);
                p.serialize(buf)?;
            }
            Packet::SubAck(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::SubAck as u8) << 4) | PacketType::SubAck.fixed_flags(),
                );
                b.serialize(buf);
                p.serialize(buf)?;
            }
            Packet::Unsubscribe(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::Unsubscribe as u8) << 4) | PacketType::Unsubscribe.fixed_flags(),
                );
                b.serialize(buf);
                p.serialize(buf)?;
            }
            Packet::UnsubAck(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::UnsubAck as u8) << 4) | PacketType::UnsubAck.fixed_flags(),
                );
                b.serialize(buf);
                p.serialize(buf)?;
            }
            Packet::PingReq(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::PingReq as u8) << 4) | PacketType::PingReq.fixed_flags(),
                );
                b.serialize(buf);
                p.serialize(buf)?;
            }
            Packet::PingRes(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::PingRes as u8) << 4) | PacketType::PingRes.fixed_flags(),
                );
                b.serialize(buf);
                p.serialize(buf)?;
            }
            Packet::Disconnect(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::Disconnect as u8) << 4) | PacketType::Disconnect.fixed_flags(),
                );
                b.serialize(buf);
                p.serialize(buf)?;
            }
            Packet::Auth(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::Auth as u8) << 4) | PacketType::Auth.fixed_flags(),
                );
                b.serialize(buf);
                p.serialize(buf);
            }
        }
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        if buf.remaining() < 2 {
            return Err(DataParseError::InsufficientBuffer {
                needed: 2,
                available: buf.remaining(),
            });
        }
        let byte1 = buf.get_u8();
        let packet_type = PacketType::parse(byte1)?;
        match packet_type {
            PacketType::Reserved => Err(DataParseError::BadPacketType),
            PacketType::Connect => Ok(Packet::Connect(Connect::deserialize(buf)?)),
            PacketType::ConnAck => Ok(Packet::ConnAck(ConnAck::deserialize(buf)?)),
            PacketType::Publish => {
                let flags = bits_u8(byte1, 0, 4);
                let data = &[flags][..];
                let mut buf = data.chain(buf);
                Ok(Packet::Publish(Publish::deserialize(&mut buf)?))
            }
            PacketType::PubAck => Ok(Packet::PubAck(PubAck::deserialize(buf)?)),
            PacketType::PubRec => Ok(Packet::PubRec(PubRec::deserialize(buf)?)),
            PacketType::PubRel => Ok(Packet::PubRel(PubRel::deserialize(buf)?)),
            PacketType::PubComp => Ok(Packet::PubComp(PubComp::deserialize(buf)?)),
            PacketType::Subscribe => Ok(Packet::Subscribe(Subscribe::deserialize(buf)?)),
            PacketType::SubAck => Ok(Packet::SubAck(SubAck::deserialize(buf)?)),
            PacketType::Unsubscribe => Ok(Packet::Unsubscribe(Unsubscribe::deserialize(buf)?)),
            PacketType::UnsubAck => Ok(Packet::UnsubAck(UnsubAck::deserialize(buf)?)),
            PacketType::PingReq => Ok(Packet::PingReq(Ping::deserialize(buf)?)),
            PacketType::PingRes => Ok(Packet::PingRes(Ping::deserialize(buf)?)),
            PacketType::Disconnect => Ok(Packet::Disconnect(Disconnect::deserialize(buf)?)),
            PacketType::Auth => Ok(Packet::Auth(Auth::deserialize(buf)?)),
        }
    }
    fn size(&self) -> usize {
        1 + match self {
            Packet::Connect(p) => p.size(),
            Packet::ConnAck(p) => p.size(),
            Packet::Publish(p) => p.size(),
            Packet::PubAck(p) => p.size(),
            Packet::PubRec(p) => p.size(),
            Packet::PubRel(p) => p.size(),
            Packet::PubComp(p) => p.size(),
            Packet::Subscribe(p) => p.size(),
            Packet::SubAck(p) => p.size(),
            Packet::Unsubscribe(p) => p.size(),
            Packet::UnsubAck(p) => p.size(),
            Packet::PingReq(p) => p.size(),
            Packet::PingRes(p) => p.size(),
            Packet::Disconnect(p) => p.size(),
            Packet::Auth(p) => p.size(),
        }
    }
}
#[cfg(test)]
mod test {
    use super::super::prelude::*;
    use super::*;
    use bytes::{Buf, Bytes, BytesMut};
    use std::sync::Arc;
    #[test]
    fn test_auth_packet() {
        let auth = Auth::new(AuthReasonCode::ReAuthenticate).build();
        let mut b = BytesMut::new();
        auth.to_bytes(&mut b).unwrap();
        assert_eq!(b.remaining(), auth.size());
        assert_eq!(
            b,
            &[
                0xf0, // auth
                0x02, // size
                0x19, // reasoncode
                0x00
            ][..]
        );
        let auth2 = Packet::from_bytes(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        auth2.to_bytes(&mut b2).unwrap();
        assert_eq!(b, b2);
    }
    #[test]
    fn test_connack_packet() {
        let mut connack = ConnAck::new();
        connack.set_session_present();
        connack.set_reason_code(ConnAckReasonCode::Banned);
        connack
            .add_prop(
                Property::ReasonString,
                MqttPropValue::new_string(Arc::from("Déjà vu")).unwrap(),
            )
            .unwrap();
        let connack = connack.build();
        let mut b = BytesMut::new();
        connack.to_bytes(&mut b).unwrap();
        assert_eq!(b.remaining(), connack.size());
        assert_eq!(
            b,
            &[
                0x20, // connack
                0x0f, // size
                0x01, // flag
                0x8a, // reasoncode
                0x0c, // props size
                0x1f, // props code
                0x00, 0x09, 0x44, 0xc3, 0xa9, 0x6a, 0xc3, 0xa0, 0x20, 0x76, 0x75 // string
            ][..]
        );
        let connack2 = Packet::from_bytes(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        connack2.to_bytes(&mut b2).unwrap();
        assert_eq!(b, b2);
    }
    #[test]
    fn test_connect_packet() {
        let mut connect = Connect::new(Arc::from("Client1")).unwrap();
        connect.set_clean_start();
        connect.set_will(Will::new(Arc::from("Hello"), Bytes::from(&b"World"[..])).unwrap());
        connect.set_will_qos(QoS::QoS1).unwrap();
        connect.set_username(Arc::from("apiformes")).unwrap();
        connect.set_keep_alive(5);
        connect
            .add_prop(Property::SessionExpiryInterval, MqttPropValue::new_u32(10))
            .unwrap();
        let connect = connect.build();
        let mut b = BytesMut::new();
        connect.to_bytes(&mut b).unwrap();
        assert_eq!(b.remaining(), connect.size());
        assert_eq!(
            b,
            &[
                0x10, // connect
                0x33, //variable_length
                0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, // Protocol Name
                0x05, // Protocol version
                0x8e, // flags
                0x00, 0x05, // Keep alive
                0x05, 0x11, 0x00, 0x00, 0x00, 0x0a, // Properties
                0x00, 0x07, 0x43, 0x6c, 0x69, 0x65, 0x6e, 0x74, 0x31, // clientID
                0x00, // will props
                0x0, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f, // will topic
                0x0, 0x05, 0x57, 0x6f, 0x72, 0x6c, 0x64, // will payload][..])
                0x00, 0x09, 0x61, 0x70, 0x69, 0x66, 0x6f, 0x72, 0x6d, 0x65, 0x73, // clientID
            ][..]
        );
        let connect2 = Packet::from_bytes(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        connect2.serialize(&mut b2).unwrap();
        assert_eq!(b, b2);
    }
    #[test]
    fn test_disconnect_packet() {
        let disconnect = Disconnect::new(DisconnectReasonCode::UnspecifiedError).build();
        let mut b = BytesMut::new();
        disconnect.to_bytes(&mut b).unwrap();
        assert_eq!(b.remaining(), disconnect.size());
        assert_eq!(
            b,
            &[
                0xe0, //disconnect
                0x02, // size
                0x80, // reasoncode
                0x00
            ][..]
        );
        let disconnect2 = Packet::from_bytes(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        disconnect2.serialize(&mut b2).unwrap();
        assert_eq!(b, b2);
    }
    #[test]
    fn test_ping_req_packet() {
        let ping_req = Ping::new().build_req();
        let mut b = BytesMut::new();
        ping_req.to_bytes(&mut b).unwrap();
        assert_eq!(b.remaining(), ping_req.size());
        assert_eq!(
            b,
            &[
                0xc0, // pingreq
                0x00, // size
            ][..]
        );
        let ping_req2 = Packet::from_bytes(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        ping_req2.serialize(&mut b2).unwrap();
        assert_eq!(b, b2);
    }
    #[test]
    fn test_ping_res_packet() {
        let ping_res = Ping::new().build_res();
        let mut b = BytesMut::new();
        ping_res.to_bytes(&mut b).unwrap();
        assert_eq!(b.remaining(), ping_res.size());
        assert_eq!(
            b,
            &[
                0xd0, // pingres
                0x00, // size
            ][..]
        );
        let ping_res2 = Packet::from_bytes(&mut b.clone()).unwrap();
        let mut b2 = BytesMut::new();
        ping_res2.serialize(&mut b2).unwrap();
        assert_eq!(b, b2);
    }
}
