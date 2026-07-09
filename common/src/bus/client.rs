use std::{collections::HashSet, sync::Arc, time::Duration};

use bytes::Bytes;
use kameo::{message, prelude::*};
use thiserror::Error;
use tokio::{select, sync::broadcast, time::timeout};

use crate::{
    bus::{
        encoding,
        mqtt::{MqttClient, MqttEvent},
    },
    utils::actors::{
        ActorHandle, HandleLookupError, PublisherHandle, SpawnedActor, SpawnedActors,
        SubscriberHandle, spawn_pubsub,
    },
};

use super::mqtt;

/// Name of the client actor
const CLIENT_NAME: &str = "bus.client";

/// Name of the PubSub actor that delivers messages
const MESSAGE_PUBSUB_NAME: &str = "bus.client.message";

/// Name of the PubSub actor that delivers online changes
const ONLINE_PUBSUB_NAME: &str = "bus.client.online";

/// Name of the PubSub actor that delivers instance online changes
const INSTANCE_ONLINE_PUBSUB_NAME: &str = "bus.client.instance-online";

const ONLINE_DOMAIN: &str = "online";

#[derive(Debug)]
pub struct ClientConfig {
    pub instance_name: Arc<String>,
    pub server_address: String,
}

/// Client access to the client actor
#[derive(Debug, Clone)]
pub struct ClientHandle {
    actor: ActorHandle<Client>,
    on_message: SubscriberHandle<Message>,
    on_online: SubscriberHandle<Online>,
    on_instance_online: SubscriberHandle<InstanceOnline>,
}

impl ClientHandle {
    /// Create a new access
    pub fn new() -> Result<Self, HandleLookupError> {
        Ok(Self {
            actor: ActorHandle::from_name(CLIENT_NAME)?,
            on_message: SubscriberHandle::from_name(MESSAGE_PUBSUB_NAME)?,
            on_online: SubscriberHandle::from_name(ONLINE_PUBSUB_NAME)?,
            on_instance_online: SubscriberHandle::from_name(INSTANCE_ONLINE_PUBSUB_NAME)?,
        })
    }

    /// Publish a message to MQTT
    pub fn publish(&self, topic: Topic, payload: Bytes, retain: bool) {
        self.actor.send(Publish {
            topic,
            payload,
            retain,
        });
    }

    /// Clear a retained message
    pub fn clear_retain(&self, topic: Topic) {
        self.publish(topic, Bytes::new(), true);
    }

    /// Subscribe to an MQTT topic
    pub fn subscribe(&self, subscription: Subscription) {
        self.actor.send(Subscribe(subscription));
    }

    /// Unsubscribe to an MQTT topic
    pub fn unsubscribe(&self, subscription: Subscription) {
        self.actor.send(Unsubscribe(subscription));
    }

    /// Get the PubSub for incoming MQTT messages
    pub fn on_message(&self) -> &SubscriberHandle<Message> {
        &self.on_message
    }

    /// Get the PubSub for online
    pub fn on_online(&self) -> &SubscriberHandle<Online> {
        &self.on_online
    }

    /// Get the PubSub for instance onlone
    pub fn on_instance_online(&self) -> &SubscriberHandle<InstanceOnline> {
        &self.on_instance_online
    }
}

/// Init PubSub links related to client (no dependency)
pub async fn init_pubsubs(actors: &mut SpawnedActors) {
    actors.add(spawn_pubsub::<InstanceOnline>(INSTANCE_ONLINE_PUBSUB_NAME).await);
    actors.add(spawn_pubsub::<Online>(ONLINE_PUBSUB_NAME).await);
    actors.add(spawn_pubsub::<Message>(MESSAGE_PUBSUB_NAME).await);
}

/// Init client actor
pub async fn init_actor(actors: &mut SpawnedActors, config: ClientConfig) {
    let (client, _) = SpawnedActor::start::<Client>(config).await;

    client.register(CLIENT_NAME);

    actors.add(client);
}

/// Client manages the MQTT connection, providing an interface for the bus to interact with the MQTT layer.
#[derive(Debug)]
struct Client {
    instance_name: Arc<String>,

