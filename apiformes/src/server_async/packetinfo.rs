use crate::packets::prelude::Packet;
use std::sync::Arc;

pub struct PacketInfo {
    pub senderid: Arc<str>,
    pub packet: Packet,
}
