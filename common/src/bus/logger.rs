use std::{process, sync::Arc, time::SystemTime};

use bytes::Bytes;
use kameo::{message, prelude::*};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

use crate::{
    bus::client::{self, ClientHandle, Subscription, TopicBuilder}, utils::{
        self, actors::{ActorHandle, PublisherHandle, SpawnedActor, SpawnedActors, SubscriberHandle, spawn_pubsub}, logger::{LogEvent, LogSink, LogValue, LoggerHandle as SysLoggerHandle},
    },
};

const DOMAIN: &str = "logger";

const LOGGER_NAME: &str = "bus.logger";

/// Name of the PubSub actor that delivers remote logger records
const REMOTE_RECORDS_PUBSUB_NAME: &str = "bus.logger.remote-records";

#[derive(Debug)]
pub struct LoggerConfig {
    pub instance_name: Arc<String>,
    pub listen_remote: bool,
}

/// Client access to the logger actor
#[derive(Debug, Clone)]
pub struct LoggerHandle {
    on_remote_record: SubscriberHandle<LogRecord>,
}

impl LoggerHandle {
    /// Create a new access
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            on_remote_record: SubscriberHandle::from_name(REMOTE_RECORDS_PUBSUB_NAME)?,
        })
    }

    /// Get the PubSub for remote logger records
    pub fn on_remote_record(&self) -> &SubscriberHandle<LogRecord> {
        &self.on_remote_record
    }
}

pub async fn init_pubsubs(actors: &mut SpawnedActors) {
    actors.add(spawn_pubsub::<LogRecord>(REMOTE_RECORDS_PUBSUB_NAME).await);
}

pub async fn init_actor(actors: &mut SpawnedActors, config: LoggerConfig) {
    let (logger, _) = SpawnedActor::start::<Logger>(config).await;

    logger.register(LOGGER_NAME);

    actors.add(logger);
}

#[derive(Debug)]
struct Logger {
    client: ClientHandle,
    publisher: LogPublisher,
    remote: Option<Remote>,
    logger: Option<SysLoggerHandle>,
    online: bool,
    offline_queue: Vec<SysLogRecord>,
}

impl Logger {
    fn flush(&mut self) {
        for record in self.offline_queue.drain(..) {
            self.publisher.publish(record);
        }
    }
}

impl Actor for Logger {
    type Args = LoggerConfig;
    type Error = anyhow::Error;

    async fn on_start(config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let sys_logger = SysLogger(ActorHandle::from_ref(actor_ref.clone(), LOGGER_NAME));
        let logger = utils::logger::add_logger(Box::new(sys_logger));

        let remote = if config.listen_remote {
            Some(Remote::new(actor_ref.clone())?)
        } else {
            None
        };

        let _self = Self {
            client: ClientHandle::new()?,
            publisher: LogPublisher::new(config.instance_name)?,
            remote,
            logger: Some(logger),
            online: false,
            offline_queue: Vec::new(),
        };

        _self.client.on_online().subscribe(actor_ref);

        Ok(_self)
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        // Drop the logger
        self.logger = None;

        // Drop the remote
        self.remote = None;

        self.offline_queue.clear();

        Ok(())
    }
}

impl message::Message<client::Online> for Logger {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: client::Online,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.online = msg.is_online();

        if self.online {
            self.flush();
        }
    }
}

impl message::Message<client::Message> for Logger {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: client::Message,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let Some(remote) = &self.remote else {
            tracing::error!("got message without remote");
            return;
        };

        remote.on_message(msg);
    }
}

impl message::Message<SysLogRecord> for Logger {
    type Reply = ();

    async fn handle(
        &mut self,
        record: SysLogRecord,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.online {
            self.publisher.publish(record);
        } else {
            // Note: we may miss some logs when we become offline (the mqtt send queue will be discarded)
            self.offline_queue.push(record);
        }
    }
}

#[derive(Debug)]
struct LogPublisher {
    client: ClientHandle,
    instance_name: Arc<String>,
    pid: u32,
}

impl LogPublisher {
    pub fn new(instance_name: Arc<String>) -> anyhow::Result<Self> {
        Ok(Self {
            client: ClientHandle::new()?,
            instance_name,
            pid: process::id(),
        })
    }

    pub fn publish(&self, record: SysLogRecord) {
        let mut fields = record.event.fields;

        let mut message = if let Some(LogValue::Str(message)) = fields
            .extract_if(.., |(key, _value)| key == "message")
            .next()
            .map(|(_key, value)| value)
        {
            message
        } else {
            "??".to_owned()
        };

        let error = if let Some(LogValue::Str(error)) = fields
            .extract_if(.., |(key, _value)| key == "error")
            .next()
            .map(|(_key, value)| value)
        {
            Some(LogError {
                name: "Error".to_owned(),
                message: error,
                stack: "".to_owned(),
            })
        } else {
            None
        };

        let parts: Vec<_> = fields
            .into_iter()
            .map(|(key, value)| match value {
                LogValue::Bool(value) => format!("{}:{}", key, value),
                LogValue::I64(value) => format!("{}:{}", key, value),
                LogValue::U64(value) => format!("{}:{}", key, value),
                LogValue::F64(value) => format!("{}:{}", key, value),
                LogValue::Str(value) => format!("{}:'{}'", key, value),
            })
            .collect();

        if !parts.is_empty() {
            message += " - ";
            message += &parts.join(", ");
        }

        // TODO: format KV
        let record = LogRecord {
            name: record.event.target,
            instance_name: (*self.instance_name).clone(),
            pid: self.pid,
            level: record.event.level.into(),
            msg: message,
            err: error,
            time: record.time,
            v: 0,
        };

        let topic = TopicBuilder::local(&self.instance_name, DOMAIN).build();
        let payload = match serde_json::to_vec(&record) {
            Ok(payload) => Bytes::from_owner(payload),
            Err(error) => {
                tracing::error!(?error, ?record, "cannot serialize log record");
                return;
            }
        };
        self.client.publish(topic, payload, false);
    }
}

