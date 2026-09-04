use std::collections::VecDeque;

use chrono::{DateTime, Local, SecondsFormat};
use common::{
    bus::logger::{LogLevel, LogRecord, LoggerHandle},
    utils::actors::{ActorHandle, HandleLookupError, SpawnedActor, SpawnedActors},
};
use kameo::{message, prelude::*};
use studio_web_api::{logging, protocol};
use thiserror::Error;

use crate::web::{DispatcherBuilder, NotifierManager, ServiceRequest, SessionEvent};

const LOGGING_NAME: &str = "logging";
const LOG_BUFFER_SIZE: usize = 1000;

pub async fn init(actors: &mut SpawnedActors, dispatcher: &mut DispatcherBuilder) {
    let (logging, _) = SpawnedActor::start::<Logging>(()).await;

    logging.register(LOGGING_NAME);
    actors.add(logging);

    let actor: ActorRef<_> = ActorHandle::<Logging>::from_name(LOGGING_NAME)
        .expect("cannot get logging actor handle")
        .into();

    dispatcher.register_session_handler(actor.clone());
    dispatcher.register_call::<StartNotifyReq, _>("logging/start-notify-logs", actor.clone());
    dispatcher.register_call::<StopNotifyReq, _>("logging/stop-notify-logs", actor);
}

#[derive(Debug, serde::Deserialize)]
struct StartNotifyReq;

#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
struct StartNotifyRes(protocol::NotifierId);

#[derive(Debug, serde::Deserialize)]
#[serde(transparent)]
struct StopNotifyReq(protocol::NotifierId);

#[derive(Debug, serde::Serialize)]
struct StopNotifyRes;

#[derive(Debug, Error)]
enum LoggingError {
    #[error("failed to lookup actor handle: {0}")]
    HandleLookup(#[from] HandleLookupError),
}

#[derive(Debug)]
struct Logging {
    records: VecDeque<logging::LogRecord>,
    notifiers: NotifierManager<logging::LogRecord>,
}

impl Actor for Logging {
    type Args = ();
    type Error = LoggingError;

    async fn on_start(_args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        LoggerHandle::new()?.on_remote_record().subscribe(actor_ref);

        Ok(Self {
            records: VecDeque::with_capacity(LOG_BUFFER_SIZE),
            notifiers: NotifierManager::new("logging/logs"),
        })
    }
}

impl message::Message<SessionEvent> for Logging {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.notifiers.session_event(&msg);
    }
}

impl message::Message<LogRecord> for Logging {
    type Reply = ();

    async fn handle(
        &mut self,
        record: LogRecord,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.add_record(Self::convert_record(record));
    }
}

impl message::Message<ServiceRequest<StartNotifyReq>> for Logging {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<StartNotifyReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
        let notifier = self
            .notifiers
            .create_notifier(call.session().clone())
            .clone();

        call.reply_ok(StartNotifyRes(protocol::NotifierId {
            notifier_id: notifier.notifier_id().into(),
        }));

        for record in &self.records {
            notifier.notify(record);
        }
    }
}

impl message::Message<ServiceRequest<StopNotifyReq>> for Logging {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<StopNotifyReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
        let notifier_id = &call.request().0;
        self.notifiers.remove_notifier(&notifier_id.notifier_id);

        call.reply_ok(StopNotifyRes);
    }
}

impl Logging {
    fn add_record(&mut self, record: logging::LogRecord) {
        if self.records.len() == LOG_BUFFER_SIZE {
            self.records.pop_front();
        }

        self.records.push_back(record.clone());
        self.notifiers.notify_all(&record);
    }

    fn convert_record(record: LogRecord) -> logging::LogRecord {
        logging::LogRecord {
            name: record.name,
            instance_name: record.instance_name,
            hostname: record.hostname,
            pid: record.pid as i32,
            level: match record.level {
                LogLevel::Fatal => 60,
                LogLevel::Error => 50,
                LogLevel::Warn => 40,
                LogLevel::Info => 30,
                LogLevel::Debug => 20,
                LogLevel::Trace => 10,
            },
            msg: record.msg,
            time: DateTime::<Local>::from(record.time).to_rfc3339_opts(SecondsFormat::Secs, false),
            v: record.v as i32,
            err: record.err.map(|error| logging::LogRecordError {
                message: error.message,
                name: error.name,
                stack: error.stack,
            }),
        }
    }
}
