mod mqttclient;

pub use mqttclient::MqttListener;

use crate::packets::prelude::*;
use crate::server_async::{cfg::*, config::MqttServerConfig, error::ServerError};
use mqttclient::MqttClient;
use std::sync::Arc;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

enum Connection {
    Mqtt(MqttClient),
}

pub struct Client {
    conn: Connection,
    cfg: Arc<MqttServerConfig>,
    pub session_expirary: u32,
    pub recv_max: u16,
    pub max_packet_size: u32,
    pub topic_alias_max: u16,
    pub response_info: bool,
    pub problem_info: bool,
    pub clientid: String,
}

impl Client {
    pub(self) fn new(c: Connection, cfg: Arc<MqttServerConfig>) -> Self {
        Client {
            conn: c,
            cfg,
            session_expirary: 0,
            recv_max: u16::MAX,
            max_packet_size: u32::MAX,
            topic_alias_max: 0,
            response_info: false,
            problem_info: true,
            clientid: String::new(),
        }
    }
    pub async fn recv(&mut self) -> Result<Packet, ServerError> {
        match &mut self.conn {
            Connection::Mqtt(c) => c.recv().await,
        }
    }
    pub async fn send(&mut self, p: &Packet) -> Result<(), ServerError> {
        match &mut self.conn {
            Connection::Mqtt(c) => c.send(p).await,
        }
    }
    async fn unimplemented(&mut self) -> Result<(), ServerError> {
        let mut connack = ConnAck::new();
        connack.set_reason_code(ConnAckReasonCode::ImplementationSpecificError);
        self.send(&connack.build()).await?;
        Err(ServerError::Misc("Unimplemented".to_owned()))
    }
    #[instrument(skip(self, connect))]
    async fn process_connect(&mut self, connect: Connect) -> Result<(), ServerError> {
        if connect.username().is_some() {
            error!("Client attempted using username for authentication which is not supported");
            return self.unimplemented().await;
        }
        if connect.password().is_some() {
            error!("Client attempted using password for authentication which is not supported");
            return self.unimplemented().await;
        }
        if connect.will().is_some() {
            error!("Client attempted having a will which is not supported");
            return self.unimplemented().await;
        }
        if !connect.flags().contains(ConnectFlags::CLEAN_START) {
            error!("Client attempted reusing session which is not supported");
            return self.unimplemented().await;
        }
        for (k, v) in connect.props_iter() {
            match k {
                Property::SessionExpiryInterval => self.session_expirary = v.into_u32().unwrap(),
                Property::ReceiveMaximum => self.recv_max = v.into_u16().unwrap(),
                Property::MaximumPacketSize => self.max_packet_size = v.into_u32().unwrap(),
                Property::TopicAliasMaximum => self.topic_alias_max = v.into_u16().unwrap(),
                Property::RequestResponseInformation => self.response_info = v.into_bool().unwrap(),
                Property::RequestProblemInformation => self.problem_info = v.into_bool().unwrap(),
                Property::UserProperty => warn!(
                    "Client is using strange property in connect packet {:?}",
                    v.into_str_pair().unwrap()
                ),
                Property::AuthenticationMethod => {
                    error!("Client attempted Authentication which is not supported");
                    return self.unimplemented().await;
                }
                Property::AuthenticationData => {
                    error!("Client attempted Authentication which is not supported");
                    return self.unimplemented().await;
                }
                _ => error!(
                    "Internal Error: {:?} should not be part of Connect packet",
                    k
                ),
            }
        }
        let mut connack = ConnAck::new();
        connack.set_reason_code(ConnAckReasonCode::Success);
        // TODO once we support sessions
        // If the Server accepts a connection with Clean Start set to 1, the Server MUST set Session Present to 0 in
        // the CONNACK packet in addition to setting a 0x00 (Success) Reason Code in the CONNACK packet
        // If the Server accepts a connection with Clean Start set to 0 and the Server has Session State for the
        // ClientID, it MUST set Session Present to 1 in the CONNACK packet, otherwise it MUST set Session
        // Present to 0 in the CONNACK packet. In both cases it MUST set a 0x00 (Success) Reason Code in the
        // CONNACK packet
        connack
            .add_prop(
                Property::SessionExpiryInterval,
                MqttPropValue::new_u32(self.session_expirary),
            )
            .unwrap();
        connack
            .add_prop(
                Property::ReceiveMaximum,
                MqttPropValue::new_u16(self.recv_max),
            )
            .unwrap();
        connack
            .add_prop(
                Property::ReceiveMaximum,
                MqttPropValue::new_u16(self.recv_max),
            )
            .unwrap();
        connack
            .add_prop(Property::MaximumQoS, MqttPropValue::new_u8(MAX_QOS))
            .unwrap();
        match connect.clientid() {
            "" => {
                self.clientid = Uuid::new_v4().to_hyphenated().to_string();
                info!("Assigning {} to client", self.clientid);
                connack
                    .add_prop(
                        Property::AssignedClientIdentifier,
                        MqttPropValue::new_string(&self.clientid).unwrap(),
                    )
                    .unwrap();
            }
            x => self.clientid = x.to_string(),
        }
        connack
            .add_prop(
                Property::TopicAlias,
                MqttPropValue::new_u16(TOPIC_ALIAS_MAX),
            )
            .unwrap();
        connack
            .add_prop(
                Property::WildcardSubscriptionAvailable,
                MqttPropValue::new_bool(WILDCARD_SUB),
            )
            .unwrap();
        connack
            .add_prop(
                Property::SubscriptionIdentifierAvailable,
                MqttPropValue::new_bool(SUB_ID),
            )
            .unwrap();
        connack
            .add_prop(
                Property::SharedSubscriptionAvailable,
                MqttPropValue::new_bool(SHARED_SUB),
            )
            .unwrap();
        connack
            .add_prop(
                Property::ServerKeepAlive,
                MqttPropValue::new_u16(self.cfg.keep_alive),
            )
            .unwrap();
        self.send(&connack.build()).await
    }
    pub async fn connect(&mut self) -> Result<(), ServerError> {
        match self.recv().await? {
            Packet::Connect(c) => self.process_connect(c).await,
            _ => Err(ServerError::FirstPacketNotConnect),
        }
    }
}
