use super::{topics::Topics, Client, MqttServerConfig, ServerError};
use tokio::sync::{mpsc::UnboundedReceiver, Notify, RwLock};
use tokio::task::JoinHandle;

use crate::packets::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, instrument, warn};

pub struct Dispatcher {
    topics: Arc<RwLock<Topics>>,
    cfg: Arc<MqttServerConfig>,
    shutdown: Arc<Notify>,
    clients: Arc<RwLock<HashMap<String, Client>>>,
    incoming: UnboundedReceiver<(String, Packet)>,
}

impl Dispatcher {
    pub fn new(
        topics: Arc<RwLock<Topics>>,
        cfg: Arc<MqttServerConfig>,
        shutdown: Arc<Notify>,
        clients: Arc<RwLock<HashMap<String, Client>>>,
        incoming: UnboundedReceiver<(String, Packet)>,
    ) -> Self {
        Dispatcher {
            topics,
            cfg,
            shutdown,
            clients,
            incoming,
        }
    }
    async fn unimplemented(&mut self, client: &str) -> Result<(), ServerError> {
        let disconnect = Disconnect::new(DisconnectReasonCode::ImplementationSpecificError).build();
        let clients = self.clients.read().await;
        let c = clients.get(client).unwrap();
        if let Err(e) = c.send(disconnect) {
            error!(clientid = client, "Internal Error: tx closed");
        }
        Err(ServerError::Misc("Unimplemented".to_owned()))
    }

    async fn process_publish(&mut self, client: &str, publish: Publish) -> Result<(), ServerError> {
        match publish.qos() {
            QoS::QoS0 => (),
            QoS::QoS1 => return self.unimplemented(client).await,
            QoS::QoS2 => return self.unimplemented(client).await,
        }
        if publish
            .flags()
            .intersects(PublishFlags::DUP | PublishFlags::RETAIN)
        {
            return self.unimplemented(client).await;
        }
        let topic = publish.topic_name();
        let mut response = Publish::new(topic).unwrap();
        for (k, v) in publish.props_iter() {
            match k {
                Property::PayloadFormatIndicator => response
                    .add_prop(Property::PayloadFormatIndicator, v.clone())
                    .unwrap(),
                Property::MessageExpiryInterval => return self.unimplemented(client).await,
                Property::TopicAlias => return self.unimplemented(client).await,
                Property::ResponseTopic => response
                    .add_prop(Property::ResponseTopic, v.clone())
                    .unwrap(),
                Property::CorrelationData => response
                    .add_prop(Property::CorrelationData, v.clone())
                    .unwrap(),
                Property::UserProperty => response
                    .add_prop(Property::UserProperty, v.clone())
                    .unwrap(),
                Property::SubscriptionIdentifier => return self.unimplemented(client).await,
                Property::ContentType => {
                    response.add_prop(Property::ContentType, v.clone()).unwrap()
                }
                _ => error!(
                    "Internal Error: {:?} should not be part of publish packet",
                    k
                ),
            }
        }
        response.set_payload(publish.payload());
        let resp = response.build();
        if let Some(ids) = self.topics.read().await.get_subscribed(topic) {
            let clients = self.clients.read().await;
            for id in ids {
                if let Some(c) = clients.get(id) {
                    if let Err(e) = c.send(resp.clone()) {
                        error!(clientid = id.as_str(), "Internal Error: tx closed");

                    };
                }
            }
        }
        Ok(())
    }
    async fn process_packet(&mut self, client: &str, packet: Packet) -> Result<(), ServerError> {
        match packet {
            Packet::Publish(publish) => self.process_publish(&client, publish).await,
            _ => self.unimplemented(client).await,
        }
    }
    async fn process_forever(mut self) {
        loop {
            let (client, packet) = match self.incoming.recv().await {
                Some(data) => data,
                None => {
                    warn!("incomming tx is closed");
                    break;
                }
            };
            if let Err(e) = self.process_packet(&client, packet).await {
                error!(clientid = &*client, "{:?}", e);
            }
        }
    }
    #[instrument(name = "Dispatcher::run", skip(self))]
    async fn run(self) {
        let shutdown = self.shutdown.clone();
        tokio::select! {
            _ = shutdown.notified() => (),
            _ = self.process_forever() => (),
        }
    }
    pub async fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            self.run().await;
        })
    }
}