#[derive(Debug)]
struct Remote {
    client: ClientHandle,
    publisher: PublisherHandle<LogRecord>,
}

impl Remote {
    pub fn new(actor_ref: ActorRef<Logger>) -> anyhow::Result<Self> {
        let _self = Self {
            client: ClientHandle::new()?,
            publisher: PublisherHandle::from_name(REMOTE_RECORDS_PUBSUB_NAME)?,
        };

        _self.client.subscribe(Self::build_subscription());
        _self.client.on_message().subscribe(actor_ref);

        Ok(_self)
    }

    fn build_subscription() -> Subscription {
        TopicBuilder::any_instance(DOMAIN).build()
    }

    pub fn on_message(&self, msg: client::Message) {
        let Some(topic) = msg.parse_topic() else {
            return;
        };

        if topic.domain != DOMAIN {
            return;
        }

        let instance = topic.instance;
        let record = match serde_json::from_slice::<LogRecord>(msg.payload()) {
            Ok(record) => record,
            Err(error) => {
                tracing::error!(?error, instance, "unable to read log record");
                return;
            }
        };

        if instance != record.instance_name {
            tracing::warn!(
                record_instance = record.instance_name,
                topic_instance = instance,
                "got log record and topic instance name mismatch"
            );
        }

        self.publisher.publish(record);
    }
}

impl Drop for Remote {
    fn drop(&mut self) {
        self.client.unsubscribe(Self::build_subscription());
    }
}

#[derive(Debug)]
struct SysLogger(ActorHandle<Logger>);

impl LogSink for SysLogger {
    fn emit(&self, event: &LogEvent) {
        // Log to max DEBUG.
        if event.level > tracing::Level::DEBUG {
            return;
        }

        self.0.send(SysLogRecord {
            event: event.clone(),
            time: SystemTime::now(),
        });
    }
}

#[derive(Debug, Clone)]
struct SysLogRecord {
    event: LogEvent,
    time: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogRecord {
    pub name: String,
    pub instance_name: String,
    pub pid: u32,
    pub level: LogLevel,
    pub msg: String,
    pub err: Option<LogError>,
    #[serde(with = "rfc3339")]
    pub time: SystemTime,
    pub v: u32, // 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum LogLevel {
    /// The service/app is going to stop or become unusable now. An operator should definitely look into this soon.
    Fatal = 60,

    /// Fatal for a particular request, but the service/app continues servicing other requests. An operator should look at this soon(ish).
    Error = 50,

    /// A note on something that should probably be looked at by an operator eventually.
    Warn = 40,

    /// Detail on regular operation.
    Info = 30,

    /// Anything else, i.e. too verbose to be included in "info" level.
    Debug = 20,

    /// Logging from external libraries used by your app or very detailed application logging.
    Trace = 10,
}

impl From<tracing::Level> for LogLevel {
    fn from(value: tracing::Level) -> Self {
        match value {
            tracing::Level::ERROR => LogLevel::Error,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::TRACE => LogLevel::Trace,
        }
    }
}

impl Serialize for LogLevel {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u32(*self as u32)
    }
}

impl<'de> Deserialize<'de> for LogLevel {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = u32::deserialize(d)?;
        match value {
            60 => Ok(LogLevel::Fatal),
            50 => Ok(LogLevel::Error),
            40 => Ok(LogLevel::Warn),
            30 => Ok(LogLevel::Info),
            20 => Ok(LogLevel::Debug),
            10 => Ok(LogLevel::Trace),
            other => Err(Error::custom(format!("invalid LogLevel: {}", other))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogError {
    pub message: String,
    pub name: String,
    pub stack: String,
}

mod rfc3339 {
    use chrono::{DateTime, Local, SecondsFormat};
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::SystemTime;

    pub fn serialize<S: Serializer>(time: &SystemTime, s: S) -> Result<S::Ok, S::Error> {
        let dt: DateTime<Local> = (*time).into();
        s.serialize_str(&dt.to_rfc3339_opts(SecondsFormat::Secs, false))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<SystemTime, D::Error> {
        let s = String::deserialize(d)?;
        let dt = DateTime::parse_from_rfc3339(&s).map_err(serde::de::Error::custom)?;
        Ok(dt.into())
    }
}
