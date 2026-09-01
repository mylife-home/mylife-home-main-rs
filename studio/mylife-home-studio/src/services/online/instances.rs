use std::collections::HashMap;

use common::{
    bus::metadata::{MetadataHandle, RemoteUpdate},
    instance_info::types::InstanceInfo,
    utils::actors::{ActorHandle, HandleLookupError, SpawnedActor, SpawnedActors},
};
use kameo::{error::Infallible, message, prelude::*};
use studio_web_api::{online, protocol};
use thiserror::Error;

use crate::web::{DispatcherBuilder, NotifierManager, ServiceRequest, SessionEvent};

const ONLINE_INSTANCES_NAME: &str = "online-instances";
const INSTANCE_INFO_PATH: &str = "instance-info";

pub async fn init(actors: &mut SpawnedActors, dispatcher: &mut DispatcherBuilder) {
    let (online_instances, _) =
        SpawnedActor::start::<OnlineInstances>(()).await;

    online_instances.register(ONLINE_INSTANCES_NAME);
    actors.add(online_instances);

    let actor: ActorRef<_> = ActorHandle::<OnlineInstances>::from_name(ONLINE_INSTANCES_NAME)
        .expect("cannot get online instances actor handle")
        .into();

    dispatcher.register_session_handler(actor.clone());
    dispatcher.register_call::<StartNotifyReq, _>("online/start-notify-instance-info", actor.clone());
    dispatcher.register_call::<StopNotifyReq, _>("online/stop-notify-instance-info", actor);
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
enum OnlineInstancesError {
    #[error("failed to lookup actor handle: {0}")]
    HandleLookup(#[from] HandleLookupError),
}

#[derive(Debug)]
struct OnlineInstances {
    instances: HashMap<String, online::InstanceInfo>,
    notifiers: NotifierManager<online::UpdateInstanceInfoData>,
}

impl Actor for OnlineInstances {
    type Args = ();
    type Error = OnlineInstancesError;

    async fn on_start(
        _args: Self::Args,
        actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        let metadata = MetadataHandle::new()?;

        metadata.on_remote_update().subscribe(actor_ref);

        Ok(Self {
            instances: HashMap::new(),
            notifiers: NotifierManager::new("online/instance-info"),
        })
    }
}

impl message::Message<SessionEvent> for OnlineInstances {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.notifiers.session_event(&msg);
    }
}

impl message::Message<RemoteUpdate> for OnlineInstances {
    type Reply = ();

    async fn handle(
        &mut self,
        update: RemoteUpdate,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if update.path() != INSTANCE_INFO_PATH {
            return;
        }

        let instance_name = update.instance().to_owned();
        if update.has_value() {
            match update.read_value::<InstanceInfo>() {
                Ok(info) => self.set_instance(instance_name, convert_instance_info(info)),
                Err(error) => tracing::error!(%error, instance = update.instance(), "could not read remote instance info"),
            }
        } else {
            self.clear_instance(&instance_name);
        }
    }
}

impl message::Message<ServiceRequest<StartNotifyReq>> for OnlineInstances {
    type Reply = DelegatedReply<Result<StartNotifyRes, Infallible>>;

    async fn handle(
        &mut self,
        msg: ServiceRequest<StartNotifyReq>,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let (session, _) = msg.into();
        let notifier = self.notifiers.create_notifier(session);

        let response = ctx.reply(Ok(StartNotifyRes(protocol::NotifierId {
            notifier_id: notifier.notifier_id().into(),
        })));

        for (instance_name, data) in &self.instances {
            notifier.notify(&online::UpdateInstanceInfoData::Set(online::SetInstanceInfoData {
                instance_name: instance_name.clone(),
                data: data.clone(),
            }));
        }

        response
    }
}

impl message::Message<ServiceRequest<StopNotifyReq>> for OnlineInstances {
    type Reply = Result<StopNotifyRes, Infallible>;

    async fn handle(
        &mut self,
        msg: ServiceRequest<StopNotifyReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let (_, notifier_id) = msg.into();
        self.notifiers
            .remove_notifier(notifier_id.0.notifier_id.as_str());

        Ok(StopNotifyRes)
    }
}

impl OnlineInstances {
    fn set_instance(&mut self, instance_name: String, data: online::InstanceInfo) {
        self.instances.insert(instance_name.clone(), data.clone());
        
        self.notifiers
            .notify_all(&online::UpdateInstanceInfoData::Set(online::SetInstanceInfoData {
                instance_name,
                data,
            }));
    }

    fn clear_instance(&mut self, instance_name: &str) {
        if self.instances.remove(instance_name).is_none() {
            return;
        }

        self.notifiers.notify_all(&online::UpdateInstanceInfoData::Clear(
            online::ClearInstanceInfoData {
                instance_name: instance_name.to_owned(),
            },
        ));
    }
}

fn convert_instance_info(info: InstanceInfo) -> online::InstanceInfo {
    online::InstanceInfo {
        r#type: info.r#type,
        hardware: info.hardware,
        versions: info.versions,
        system_uptime: info.system_uptime.as_secs() as i64,
        instance_uptime: info.instance_uptime.as_secs() as i64,
        hostname: info.hostname,
        capabilities: info.capabilities,
        wifi: info.wifi.map(|wifi| online::InstanceInfoWifi {
            rssi: wifi.rssi as i32,
        }),
    }
}
