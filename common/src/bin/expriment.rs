use std::{sync::Arc, time::Duration};

use common::bus::mqtt::MqttClient;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // let payload = env::var("MQTT_PAYLOAD").unwrap_or_else(|_| "hello from common".to_owned());

    let client = Arc::new(
        MqttClient::create(
            "common-demo-client".to_owned(),
            "rpi-dev-home-main:1883".to_owned(),
        )
        .expect("failed to start mqtt client"),
    );

    let mut events = client.events();
    let thread_client = Arc::downgrade(&client);
    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            println!("event: {event:?}");

            if let common::bus::mqtt::MqttEvent::Connected = event {
                let client = thread_client.upgrade().expect("failed to upgrade client");
                client
                    .subscribe(vec![String::from("#")])
                    .expect("failed to subscribe");
            }
        }
    });

    // client.subscribe(vec![String::from("#")]).expect("failed to subscribe");
    // client
    //     .publish(&topic, payload.as_bytes(), false)
    //     .expect("failed to publish");

    tokio::time::sleep(Duration::from_secs(3)).await;
    let client = Arc::try_unwrap(client).expect("failed to unwrap client");
    client.shutdown().await;
}
