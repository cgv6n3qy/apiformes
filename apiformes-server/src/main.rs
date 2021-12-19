use apiformes::server_async::{MqttServer, MqttServerConfig};
use tracing_subscriber::FmtSubscriber;
#[tokio::main]
async fn main() {
    let sub = FmtSubscriber::new();
    tracing::subscriber::set_global_default(sub).expect("setting tracing default failed");
    let cfg = MqttServerConfig {
        mqtt_socketaddr: Some("0.0.0.0:1883".parse().unwrap()),
        keep_alive: 50
    };
    let server = MqttServer::new(cfg).await.unwrap();
    server.shutdown().await;
}
