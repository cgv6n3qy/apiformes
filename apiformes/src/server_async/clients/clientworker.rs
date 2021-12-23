use super::{mqttclient::MqttClient, Client};
use crate::packets::prelude::*;
use crate::server_async::{cfg::*, config::MqttServerConfig, error::ServerError};
use std::sync::Arc;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;
use tokio::sync::Notify;

pub(super) enum Connection {
    Mqtt(MqttClient),
}

pub(super) struct ClientWorker {
    conn: Connection,
    cfg: Arc<MqttServerConfig>,
    internals: Client,
}

impl ClientWorker {
    pub(super) fn internals(&self) -> &Client {
        &self.internals
    }
    pub(super) fn cfg(&self) -> Arc<MqttServerConfig> {
        self.cfg.clone()
    }
    pub(super) fn new(c: Connection, cfg: Arc<MqttServerConfig>, shutdown: Arc<Notify>) -> Self {
        ClientWorker {
            conn: c,
            cfg,
            internals: Client::new(shutdown),
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
    #[instrument(name = "ClientWorker::process_connect", skip_all)]
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
                Property::SessionExpiryInterval => {
                    self.internals.session_expirary = v.into_u32().unwrap()
                }
                Property::ReceiveMaximum => self.internals.recv_max = v.into_u16().unwrap(),
                Property::MaximumPacketSize => {
                    self.internals.max_packet_size = v.into_u32().unwrap()
                }
                Property::TopicAliasMaximum => {
                    self.internals.topic_alias_max = v.into_u16().unwrap()
                }
                Property::RequestResponseInformation => {
                    self.internals.response_info = v.into_bool().unwrap()
                }
                Property::RequestProblemInformation => {
                    self.internals.problem_info = v.into_bool().unwrap()
                }
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
                MqttPropValue::new_u32(self.internals.session_expirary),
            )
            .unwrap();
        connack
            .add_prop(
                Property::ReceiveMaximum,
                MqttPropValue::new_u16(self.internals.recv_max),
            )
            .unwrap();
        connack
            .add_prop(Property::MaximumQoS, MqttPropValue::new_u8(MAX_QOS))
            .unwrap();
        match connect.clientid() {
            "" => {
                self.internals.clientid = Uuid::new_v4().to_hyphenated().to_string();
                info!("Assigning {} to client", self.internals.clientid);
                connack
                    .add_prop(
                        Property::AssignedClientIdentifier,
                        MqttPropValue::new_string(&self.internals.clientid).unwrap(),
                    )
                    .unwrap();
            }
            x => self.internals.clientid = x.to_string(),
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
