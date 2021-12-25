use apiformes::server_async::{MqttServer, MqttServerConfig};
use tracing_subscriber::{filter::EnvFilter, FmtSubscriber};

#[tokio::main]
async fn main() {
    let filter = EnvFilter::from_default_env(); //.add_directive(LevelFilter::INFO.into());
    let sub = FmtSubscriber::builder()
        .with_env_filter(filter)
        .with_ansi(true)
        .finish();

    tracing::subscriber::set_global_default(sub).expect("setting tracing default failed");
    let cfg = MqttServerConfig {
        mqtt_socketaddr: Some("0.0.0.0:1883".parse().unwrap()),
        keep_alive: 50,
    };
    let _server = MqttServer::new(cfg).await.unwrap();
    //server.shutdown().await;
    loop {
        tokio::task::yield_now().await;
    }
}
