use common::utils::actors::{ActorHandle, HandleLookupError, SpawnedActor, SpawnedActors};
use kameo::{error::Infallible, message, prelude::*};
use studio_web_api::protocol;

use crate::web::{DispatcherBuilder, NotifierManager, ServiceCall, SessionEvent};

const GIT_NAME: &str = "git";

pub async fn init(actors: &mut SpawnedActors, dispatcher: &mut DispatcherBuilder) {
    let (git, _) = SpawnedActor::start::<Git>(()).await;

    git.register(GIT_NAME);

    actors.add(git);

    let actor: ActorRef<_> = ActorHandle::<Git>::from_name(GIT_NAME)
        .expect("cannot get git actor handle")
        .into();

    dispatcher.register_session_handler(actor.clone());
    dispatcher.register_call::<StartNotifyReq, _>("git/start-notify", actor.clone());
    dispatcher.register_call::<StopNotifyReq, _>("git/stop-notify", actor);
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
struct Git {
    notifiers: NotifierManager<studio_web_api::git::GitStatusNotification>,
    current_status: studio_web_api::git::GitStatus,
}

impl Actor for Git {
    type Args = ();
    type Error = HandleLookupError;

    async fn on_start(_args: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(Self {
            notifiers: NotifierManager::new("git/status"),
            current_status: studio_web_api::git::GitStatus {
                app_url: None,
                branch: "<unknown>".to_owned(),
                changed_features: Vec::new(),
                ahead: None,
                behind: None,
            },
        })
    }
}

impl message::Message<SessionEvent> for Git {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.notifiers.session_event(&msg);
    }
}

impl message::Message<ServiceCall<StartNotifyReq>> for Git {
    type Reply = ();

    async fn handle(
        &mut self,
        call: ServiceCall<StartNotifyReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let notifier = self.notifiers.create_notifier(call.session().clone());

        call.reply_ok(StartNotifyRes(protocol::NotifierId {
            notifier_id: notifier.notifier_id().into(),
        }));

        notifier.notify(&studio_web_api::git::GitStatusNotification {
            status: self.current_status.clone(),
        });
    }
}

impl message::Message<ServiceCall<StopNotifyReq>> for Git {
    type Reply = ();

    async fn handle(
        &mut self,
        call: ServiceCall<StopNotifyReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let notifier_id = &call.request().0;
        self.notifiers
            .remove_notifier(notifier_id.notifier_id.as_str());

        call.reply_ok(StopNotifyRes);
    }
}
