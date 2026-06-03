use std::{collections::HashSet, time::Duration};

use bytes::Bytes;
use tokio::{sync::broadcast, time::timeout};

use crate::bus::encoding;

use super::mqtt;

/// Client manages the MQTT connection, providing an interface for the bus to interact with the MQTT layer.
#[derive(Debug)]
pub struct Client {
    instance_name: String,
    mqtt_client: mqtt::MqttClient,
    events: broadcast::Receiver<mqtt::MqttEvent>,
    online: bool,
    subscriptions: HashSet<String>,
}

impl Client {
    /// Create a new Client with the given instance name and server address.
    pub fn create(instance_name: String, server_address: String) -> anyhow::Result<Self> {
        let last_will = mqtt::LastWill {
            topic: format!("{}/online", instance_name),
            payload: Bytes::new(),
            retain: true,
        };

        let mqtt_client =
            mqtt::MqttClient::create(instance_name.clone(), server_address, Some(last_will))?;
        let events = mqtt_client.events();

        Ok(Self {
            instance_name,
            mqtt_client,
            events,
            online: false,
            subscriptions: HashSet::new(),
        })
    }

    /// Wait for the next MQTT event, returning it when received. This method will block until an event is received or the event channel is closed.
    ///
    /// Reserved for the main loop of the bus.
    pub async fn next_event(&mut self) -> Option<MqttEvent> {
        loop {
            match self.events.recv().await {
                Ok(event) => {
                    if let Some(event) = self.translate_event(event) {
                        self.process_event(&event).await;
                        return Some(event);
                    } else {
                        continue;
                    }
                }
                Err(e) => match e {
                    broadcast::error::RecvError::Lagged(count) => {
                        log::warn!("MQTT event channel lagged, skipped {} messages", count);
                        continue;
                    }
                    broadcast::error::RecvError::Closed => {
                        log::error!("MQTT event channel closed");
                        return None;
                    }
                },
            }
        }
    }

    fn translate_event(&mut self, event: mqtt::MqttEvent) -> Option<MqttEvent> {
        match event {
            mqtt::MqttEvent::Connected => {
                return Some(MqttEvent::Connected);
            }

            mqtt::MqttEvent::Disconnected { .. } => {
                return Some(MqttEvent::Disconnected);
            }

            mqtt::MqttEvent::Error(e) => {
                log::error!("MQTT error: {}", e);
                return None;
            }

            mqtt::MqttEvent::Message {
                topic,
                payload,
                retain,
            } => {
                return Some(MqttEvent::Message(MqttMessage::new(topic, payload, retain)));
            }
        }
    }

    async fn process_event(&mut self, event: &MqttEvent) {
        match event {
            MqttEvent::Connected => {
                if let Err(e) = self.clear_resident_state().await {
                    log::error!("Failed to clear resident state: {}", e);
                    return;
                }

                self.publish(
                    TopicBuilder::local(&self.instance_name, "online").build(),
                    encoding::write_bool(true),
                    true,
                );

                self.online = true;

                if let Err(e) = self
                    .mqtt_client
                    .subscribe(self.subscriptions.iter().cloned().collect())
                {
                    log::error!(
                        "failed to subscribe to topics {:?}: {}",
                        self.subscriptions,
                        e
                    );
                }

                log::info!("MQTT client connected");
            }

            MqttEvent::Disconnected { .. } => {
                self.online = false;
                log::warn!("MQTT client disconnected");
            }

            _ => {}
        }
    }

    async fn clear_resident_state(&mut self) -> anyhow::Result<()> {
        // register on self state, and remove on every message received
        // wait 1 sec after last message receive

        let _temp_sub =
            TempSubscription::new(&self.mqtt_client, format!("{}/#", self.instance_name));

        loop {
            match timeout(Duration::from_secs(1), self.events.recv()).await {
                Ok(Ok(event)) => {
                    if let mqtt::MqttEvent::Message { topic, retain, .. } = event {
                        if retain && topic.starts_with(&format!("{}/", self.instance_name)) {
                            self.clear_retain(Topic(topic));
                        }

                        continue;
                    } else {
                        anyhow::bail!(
                            "Received non-message event while clearing resident state: {:?}",
                            event
                        );
                    }
                }
                Ok(Err(e)) => match e {
                    broadcast::error::RecvError::Lagged(count) => {
                        log::warn!("MQTT event channel lagged, skipped {} messages", count);
                        continue;
                    }
                    broadcast::error::RecvError::Closed => {
                        anyhow::bail!("MQTT event channel closed");
                    }
                },
                Err(_) => {
                    // timeout, no message received for 1 second, consider resident state cleared
                    log::trace!("Resident state cleared");
                    return Ok(());
                }
            }
        }
    }

