use super::{
    auth::Auth,
    connack::ConnAck,
    connect::Connect,
    data::MqttOneBytesInt,
    disconnect::Disconnect,
    helpers::bits_u8,
    packet_type::PacketType,
    parsable::{DataParseError, Parsable},
    ping::{PingReq, PingRes},
    puback::PubAck,
    pubcomp::PubComp,
    publish::Publish,
    pubrec::PubRec,
    pubrel::PubRel,
    suback::SubAck,
    subscribe::Subscribe,
    unsuback::UnsubAck,
    unsubscribe::Unsubscribe,
};
use bytes::{Buf, BufMut};

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
    PingReq(PingReq),
    PingRes(PingRes),
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
                b.serialize(buf)?;
                p.serialize(buf)?;
            }
            Packet::ConnAck(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::ConnAck as u8) << 4) | PacketType::ConnAck.fixed_flags(),
                );
                b.serialize(buf)?;
                p.serialize(buf)?;
            }
            Packet::Publish(p) => {
                let b = MqttOneBytesInt::new(((PacketType::Publish as u8) << 4) | p.flags().bits());
                b.serialize(buf)?;
                p.serialize(buf)?;
            }
            Packet::PubAck(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::PubAck as u8) << 4) | PacketType::PubAck.fixed_flags(),
                );
                b.serialize(buf)?;
                p.serialize(buf)?;
            }
            Packet::PubRec(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::PubRec as u8) << 4) | PacketType::PubRec.fixed_flags(),
                );
                b.serialize(buf)?;
                p.serialize(buf)?;
            }
            Packet::PubRel(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::PubRel as u8) << 4) | PacketType::PubRel.fixed_flags(),
                );
                b.serialize(buf)?;
                p.serialize(buf)?;
            }
            Packet::PubComp(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::PubComp as u8) << 4) | PacketType::PubComp.fixed_flags(),
                );
                b.serialize(buf)?;
                p.serialize(buf)?;
            }
            Packet::Subscribe(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::Subscribe as u8) << 4) | PacketType::Subscribe.fixed_flags(),
                );
                b.serialize(buf)?;
                p.serialize(buf)?;
            }
            Packet::SubAck(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::SubAck as u8) << 4) | PacketType::SubAck.fixed_flags(),
                );
                b.serialize(buf)?;
                p.serialize(buf)?;
            }
            Packet::Unsubscribe(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::Unsubscribe as u8) << 4) | PacketType::Unsubscribe.fixed_flags(),
                );
                b.serialize(buf)?;
                p.serialize(buf)?;
            }
            Packet::UnsubAck(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::UnsubAck as u8) << 4) | PacketType::UnsubAck.fixed_flags(),
                );
                b.serialize(buf)?;
                p.serialize(buf)?;
            }
            Packet::PingReq(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::PingReq as u8) << 4) | PacketType::PingReq.fixed_flags(),
                );
                b.serialize(buf)?;
                p.serialize(buf)?;
            }
            Packet::PingRes(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::PingRes as u8) << 4) | PacketType::PingRes.fixed_flags(),
                );
                b.serialize(buf)?;
                p.serialize(buf)?;
            }
            Packet::Disconnect(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::Disconnect as u8) << 4) | PacketType::Disconnect.fixed_flags(),
                );
                b.serialize(buf)?;
                p.serialize(buf)?;
            }
            Packet::Auth(p) => {
                let b = MqttOneBytesInt::new(
                    ((PacketType::Auth as u8) << 4) | PacketType::Auth.fixed_flags(),
                );
                b.serialize(buf)?;
                p.serialize(buf)?;
            }
        }
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
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
            PacketType::PingReq => Ok(Packet::PingReq(PingReq::deserialize(buf)?)),
            PacketType::PingRes => Ok(Packet::PingRes(PingRes::deserialize(buf)?)),
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