    mqtt_client: Option<MqttClient>,
    events: broadcast::Receiver<MqttEvent>,

    subscriptions: HashSet<String>,
    online_instances: HashSet<String>,

    on_message: PublisherHandle<Message>,
    on_online: PublisherHandle<Online>,
    on_instance_online: PublisherHandle<InstanceOnline>,
}

/// Error that occurs when the client actor fails to start or operate correctly.
#[derive(Debug, Error)]
pub enum ClientActorError {
    #[error("Failed to lookup actor handle: {0}")]
    HandleLookupError(#[from] HandleLookupError),
    #[error("MQTT error: {0}")]
    MqttError(#[from] mqtt::MqttError),
}

impl Actor for Client {
    type Args = ClientConfig;
    type Error = ClientActorError;

    async fn on_start(
        config: ClientConfig,
        _actor_ref: ActorRef<Self>,
    ) -> Result<Self, ClientActorError> {
        let last_will = mqtt::LastWill {
            topic: format!("{}/online", config.instance_name),
            payload: Bytes::new(),
            retain: true,
        };

        let mqtt_client = MqttClient::create(
            (*config.instance_name).clone(),
            config.server_address,
            Some(last_will),
        )?;

        let events = mqtt_client.events();

        Ok(Self {
            instance_name: config.instance_name,
            mqtt_client: Some(mqtt_client),
            events,
            subscriptions: HashSet::new(),
            online_instances: HashSet::new(),
            on_message: PublisherHandle::from_name(MESSAGE_PUBSUB_NAME)?,
            on_online: PublisherHandle::from_name(ONLINE_PUBSUB_NAME)?,
            on_instance_online: PublisherHandle::from_name(INSTANCE_ONLINE_PUBSUB_NAME)?,
        })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), ClientActorError> {
        self.mark_offline();

        let mqtt_client = self.mqtt_client.take().expect("incorrect state");
        mqtt_client.shutdown().await;

        Ok(())
    }

    async fn next(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        mailbox_rx: &mut MailboxReceiver<Self>,
    ) -> Result<Option<mailbox::Signal<Self>>, ClientActorError> {
        loop {
            select! {
                event = self.get_next_event() => {
                    self.process_event(event).await;
                },
                res = mailbox_rx.recv() => {
                    return Ok(res)
                }
            }
        }
    }
}

impl message::Message<Subscribe> for Client {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: Subscribe,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let topic = msg.0.as_str();

        let Some(mqtt_client) = &self.mqtt_client else {
            tracing::error!(
                error = "mqtt client not set",
                topic,
                "failed to subscribe to topic"
            );
            return;
        };

        if self.subscriptions.insert(topic.to_owned()) {
            if let Err(error) = mqtt_client.subscribe(vec![topic.to_owned()]) {
                tracing::error!(%error, topic, "failed to subscribe to topic");
            }

            tracing::trace!(topic, "subscribed to topic");
        }
    }
}

impl message::Message<Unsubscribe> for Client {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: Unsubscribe,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let topic = msg.0.as_str();

        let Some(mqtt_client) = &self.mqtt_client else {
            tracing::error!(
                error = "mqtt client not set",
                topic,
                "failed to unsubscribe from topic"
            );
            return;
        };

        if self.subscriptions.remove(topic) {
            if let Err(error) = mqtt_client.unsubscribe(vec![topic.to_owned()]) {
                tracing::error!(%error, topic, "failed to unsubscribe from topic");
            }

            tracing::trace!(topic, "unsubscribed from topic");
        }
    }
}

impl message::Message<Publish> for Client {
    type Reply = ();

    async fn handle(&mut self, msg: Publish, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.publish(msg.topic, msg.payload, msg.retain);
    }
}

impl Client {
    async fn get_next_event(&mut self) -> mqtt::MqttEvent {
        loop {
            match self.events.recv().await {
                Ok(event) => {
                    return event;
                }
                Err(e) => match e {
                    broadcast::error::RecvError::Lagged(count) => {
                        tracing::warn!(count, "MQTT event channel lagged, skipped messages");
                    }
                    broadcast::error::RecvError::Closed => {
                        panic!("MQTT event channel closed");
                    }
                },
            }
        }
    }

