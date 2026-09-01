use common::{
    bus::client,
    utils::actors::{ActorHandle, HandleLookupError, SpawnedActor, SpawnedActors},
};
use kameo::{error::Infallible, message, prelude::*};
use studio_web_api::protocol;

use crate::web::{DispatcherBuilder, NotifierManager, ServiceRequest, SessionEvent};

const ONLINE_STATUS_NAME: &str = "online-status";

pub async fn init_actor(actors: &mut SpawnedActors, dispatcher: &mut DispatcherBuilder) {
    let (online_status, _) = SpawnedActor::start::<OnlineStatus>(()).await;

    online_status.register(ONLINE_STATUS_NAME);

    actors.add(online_status);

    let actor: ActorRef<_> = ActorHandle::<OnlineStatus>::from_name(ONLINE_STATUS_NAME)
        .expect("Cannot get online status access")
        .into();
    dispatcher.register_session_handler(actor.clone());
    dispatcher.register_call::<StartNotifyReq, _>("online/start-notify-status", actor.clone());
    dispatcher.register_call::<StopNotifyReq, _>("online/stop-notify-status", actor.clone());
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

#[derive(Debug)]
struct OnlineStatus {
    notifiers: NotifierManager<studio_web_api::online::Status>,
    current_status: studio_web_api::online::Status,
}

impl Actor for OnlineStatus {
    type Args = ();
    type Error = HandleLookupError;

    async fn on_start(_args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        client::ClientHandle::new()?
            .on_online()
            .subscribe(actor_ref);

        Ok(Self {
            notifiers: NotifierManager::new("online/status"),
            current_status: studio_web_api::online::Status {
                // Note: must be constructed before client connects
                transport_connected: false,
            },
        })
    }
}

impl message::Message<SessionEvent> for OnlineStatus {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.notifiers.session_event(&msg);
    }
}

// start notify
impl message::Message<ServiceRequest<StartNotifyReq>> for OnlineStatus {
    type Reply = DelegatedReply<Result<StartNotifyRes, Infallible>>;

    async fn handle(
        &mut self,
        msg: ServiceRequest<StartNotifyReq>,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let (session, _) = msg.into();
        let notifier = self.notifiers.create_notifier(session);

        let res = ctx.reply(Ok(StartNotifyRes(protocol::NotifierId {
            notifier_id: notifier.notifier_id().into(),
        })));

        // Send the current status right after the notifier is created to ensure the client receives the latest status immediately.
        notifier.notify(&self.current_status);

        res
    }
}

// stop notify
impl message::Message<ServiceRequest<StopNotifyReq>> for OnlineStatus {
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

impl message::Message<client::Online> for OnlineStatus {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: client::Online,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.current_status.transport_connected = msg.is_online();
        self.notifiers.notify_all(&self.current_status);
    }
}
