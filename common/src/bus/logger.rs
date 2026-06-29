use std::{alloc::System, collections::HashMap, process, sync::Arc, time::SystemTime};

use bytes::Bytes;
use kameo::{message, prelude::*};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

use crate::{
    bus::{
        client::{self, ClientHandle, Subscription, TopicBuilder},
        encoding,
    },
    utils::actors::{
        ActorHandle, PublisherHandle, SpawnedActor, SpawnedActors, SubscriberHandle, spawn_pubsub,
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
    log_publisher: PublisherHandle<LogRecord>,
    instance_name: Arc<String>,
    pid: u32,
    listen_remote: bool,
}

impl Logger {
    fn build_subscription() -> Subscription {
        TopicBuilder::any_instance(DOMAIN).build()
    }
}

impl Actor for Logger {
    type Args = LoggerConfig;
    type Error = anyhow::Error;

    async fn on_start(config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let _self = Self {
            client: ClientHandle::new()?,
            log_publisher: PublisherHandle::from_name(REMOTE_RECORDS_PUBSUB_NAME)?,
            instance_name: config.instance_name,
            pid: process::id(),
            listen_remote: config.listen_remote,
        };

        if _self.listen_remote {
            _self.client.subscribe(Self::build_subscription());
            _self.client.on_message().subscribe(actor_ref);
        }

        Ok(_self)
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        if self.listen_remote {
            self.client.unsubscribe(Self::build_subscription());
        }

        Ok(())
    }
}

impl message::Message<client::Message> for Logger {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: client::Message,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
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
                tracing::error!(?error, instance; "unable to read log record");
                return;
            }
        };

        if instance != record.instance_name {
            tracing::warn!(record_instance = record.instance_name, topic_instance = instance; "got log record and topic instance name mismatch");
        }

        self.log_publisher.publish(record);
    }
}

impl message::Message<SysLogRecord> for Logger {
    type Reply = ();

    async fn handle(
        &mut self,
        record: SysLogRecord,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.log_publisher.publish(LogRecord {
            name: record.name.unwrap_or_else("unknown"),
            instance_name: self.instance_name.clone(),
            pid: self.pid,
            level: record.level,
            msg: record.msg,
            err: (),
            time: record.time,
            v: 0,
        });
    }
}

#[derive(Debug)]
struct SysLogger(ActorHandle<Logger>);

impl tracing::Log for SysLogger {
    fn enabled(&self, metadata: &tracing::Metadata) -> bool {
        // Log to max DEBUG
        metadata.level() <= tracing::Level::Debug
    }

    fn log(&self, record: &tracing::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        self.0.send(SysLogRecord {
            name: record.module_path().map(|s| s.to_owned()),
            level: record.level(),
            msg: format!("{}", record.args()),
            time: SystemTime::now(),
        });
    }

    fn flush(&self) {
        // Nothing to do
    }
}

#[derive(Debug, Clone)]
struct SysLogRecord {
    name: Option<String>,
    level: tracing::Level,
    msg: String,
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
