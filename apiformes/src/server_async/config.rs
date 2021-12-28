use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[cfg(feature = "noise")]
#[derive(Serialize, Deserialize, PartialEq)]
pub enum Permeability {
    Permissive,
    Strict,
}

#[derive(Serialize, Deserialize)]
pub struct MqttServerConfig {
    /// IP and port for MQTT without encryption
    pub mqtt_socketaddr: Option<SocketAddr>,
    /// time in seconds
    pub keep_alive: u16,
    /// The size of packets dispatcher queue in bytes. The size is rounded down to
    /// the nearest element size with 1 element being the minimum. The smaller the
    /// number the more back pressure applied to client threads which mean more contex
    /// switching between threads.
    pub dispatcher_queue_size: usize,
    #[cfg(feature = "noise")]
    /// IP and port for encrypted MQTT
    pub noise_socketaddr: Option<SocketAddr>,

    #[cfg(feature = "noise")]
    /// Forward Packets sent over Noise to clients listening to TCP
    pub channel_permeability: Permeability,

    #[cfg(feature = "noise")]
    pub private_key: [u8; 32],
}
