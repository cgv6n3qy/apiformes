use apiformes::server_async::{MqttServer, MqttServerConfig, Permeability};
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
        noise_socketaddr: Some("0.0.0.0:8883".parse().unwrap()),
        channel_permeability: Permeability::Strict,
        dispatcher_queue_size: 1024 * 1024,
        private_key: [
            205, 100, 157, 80, 236, 140, 109, 150, 91, 254, 27, 10, 200, 89, 193, 158, 49, 238, 24,
            134, 137, 225, 220, 169, 32, 209, 239, 35, 2, 254, 0, 166,
        ],
        //public_key = [
        //          180, 132, 40, 246, 52, 36, 9, 93, 224,
        //          18, 51, 123, 188, 226, 131, 145, 196,
        //          93, 24, 112, 227, 133, 8, 199, 229, 139,
        //          2, 248, 5, 115, 136, 37
        //  ]
    };
    let _server = MqttServer::new(cfg).await.unwrap();
    //server.shutdown().await;
    loop {
        tokio::task::yield_now().await;
    }
}
