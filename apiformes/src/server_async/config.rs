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

    #[cfg(feature = "noise")]
    /// IP and port for encrypted MQTT
    pub noise_socketaddr: Option<SocketAddr>,

    #[cfg(feature = "noise")]
    /// Forward Packets sent over Noise to clients listening to TCP
    pub channel_permeability: Permeability,

    #[cfg(feature = "noise")]
    pub private_key: [u8; 32],
}