    /// Terminate the client, closing the MQTT connection and cleaning up resources. After calling this method, the client should not be used anymore.
    ///
    /// Reserved for the main loop of the bus.
    pub async fn shutdown(self) {
        self.clear_retain(TopicBuilder::local(&self.instance_name, "online").build());
        self.mqtt_client.shutdown().await;
    }

    /// Check if the client is currently online.
    pub fn online(&self) -> bool {
        self.online
    }

    /// Subscribe to a topic, adding it to the set of subscriptions and sending a subscribe request to the MQTT client if it's a new subscription.
    pub fn subscribe(&mut self, topic: Subscription) {
        let topic = topic.as_str();
        if self.subscriptions.insert(topic.to_owned()) {
            if let Err(e) = self.mqtt_client.subscribe(vec![topic.to_owned()]) {
                log::error!("failed to subscribe to topic {}: {}", topic, e);
            }
        }
    }

    /// Unsubscribe from a topic, removing it from the set of subscriptions and sending an unsubscribe request to the MQTT client if it was previously subscribed.
    pub fn unsubscribe(&mut self, topic: Subscription) {
        let topic = topic.as_str();
        if self.subscriptions.remove(topic) {
            if let Err(e) = self.mqtt_client.unsubscribe(vec![topic.to_owned()]) {
                log::error!("failed to unsubscribe from topic {}: {}", topic, e);
            }
        }
    }

    /// Publish a message to a topic, sending a publish request to the MQTT client.
    pub fn publish(&self, topic: Topic, payload: Bytes, retain: bool) {
        if let Err(e) = self.mqtt_client.publish(topic.to_string(), payload, retain) {
            log::error!("failed to publish message to topic {}: {}", topic, e);
        }
    }

    /// Clear the retained message of a topic.
    pub fn clear_retain(&self, topic: Topic) {
        self.publish(topic, Bytes::new(), true);
    }

    /// Get the instance name
    pub fn instance_name(&self) -> &str {
        &self.instance_name
    }
}

struct TempSubscription<'a> {
    client: &'a mqtt::MqttClient,
    topic: String,
}

impl<'a> TempSubscription<'a> {
    pub fn new(client: &'a mqtt::MqttClient, topic: String) -> Self {
        if let Err(e) = client.subscribe(vec![topic.clone()]) {
            log::error!("failed to subscribe to topic {}: {}", topic, e);
        }

        Self { client, topic }
    }
}

impl Drop for TempSubscription<'_> {
    fn drop(&mut self) {
        if let Err(e) = self.client.unsubscribe(vec![self.topic.clone()]) {
            log::error!("failed to unsubscribe from topic {}: {}", self.topic, e);
        }
    }
}

/// Output event of the client
#[derive(Debug, Clone)]
pub enum MqttEvent {
    /// Emitted when the client successfully establishes a connection to the broker.
    Connected,
    /// Emitted when the client loses connection to the broker or the broker
    /// closes the connection.
    Disconnected,
    /// Emitted when a message is received from a subscribed topic.
    Message(MqttMessage),
}

/// MQTT message, with additional helper methods dedicated to bus protocol.
#[derive(Debug, Clone)]
pub struct MqttMessage {
    topic: String,
    payload: Bytes,
    retain: bool,
}

impl MqttMessage {
    /// Create a new MqttMessage with the given topic, payload and retain flag.
    pub fn new(topic: String, payload: Bytes, retain: bool) -> Self {
        Self {
            topic,
            payload,
            retain,
        }
    }

    /// Get the topic of the message.
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// Get the payload of the message.
    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    /// Get the retain flag of the message.
    pub fn retain(&self) -> bool {
        self.retain
    }

    /// Get the instance part of the topic, which is the first segment.
    pub fn instance(&self) -> Option<&str> {
        self.topic.split('/').nth(0)
    }

    /// Get the domain part of the topic, which is the second segment.
    pub fn domain(&self) -> Option<&str> {
        self.topic.split('/').nth(1)
    }
}

/// A concrete, fully resolved topic with no wildcards. Suitable for publishing,
/// and usable as an exact-match subscription via `From<Topic>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Topic(String);

