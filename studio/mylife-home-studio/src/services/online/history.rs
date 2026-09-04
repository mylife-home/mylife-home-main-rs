use std::{collections::VecDeque, time::SystemTime};

use common::{
    bus::client,
    components::{
        metadata::MemberType,
        registry::{RegistryHandle, RegistryUpdated},
        types::Value,
    },
    utils::actors::{ActorHandle, HandleLookupError, SpawnedActor, SpawnedActors},
};
use kameo::{message, prelude::*};
use studio_web_api::{online, protocol};
use thiserror::Error;

use crate::web::{DispatcherBuilder, NotifierManager, ServiceRequest, SessionEvent};

const ONLINE_HISTORY_NAME: &str = "online-history";
const HISTORY_SIZE: usize = 1000;

pub async fn init(actors: &mut SpawnedActors, dispatcher: &mut DispatcherBuilder) {
    let (online_history, _) = SpawnedActor::start::<OnlineHistory>(()).await;

    online_history.register(ONLINE_HISTORY_NAME);
    actors.add(online_history);

    let actor: ActorRef<_> = ActorHandle::<OnlineHistory>::from_name(ONLINE_HISTORY_NAME)
        .expect("cannot get online history actor handle")
        .into();

    dispatcher.register_session_handler(actor.clone());
    dispatcher.register_call::<StartNotifyReq, _>("online/start-notify-history", actor.clone());
    dispatcher.register_call::<StopNotifyReq, _>("online/stop-notify-history", actor);
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
enum OnlineHistoryError {
    #[error("failed to lookup actor handle: {0}")]
    HandleLookup(#[from] HandleLookupError),
}

#[derive(Debug)]
struct OnlineHistory {
    records: VecDeque<online::HistoryRecord>,
    notifiers: NotifierManager<online::HistoryRecord>,
}

impl Actor for OnlineHistory {
    type Args = ();
    type Error = OnlineHistoryError;

    async fn on_start(_args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        client::ClientHandle::new()?
            .on_instance_online()
            .subscribe(actor_ref.clone());
        RegistryHandle::new()?.on_update().subscribe(actor_ref);

        Ok(Self {
            records: VecDeque::with_capacity(HISTORY_SIZE),
            notifiers: NotifierManager::new("online/history"),
        })
    }
}

impl message::Message<SessionEvent> for OnlineHistory {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.notifiers.session_event(&msg);
    }
}

impl message::Message<client::InstanceOnline> for OnlineHistory {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: client::InstanceOnline,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = online::InstanceHistoryRecord {
            timestamp: Self::timestamp(),
            instance_name: msg.instance().to_owned(),
        };

        self.add_record(if msg.is_online() {
            online::HistoryRecord::InstanceSet(record)
        } else {
            online::HistoryRecord::InstanceClear(record)
        });
    }
}

impl message::Message<RegistryUpdated> for OnlineHistory {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: RegistryUpdated,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let timestamp = Self::timestamp();
        let record = match msg {
            RegistryUpdated::ComponentAdded(data) => {
                online::HistoryRecord::ComponentSet(online::ComponentSetHistoryRecord {
                    timestamp,
                    instance_name: Self::instance_name(data.instance()),
                    component_id: data.component_id().to_owned(),
                    states: Some(
                        data.plugin()
                            .members()
                            .iter()
                            .filter(|(_, member)| member.member_type() == MemberType::State)
                            .map(|(name, _)| (name.clone(), serde_json::Value::Null))
                            .collect(),
                    ),
                })
            }
            RegistryUpdated::ComponentRemoved(data) => {
                online::HistoryRecord::ComponentClear(online::ComponentClearHistoryRecord {
                    timestamp,
                    instance_name: Self::instance_name(data.instance()),
                    component_id: data.component_id().to_owned(),
                })
            }
            RegistryUpdated::ComponentStateChanged(data) => {
                online::HistoryRecord::StateSet(online::StateHistoryRecord {
                    timestamp,
                    instance_name: Self::instance_name(data.instance()),
                    component_id: data.component_id().to_owned(),
                    state_name: data.state().to_owned(),
                    state_value: Self::convert_value(data.value()),
                })
            }
            RegistryUpdated::PluginAdded(_) | RegistryUpdated::PluginRemoved(_) => return,
        };

        self.add_record(record);
    }
}

impl message::Message<ServiceRequest<StartNotifyReq>> for OnlineHistory {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<StartNotifyReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
        let notifier = self.notifiers.create_notifier(call.session().clone());

        call.reply_ok(StartNotifyRes(protocol::NotifierId {
            notifier_id: notifier.notifier_id().into(),
        }));

        for record in &self.records {
            notifier.notify(record);
        }
    }
}

impl message::Message<ServiceRequest<StopNotifyReq>> for OnlineHistory {
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

impl OnlineHistory {
    fn add_record(&mut self, record: online::HistoryRecord) {
        if self.records.len() == HISTORY_SIZE {
            self.records.pop_front();
        }

        self.records.push_back(record.clone());
        self.notifiers.notify_all(&record);
    }

    fn timestamp() -> i64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system clock is before Unix epoch")
            .as_millis()
            .try_into()
            .expect("timestamp does not fit in i64")
    }

    fn instance_name(instance: Option<&str>) -> String {
        instance.unwrap_or("local").to_owned()
    }

    fn convert_value(value: &Value) -> serde_json::Value {
        match value {
            Value::Range(value) => (*value).into(),
            Value::Text(value) | Value::Enum(value) => value.clone().into(),
            Value::Float(value) => serde_json::Number::from_f64(*value)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::Bool(value) => (*value).into(),
            Value::Complex => serde_json::Value::Null,
        }
    }
}