    async fn process_event(&mut self, event: MqttEvent) {
        match event {
            MqttEvent::Connected => {
                if !self.clear_resident_state().await {
                    return;
                }

                self.publish(
                    TopicBuilder::local(&self.instance_name, ONLINE_DOMAIN).build(),
                    encoding::write_bool(true),
                    true,
                );

                self.on_online.publish(Online(true));
                self.resume_subscriptions();

                tracing::info!("MQTT client connected");
            }

            MqttEvent::Disconnected { reason } => {
                self.clear_instance_online();
                self.on_online.publish(Online(false));
                tracing::info!(reason, "MQTT client disconnected");
            }

            MqttEvent::Message {
                topic,
                payload,
                retain: _,
            } => {
                // On receive, retain=true means this is a retained message being delivered
                // because we just subscribed. Only meaningful in clear_resident_state.
                let msg = Message::new(topic, payload);
                self.process_instance_online_message(&msg);
                self.on_message.publish(msg);
            }

            MqttEvent::Error(error) => {
                tracing::error!(%error, "got mqtt error");
            }
        }
    }

    async fn clear_resident_state(&mut self) -> bool {
        self.mark_offline();

        // register on self state, and remove on every message received
        // wait 1 sec after last message receive
        let mqtt_client = self.mqtt_client.as_ref().expect("mqtt client not set");

        let _temp_sub = TempSubscription::new(mqtt_client, format!("{}/#", self.instance_name));

        loop {
            match timeout(Duration::from_secs(1), self.events.recv()).await {
                Ok(Ok(event)) => {
                    if let mqtt::MqttEvent::Message { topic, retain, .. } = event {
                        if retain && topic.starts_with(&format!("{}/", self.instance_name)) {
                            self.clear_retain(Topic(topic));
                        }

                        continue;
                    } else {
                        tracing::error!(
                            ?event,
                            "failed to clear resident state: received non-message event while clearing resident state",
                        );

                        return false;
                    }
                }
                Ok(Err(e)) => match e {
                    broadcast::error::RecvError::Lagged(count) => {
                        tracing::warn!("MQTT event channel lagged, skipped {} messages", count);
                        continue;
                    }
                    broadcast::error::RecvError::Closed => {
                        panic!("MQTT event channel closed");
                    }
                },
                Err(_) => {
                    // timeout, no message received for 1 second, consider resident state cleared
                    tracing::trace!("resident state cleared");
                    return true;
                }
            }
        }
    }

    /// Publish a message to a topic, sending a publish request to the MQTT client.
    fn publish(&self, topic: Topic, payload: Bytes, retain: bool) {
        let Some(mqtt_client) = &self.mqtt_client else {
            tracing::error!(
                error = "mqtt client not set",
                %topic,
                "failed to publish message to topic"
            );
            return;
        };

        if let Err(error) = mqtt_client.publish(topic.to_string(), payload, retain) {
            tracing::error!(%error, %topic, "failed to publish message to topic");
        }
    }

    /// Clear the retained message of a topic.
    fn clear_retain(&self, topic: Topic) {
        self.publish(topic, Bytes::new(), true);
    }

    fn mark_offline(&self) {
        self.clear_retain(TopicBuilder::local(&self.instance_name, ONLINE_DOMAIN).build());
    }

    fn resume_subscriptions(&self) {
        let Some(mqtt_client) = &self.mqtt_client else {
            tracing::error!("mqtt client not set; cannot resume subscriptions");
            return;
        };

        let mut subscriptions: Vec<_> = self.subscriptions.iter().cloned().collect();

        // Add online instances subscription (builtin)
        subscriptions.push(
            TopicBuilder::any_instance(ONLINE_DOMAIN)
                .build()
                .into_string(),
        );

        if let Err(error) = mqtt_client.subscribe(subscriptions) {
            let topics = self.subscriptions.iter().cloned().collect::<Vec<_>>();
            tracing::error!(%error, ?topics, "failed to subscribe to topics");
        }
    }