impl Topic {
    /// Borrows the topic as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the topic and returns the owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for Topic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A subscription filter, which may contain `+` wildcards in any segment and an
/// optional trailing `#`. Use for subscribing; cannot be used to publish.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Subscription(String);

impl Subscription {
    /// Borrows the filter as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the filter and returns the owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for Subscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A concrete topic is a valid exact-match subscription. The reverse does not
/// hold, so there is no `From<Subscription> for Topic`.
impl From<Topic> for Subscription {
    fn from(t: Topic) -> Self {
        Subscription(t.0)
    }
}

fn check_segment(seg: &str) {
    assert!(
        !seg.contains('/') && !seg.contains('+') && !seg.contains('#'),
        "topic segment must not contain '/', '+' or '#': {seg:?}"
    );
}

/// Builds a topic that is still concrete (no `+` added yet).
///
/// While in this state the result can become either a publishable [`Topic`]
/// (`build`) or a [`Subscription`] (`subscribe_exact`, `rest`). Adding a `+`
/// with `any` transitions to [`SubscriptionBuilder`], after which only a
/// `Subscription` can be produced.
pub struct TopicBuilder {
    parts: Vec<String>,
}

impl TopicBuilder {
    /// Starts a topic on the local instance: `{instance}/{domain}`.
    pub fn local(instance: &str, domain: &str) -> Self {
        check_segment(instance);
        check_segment(domain);
        Self {
            parts: vec![instance.to_string(), domain.to_string()],
        }
    }

    /// Starts a topic targeting another instance: `{target}/{domain}`.
    pub fn remote(target: &str, domain: &str) -> Self {
        check_segment(target);
        check_segment(domain);
        Self {
            parts: vec![target.to_string(), domain.to_string()],
        }
    }

    /// Starts a filter with a wildcard instance slot: `+/{domain}` (for example
    /// `+/online`). Returns a [`SubscriptionBuilder`] because a `+` is now present,
    /// so the result can only ever be a [`Subscription`].
    pub fn any_instance(domain: &str) -> SubscriptionBuilder {
        check_segment(domain);
        SubscriptionBuilder {
            parts: vec!["+".to_string(), domain.to_string()],
        }
    }

    /// Appends one concrete path segment, staying concrete.
    pub fn segment(mut self, seg: &str) -> Self {
        check_segment(seg);
        self.parts.push(seg.to_string());
        self
    }

    /// Appends several concrete segments in order.
    pub fn segments<'a>(mut self, segs: impl IntoIterator<Item = &'a str>) -> Self {
        for s in segs {
            self = self.segment(s);
        }
        self
    }

    /// Appends a single-level wildcard `+` and transitions to
    /// [`SubscriptionBuilder`], since the result can no longer be a publishable
    /// [`Topic`].
    pub fn any(mut self) -> SubscriptionBuilder {
        self.parts.push("+".to_string());
        SubscriptionBuilder { parts: self.parts }
    }

    /// Finishes as a concrete, publishable [`Topic`].
    pub fn build(self) -> Topic {
        Topic(self.parts.join("/"))
    }

    /// Finishes as an exact-match [`Subscription`] (no wildcard).
    pub fn subscribe_exact(self) -> Subscription {
        Subscription(self.parts.join("/"))
    }

    /// Finishes as a [`Subscription`] ending in `#`, matching everything below
    /// the current path.
    pub fn rest(mut self) -> Subscription {
        self.parts.push("#".to_string());
        Subscription(self.parts.join("/"))
    }
}

/// Builds a [`Subscription`] that already contains at least one `+`.
///
/// Reached from [`TopicBuilder::any`] or [`TopicBuilder::any_instance`].
/// It can keep appending concrete or `+` segments and finishes only as a
/// `Subscription`, optionally with a trailing `#`.
pub struct SubscriptionBuilder {
    parts: Vec<String>,
}

impl SubscriptionBuilder {
    /// Appends one concrete path segment.
    pub fn segment(mut self, seg: &str) -> Self {
        check_segment(seg);
        self.parts.push(seg.to_string());
        self
    }

    /// Appends several concrete segments in order.
    pub fn segments<'a>(mut self, segs: impl IntoIterator<Item = &'a str>) -> Self {
        for s in segs {
            self = self.segment(s);
        }
        self
    }

    /// Appends another single-level wildcard `+`.
    pub fn any(mut self) -> Self {
        self.parts.push("+".to_string());
        self
    }

    /// Finishes as a [`Subscription`] with no trailing `#`.
    pub fn build(self) -> Subscription {
        Subscription(self.parts.join("/"))
    }

    /// Finishes as a [`Subscription`] ending in `#`.
    pub fn rest(mut self) -> Subscription {
        self.parts.push("#".to_string());
        Subscription(self.parts.join("/"))
    }
}
