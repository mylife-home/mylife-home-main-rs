use std::time::Duration;

use common::MqttClient;

#[tokio::main]
async fn main() {
    // let payload = env::var("MQTT_PAYLOAD").unwrap_or_else(|_| "hello from common".to_owned());

    let client = MqttClient::connect(
        "common-demo-client".to_owned(),
        "rpi-dev-home-main:1883".to_owned(),
    )
    .await
    .expect("failed to start mqtt client");

    let mut events = client.events();
    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            println!("event: {event:?}");
        }
    });

    client.subscribe("#").expect("failed to subscribe");
    // client
    //     .publish(&topic, payload.as_bytes(), false)
    //     .expect("failed to publish");

    tokio::time::sleep(Duration::from_secs(3)).await;
    client.shutdown().await.expect("failed to shutdown client");
}
