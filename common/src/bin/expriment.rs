use std::env;
use std::time::Duration;

use common::{MqttClient, MqttConfig, QoS};

#[tokio::main]
async fn main() {
    let host = env::var("BROKER_HOST").unwrap_or_else(|_| "rpi-dev-home-main".to_owned());
    let port = env::var("BROKER_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1883);
    let client_id = env::var("MQTT_CLIENT_ID")
        .unwrap_or_else(|_| format!("common-demo-{}", std::process::id()));
    let topic = env::var("MQTT_TOPIC").unwrap_or_else(|_| "#".to_owned());
    // let payload = env::var("MQTT_PAYLOAD").unwrap_or_else(|_| "hello from common".to_owned());

    let config = MqttConfig {
        broker_host: host,
        broker_port: port,
        client_id,
        event_capacity: 128,
    };

    let client = MqttClient::connect(config)
        .await
        .expect("failed to start mqtt client");

    let mut events = client.events();
    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            println!("event: {event:?}");
        }
    });

    client
        .subscribe(&topic, QoS::AtMostOnce)
        .await
        .expect("failed to subscribe");
    // client
    //     .publish(&topic, payload.as_bytes(), QoS::AtMostOnce, false)
    //     .await
    //     .expect("failed to publish");

    tokio::time::sleep(Duration::from_secs(3)).await;
    client
        .shutdown()
        .await
        .expect("failed to shutdown client");
}
