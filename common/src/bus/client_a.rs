use std::{collections::HashSet, mem::swap, time::Duration};

use bytes::Bytes;
use kameo::{message, prelude::*};
use kameo_actors::pubsub::{self, PubSub};
use tokio::{select, sync::broadcast, time::timeout};

use crate::bus::{
    client_a::State::Running,
    encoding,
    mqtt::{MqttClient, MqttEvent},
};

use super::mqtt;

#[derive(Debug, Clone)]
pub struct Publisher<Message: Send + Clone + 'static> {
    name: &'static str,
    pubsub_ref: ActorRef<PubSub<Message>>,
}

impl<Message: Send + Clone + 'static> Publisher<Message> {
    pub fn new(name: &'static str) -> Self {
        let pubsub_ref = ActorRef::lookup(name)
            .expect("error during registry looking")
            .unwrap_or_else(|| panic!("pubsub '{}' not found", name));
        Self { name, pubsub_ref }
    }

    pub fn publish(&self, msg: Message) {
        if let Err(e) = self.pubsub_ref.tell(pubsub::Publish(msg)).try_send() {
            log::error!("Could not send message to pubsub '{}': {}", self.name, e);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientRef(ActorRef<Client>);

impl ClientRef {
    const NAME: &str = "bus.client";

    pub fn new() -> Self {
        Self(
            ActorRef::lookup(Self::NAME)
                .expect("error during registry looking")
                .unwrap_or_else(|| panic!("actor '{}' not found", Self::NAME)),
        )
    }

    pub fn publish(&self, topic: Topic, payload: Bytes, retain: bool) {
        self.send(Publish {
            topic,
            payload,
            retain,
        });
    }

    pub fn clear_retain(&self, topic: Topic) {
        self.publish(topic, Bytes::new(), true);
    }

    pub fn subscribe(&self, subscription: Subscription) {
        self.send(Subscribe(subscription));
    }

    pub fn unsubscribe(&self, subscription: Subscription) {
        self.send(Unsubscribe(subscription));
    }

    fn send<Message>(&self, msg: Message)
    where
        Client: message::Message<Message>,
        Message: Send + 'static,
    {
        if let Err(e) = self.0.tell(msg).try_send() {
            log::error!("Could not send message to pubsub '{}': {}", Self::NAME, e);
        }
    }
}

/// Client manages the MQTT connection, providing an interface for the bus to interact with the MQTT layer.
#[derive(Debug)]
pub struct Client(State);

#[derive(Debug)]
enum State {
    Running(RunningData),
    Stopped,
}

#[derive(Debug)]
struct RunningData {
    instance_name: String,

    mqtt_client: MqttClient,
    events: broadcast::Receiver<MqttEvent>,

    subscriptions: HashSet<String>,

    on_message: Publisher<Message>,
    on_online: Publisher<Online>,
}

#[derive(Debug)]
pub struct ClientConfig {
    pub instance_name: String,
    pub server_address: String,
}

impl Actor for Client {
    type Args = ClientConfig;
    type Error = anyhow::Error;

    async fn on_start(
        config: ClientConfig,
        _actor_ref: kameo::prelude::ActorRef<Self>,
    ) -> anyhow::Result<Self> {
        let last_will = mqtt::LastWill {
            topic: format!("{}/online", config.instance_name),
            payload: Bytes::new(),
            retain: true,
        };

        let mqtt_client = MqttClient::create(
            config.instance_name.clone(),
            config.server_address,
            Some(last_will),
        )?;

        let events = mqtt_client.events();

        Ok(Self(State::Running(RunningData {
            instance_name: config.instance_name,
            mqtt_client,
            events,
            subscriptions: HashSet::new(),
            on_message: Publisher::new("bus.client.message"),
            on_online: Publisher::new("bus.client.online"),
        })))
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> anyhow::Result<()> {
        let mut state = State::Stopped;
        swap(&mut state, &mut self.0);

        let State::Running(data) = state else {
            panic!("incorrect state");
        };

        data.shutdown().await;

        Ok(())
    }

    async fn next(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        mailbox_rx: &mut MailboxReceiver<Self>,
    ) -> anyhow::Result<Option<mailbox::Signal<Self>>> {
        loop {
            select! {
                event = self.running_data().get_next_event() => {
                    self.running_data().process_event(event).await;
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
        self.running_data().subscribe(msg.0);
    }
}

impl message::Message<Unsubscribe> for Client {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: Unsubscribe,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.running_data().unsubscribe(msg.0);
    }
}

impl message::Message<Publish> for Client {
    type Reply = ();

    async fn handle(&mut self, msg: Publish, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.running_data()
            .publish(msg.topic, msg.payload, msg.retain);
    }
}

impl Client {
    fn running_data(&mut self) -> &mut RunningData {
        let Running(data) = &mut self.0 else {
            panic!("not running");
        };

        data
    }
}

impl RunningData {
    pub async fn get_next_event(&mut self) -> mqtt::MqttEvent {
        loop {
            match self.events.recv().await {
                Ok(event) => {
                    return event;
                }
                Err(e) => match e {
                    broadcast::error::RecvError::Lagged(count) => {
                        log::warn!("MQTT event channel lagged, skipped {} messages", count);
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
                if let Err(e) = self.clear_resident_state().await {
                    log::error!("Failed to clear resident state: {}", e);
                    return;
                }

                self.publish(
                    TopicBuilder::local(&self.instance_name, "online").build(),
                    encoding::write_bool(true),
                    true,
                );

                self.on_online.publish(Online(true));

                if let Err(e) = self
                    .mqtt_client
                    .subscribe(self.subscriptions.iter().cloned().collect())
                {
                    log::error!(
                        "failed to subscribe to topics {:?}: {}",
                        self.subscriptions.iter().cloned().collect::<Vec<_>>(),
                        e
                    );
                }

                log::info!("MQTT client connected");
            }

            MqttEvent::Disconnected { reason } => {
                self.on_online.publish(Online(false));
                log::info!("MQTT client disconnected: {}", reason);
            }

            MqttEvent::Message {
                topic,
                payload,
                retain,
            } => {
                self.on_message
                    .publish(Message::new(topic, payload, retain));
            }

            MqttEvent::Error(e) => {
                log::error!("got mqtt error: {}", e);
            }
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
}

struct TempSubscription<'a> {
    client: &'a MqttClient,
    topic: String,
}

impl<'a> TempSubscription<'a> {
    pub fn new(client: &'a MqttClient, topic: String) -> Self {
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

/// MQTT message, with additional helper methods dedicated to bus protocol.
#[derive(Debug, Clone)]
pub struct Message {
    // TODO: cheap clone since RO
    topic: String,
    payload: Bytes,
    retain: bool,
}

impl Message {
    /// Create a new Message with the given topic, payload and retain flag.
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

#[derive(Debug, Clone)]
pub struct Publish {
    topic: Topic,
    payload: Bytes,
    retain: bool,
}

#[derive(Debug, Clone)]
pub struct Online(bool);

#[derive(Debug, Clone)]
pub struct Subscribe(Subscription);

#[derive(Debug, Clone)]
pub struct Unsubscribe(Subscription);

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