    fn clear_instance_online(&mut self) {
        for instance in self.online_instances.drain() {
            self.on_instance_online.publish(InstanceOnline {
                instance: Arc::new(instance),
                online: false,
            });
        }
    }

    fn process_instance_online_message(&mut self, msg: &Message) {
        let Some(topic) = msg.parse_topic() else {
            return;
        };

        if topic.domain != ONLINE_DOMAIN || topic.instance == self.instance_name.as_str() {
            return;
        }

        let online = if msg.payload().is_empty() {
            false
        } else {
            match encoding::read_bool(msg.payload()) {
                Ok(value) => value,
                Err(error) => {
                    tracing::error!(%error, payload = ?msg.payload(), "error reading online value");
                    return;
                }
            }
        };

        self.set_instance_online(String::from(topic.instance), online);
    }

    fn set_instance_online(&mut self, instance: String, online: bool) {
        let do_publish = if online {
            self.online_instances.insert(instance.clone())
        } else {
            self.online_instances.remove(&instance)
        };

        if do_publish {
            self.on_instance_online.publish(InstanceOnline {
                instance: Arc::new(instance),
                online,
            });
        }
    }
}

struct TempSubscription<'a> {
    client: &'a MqttClient,
    topic: String,
}

impl<'a> TempSubscription<'a> {
    pub fn new(client: &'a MqttClient, topic: String) -> Self {
        if let Err(error) = client.subscribe(vec![topic.clone()]) {
            tracing::error!(%error, topic, "failed to subscribe to topic");
        }

        Self { client, topic }
    }
}

impl Drop for TempSubscription<'_> {
    fn drop(&mut self) {
        if let Err(error) = self.client.unsubscribe(vec![self.topic.clone()]) {
            tracing::error!(
                %error,
                topic = self.topic,
                "failed to unsubscribe from topic"
            );
        }
    }
}

/// MQTT message, with additional helper methods dedicated to bus protocol.
#[derive(Debug, Clone)]
pub struct Message {
    topic: Arc<String>,
    payload: Arc<Bytes>,
}

impl Message {
    /// Create a new Message with the given topic, payload and retain flag.
    fn new(topic: String, payload: Bytes) -> Self {
        Self {
            topic: Arc::new(topic),
            payload: Arc::new(payload),
        }
    }

    /// Get the topic of the message.
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// Get the payload of the message.
    pub fn payload(&self) -> &Arc<Bytes> {
        &self.payload
    }

    /// Parse the topic to extract usefull parts
    pub fn parse_topic(&'_ self) -> Option<ParsedTopic<'_>> {
        let mut parts = self.topic.splitn(3, '/');
        let Some(instance) = parts.next() else {
            return None;
        };
        let Some(domain) = parts.next() else {
            return None;
        };
        let remaining = parts.next().unwrap_or_default();

        Some(ParsedTopic {
            instance,
            domain,
            remaining,
        })
    }
}

/// Output of the topic parsing
#[derive(Debug)]
pub struct ParsedTopic<'a> {
    pub instance: &'a str,
    pub domain: &'a str,
    pub remaining: &'a str,
}

#[derive(Debug, Clone)]
struct Publish {
    topic: Topic,
    payload: Bytes,
    retain: bool,
}

#[derive(Debug, Clone)]
struct Subscribe(Subscription);

#[derive(Debug, Clone)]
struct Unsubscribe(Subscription);

#[derive(Debug, Clone)]
pub struct Online(bool);

impl Online {
    pub fn is_online(&self) -> bool {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct InstanceOnline {
    instance: Arc<String>,
    online: bool,
}

impl InstanceOnline {
    pub fn instance(&self) -> &str {
        &self.instance
    }

    pub fn is_online(&self) -> bool {
        self.online
    }
}

/// A concrete, fully resolved topic with no wildcards. Suitable for publishing,
/// and usable as an exact-match subscription via `From<Topic>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Topic(String);

impl Topic {
    pub fn from_raw(value: String) -> Self {
        Self(value)
    }

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
